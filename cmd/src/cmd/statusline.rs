//! The Claude Code status line, wired up in `claude/settings.json` as
//! `cmd statusline`
//!
//! Claude Code runs this on every status refresh with the session as JSON on
//! stdin and shows what it prints. An error in the status bar is worse than a
//! shorter line, so a missing or malformed field drops its segment, unreadable
//! input falls back to the working directory, and nothing here exits non-zero.
//! The branch comes from the repository's `HEAD` file, so the common case
//! forks no git process

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const PURPLE: &str = "\x1b[38;5;141m";
const BRIGHT_CYAN: &str = "\x1b[96m";

/// Directories deeper than this many `/`-separated parts show only their last
/// five, like Starship
const MAX_DIR_PARTS: usize = 6;
const KEPT_DIR_PARTS: usize = 5;

/// What rendering needs from the machine, so tests can run it against fixtures
pub trait Host {
    /// The home directory, shown as `~`
    fn home(&self) -> Option<PathBuf>;

    /// The directory to show when the input names none
    fn current_dir(&self) -> Option<PathBuf>;

    /// The branch checked out at `dir`, or the short commit when detached
    fn git_branch(&self, dir: &Path) -> Option<String>;
}

/// The real machine
pub struct System;

impl Host for System {
    fn home(&self) -> Option<PathBuf> {
        crate::fsutil::home_dir().ok()
    }

    fn current_dir(&self) -> Option<PathBuf> {
        std::env::current_dir().ok()
    }

    fn git_branch(&self, dir: &Path) -> Option<String> {
        git_branch(dir)
    }
}

/// Prints the status line for the session on stdin; never fails, so a bug
/// shows a blank line instead of an error
pub fn run() {
    let line = std::panic::catch_unwind(|| {
        // a read error leaves the input empty, which renders the fallback line
        let mut stdin = String::new();
        let _ = std::io::stdin().read_to_string(&mut stdin);
        render(&stdin, &System)
    })
    .unwrap_or_default();

    // claude code cancels a run that a newer refresh replaces, closing the pipe
    let _ = writeln!(std::io::stdout().lock(), "{line}");
}

/// The status line for the session JSON in `stdin`, without a trailing newline
pub fn render(stdin: &str, host: &impl Host) -> String {
    // each field is looked up on its own, so one of an unexpected type drops
    // only its own segment
    let input = serde_json::from_str::<Value>(stdin).unwrap_or(Value::Null);
    let text = |pointer: &str| {
        input
            .pointer(pointer)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
    };
    let percent = |pointer: &str| {
        input
            .pointer(pointer)
            .and_then(Value::as_f64)
            .and_then(rounded)
    };

    let mut line = String::new();

    // docs prefer `workspace.current_dir`; `cwd` holds the same value
    let dir = text("/workspace/current_dir")
        .or_else(|| text("/cwd"))
        .map(PathBuf::from)
        .or_else(|| host.current_dir());

    if let Some(dir) = dir {
        let shown = short_dir(&dir, host.home().as_deref());
        line += &format!("{BOLD}{CYAN}{shown}{RESET}");

        if let Some(branch) = host.git_branch(&dir) {
            line += &format!(" {YELLOW}[{branch}]{RESET}");
        }
    }

    let separator = format!(" {BOLD}•{RESET} ");

    if let Some(model) = text("/model/display_name").or_else(|| text("/model/id")) {
        line += &format!("{separator}{BLUE}{model}{RESET}");
    }

    if let Some(used) = percent("/context_window/used_percentage") {
        line += &format!("{separator}{}ctx: {used}%{RESET}", usage_color(used));
    }

    let limits = ["/rate_limits/five_hour", "/rate_limits/seven_day"]
        .into_iter()
        .filter_map(|window| percent(&format!("{window}/used_percentage")))
        .map(|used| format!("{}{used}%", usage_color(used)))
        .collect::<Vec<_>>();

    if !limits.is_empty() {
        let limits = limits.join(&format!("{RESET}|"));
        line += &format!("{separator}usage: {limits}{RESET}");
    }

    if let Some(style) = text("/output_style/name").filter(|style| *style != "default") {
        line += &format!("{separator}{MAGENTA}{style}{RESET}");
    }

    if let Some(mode) = text("/vim/mode") {
        line += &format!("{separator}{PURPLE}{mode}{RESET}");
    }

    if let Some(agent) = text("/agent/name") {
        line += &format!("{separator}{BRIGHT_CYAN}{agent}{RESET}");
    }

    line
}

/// A percentage rounded half to even like `printf "%.0f"`, or `None` when it
/// rounds to a negative or non-finite value
fn rounded(value: f64) -> Option<u64> {
    let digits = format!("{value:.0}");
    digits.parse().ok()
}

fn usage_color(used: u64) -> &'static str {
    match used {
        0..50 => GREEN,
        50..80 => YELLOW,
        _ => RED,
    }
}

/// `dir` with the home directory as `~`, cut to its last parts when deep
fn short_dir(dir: &Path, home: Option<&Path>) -> String {
    let dir = dir.to_string_lossy();
    let home = home.map(|home| home.to_string_lossy().trim_end_matches('/').to_owned());

    // only a whole home directory becomes `~`, so `/Users/me2` stays as is
    let dir = match home.filter(|home| !home.is_empty()) {
        Some(home) => match dir.strip_prefix(home.as_str()) {
            Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("~{rest}"),
            _ => dir.into_owned(),
        },
        None => dir.into_owned(),
    };

    let parts = dir.split('/').collect::<Vec<_>>();

    if parts.len() > MAX_DIR_PARTS {
        format!("…/{}", parts[parts.len() - KEPT_DIR_PARTS..].join("/"))
    } else {
        dir
    }
}

/// The branch checked out at `dir`, read from `HEAD` like `git symbolic-ref
/// --short HEAD`; a detached or reftable `HEAD` asks git instead
fn git_branch(dir: &Path) -> Option<String> {
    let head = fs::read_to_string(find_git_dir(dir)?.join("HEAD")).ok()?;

    match head.trim().strip_prefix("ref: refs/heads/") {
        // a reftable repository keeps this placeholder in `HEAD`
        Some(".invalid") => git(dir, &["symbolic-ref", "--short", "-q", "HEAD"])
            .or_else(|| git(dir, &["rev-parse", "--short", "HEAD"])),
        Some(branch) => Some(branch.to_owned()),
        // git picks the abbreviation length, which grows with the repository
        None => git(dir, &["rev-parse", "--short", "HEAD"]),
    }
}

/// The git directory for `dir` or its nearest ancestor in a repository
fn find_git_dir(dir: &Path) -> Option<PathBuf> {
    for ancestor in dir.ancestors() {
        let dot_git = ancestor.join(".git");

        let Ok(metadata) = fs::metadata(&dot_git) else {
            continue;
        };

        if metadata.is_dir() {
            return Some(dot_git);
        }

        // a linked worktree or submodule has a `.git` file naming its git dir
        let contents = fs::read_to_string(&dot_git).ok()?;
        let git_dir = contents.trim().strip_prefix("gitdir:")?.trim();
        return Some(ancestor.join(git_dir));
    }

    None
}

/// The trimmed stdout of `git -C dir args`, or `None` when it fails or is empty
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()?;

    let stdout = String::from_utf8(output.stdout).ok()?;
    let stdout = stdout.trim();
    (output.status.success() && !stdout.is_empty()).then(|| stdout.to_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{find_git_dir, git_branch, render, Host};

    struct Fixture;

    impl Host for Fixture {
        fn home(&self) -> Option<PathBuf> {
            Some(PathBuf::from("/Users/me"))
        }

        fn current_dir(&self) -> Option<PathBuf> {
            Some(PathBuf::from("/Users/me/launch"))
        }

        fn git_branch(&self, dir: &Path) -> Option<String> {
            dir.starts_with("/Users/me/code/dotfiles")
                .then(|| "master".to_owned())
        }
    }

    /// Expected lines match the zsh status line script this replaced byte for
    /// byte except where the port changed on purpose: the model name, the `~`
    /// boundary, and the fallback directory
    #[test]
    fn renders_session_input() {
        let cases = [
            (
                "every segment",
                r#"{
                    "cwd": "/Users/me/code/dotfiles",
                    "workspace": {"current_dir": "/Users/me/code/dotfiles/cmd"},
                    "model": {"id": "claude-opus-5-5", "display_name": "Opus 5.5"},
                    "context_window": {"used_percentage": 22.5},
                    "rate_limits": {
                        "five_hour": {"used_percentage": 13.4, "resets_at": 1},
                        "seven_day": {"used_percentage": 85, "resets_at": 2}
                    },
                    "output_style": {"name": "Explanatory"},
                    "vim": {"mode": "NORMAL"},
                    "agent": {"name": "rev"}
                }"#,
                "\x1b[1m\x1b[36m~/code/dotfiles/cmd\x1b[0m \x1b[33m[master]\x1b[0m \x1b[1m•\x1b[0m \x1b[34mOpus 5.5\x1b[0m \x1b[1m•\x1b[0m \x1b[32mctx: 22%\x1b[0m \x1b[1m•\x1b[0m usage: \x1b[32m13%\x1b[0m|\x1b[31m85%\x1b[0m \x1b[1m•\x1b[0m \x1b[35mExplanatory\x1b[0m \x1b[1m•\x1b[0m \x1b[38;5;141mNORMAL\x1b[0m \x1b[1m•\x1b[0m \x1b[96mrev\x1b[0m",
            ),
            (
                "session start, before the first response",
                r#"{
                    "cwd": "/Users/me/code/dotfiles",
                    "model": {"id": "claude-sonnet-5-5", "display_name": "Sonnet 5.5"},
                    "context_window": {"used_percentage": null},
                    "output_style": {"name": "default"}
                }"#,
                "\x1b[1m\x1b[36m~/code/dotfiles\x1b[0m \x1b[33m[master]\x1b[0m \x1b[1m•\x1b[0m \x1b[34mSonnet 5.5\x1b[0m",
            ),
            (
                "deep directory outside home, one rate limit window",
                r#"{
                    "cwd": "/srv/a/b/c/d/e/f",
                    "model": {"id": "claude-haiku-5-5"},
                    "context_window": {"used_percentage": 65},
                    "rate_limits": {"seven_day": {"used_percentage": 79.5}},
                    "vim": {"mode": "INSERT"}
                }"#,
                "\x1b[1m\x1b[36m…/b/c/d/e/f\x1b[0m \x1b[1m•\x1b[0m \x1b[34mclaude-haiku-5-5\x1b[0m \x1b[1m•\x1b[0m \x1b[33mctx: 65%\x1b[0m \x1b[1m•\x1b[0m usage: \x1b[31m80%\x1b[0m \x1b[1m•\x1b[0m \x1b[38;5;141mINSERT\x1b[0m",
            ),
            (
                "home prefix without a path boundary",
                r#"{"cwd": "/Users/me2/x"}"#,
                "\x1b[1m\x1b[36m/Users/me2/x\x1b[0m",
            ),
            (
                "fields of the wrong type",
                r#"{
                    "cwd": 5,
                    "model": {"display_name": 7, "id": "claude-opus-5-5"},
                    "context_window": {"used_percentage": "50"},
                    "rate_limits": {"five_hour": {"used_percentage": -3}, "seven_day": 9},
                    "vim": {"mode": ""}
                }"#,
                "\x1b[1m\x1b[36m~/launch\x1b[0m \x1b[1m•\x1b[0m \x1b[34mclaude-opus-5-5\x1b[0m",
            ),
            ("malformed JSON", "{\"cwd\": ", "\x1b[1m\x1b[36m~/launch\x1b[0m"),
            ("empty input", "", "\x1b[1m\x1b[36m~/launch\x1b[0m"),
        ];

        for (name, stdin, want) in cases {
            assert_eq!(render(stdin, &Fixture), want, "{name}");
        }
    }

    /// The branch is read from the `HEAD` file of the repository containing
    /// the directory, including from a subdirectory and a linked worktree
    #[test]
    fn reads_branch_from_head() {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        let worktree_git_dir = repo.join(".git/worktrees/wt");
        let worktree = root.path().join("wt");
        fs::create_dir_all(repo.join("src/deep")).unwrap();
        fs::create_dir_all(&worktree_git_dir).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        fs::write(repo.join(".git/HEAD"), "ref: refs/heads/feature/x\n").unwrap();
        fs::write(worktree_git_dir.join("HEAD"), "ref: refs/heads/wt-branch\n").unwrap();
        fs::write(worktree.join(".git"), "gitdir: ../repo/.git/worktrees/wt\n").unwrap();

        assert_eq!(git_branch(&repo).as_deref(), Some("feature/x"));
        assert_eq!(
            git_branch(&repo.join("src/deep")).as_deref(),
            Some("feature/x")
        );
        assert_eq!(git_branch(&worktree).as_deref(), Some("wt-branch"));
        assert_eq!(find_git_dir(&root.path().join("missing")), None);
    }
}
