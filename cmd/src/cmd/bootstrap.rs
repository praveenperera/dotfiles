mod install;
mod link;
mod release;

use std::ffi::OsString;

use askama::Template;
use clap::{Parser, ValueEnum};
use eyre::Result;
use xshell::{cmd, Shell};

use crate::os::Os;

#[derive(Debug, Clone, ValueEnum)]
pub enum BootstrapMode {
    Minimal,
    Full,
}

#[derive(Debug, Clone, Parser)]
pub struct Bootstrap {
    /// Bootstrap mode: 'minimal' or 'full'
    pub mode: BootstrapMode,
}

#[derive(Template)]
#[template(path = "zshrc.j2")]
struct Zshrc {
    os: Os,
}

#[derive(Template)]
#[template(path = "osx_defaults.zsh.j2")]
struct OsxDefaults {}

const MAC_ONLY_TOOLS: &[&str] = &[
    "swiftformat",
    "1password-cli",
    "xcode-build-server",
    "gpg-suite",
    "pinentry-mac",
];

const BREW_CASKS: &[&str] = &[
    "google-cloud-sdk",
    "visual-studio-code",
    "bettertouchtool",
    "github",
    "signal",
    "sublime-text",
    "raycast",
    "font-jetbrains-mono-nerd-font",
    "font-recursive-mono-nerd-font",
    "brave-browser",
    "appcleaner",
    "swiftformat-for-xcode",
    "slack",
    "selfcontrol",
    "figma",
    "lens",
];

const TOOLS_FULL: &[&str] = &[
    "bat",
    "coreutils",
    "diff-so-fancy",
    "difftastic",
    "fd",
    "fzf",
    "git",
    "git-delta",
    "gnupg",
    "direnv",
    "htop",
    "jq",
    "neovim",
    "ripgrep",
    "sccache",
    "starship",
    "sk",
    "tmux",
    "fnm",
    "btop",
    "htop",
    "eza",
    "pkg-config",
    "zoxide",
    "kubectl",
    "gpg",
    "tree",
    "shellcheck",
    "elixir",
    "topgrade",
    "go",
    "atuin",
    "mcfly",
    "zsh",
    "just",
    "rust-analyzer",
    "serpl",
    "zig",
    "zls",
    "k9s",
    "gh",
    "deno",
    "yt-dlp",
    "watchexec",
    "uv",
    "hyperfine",
    "nodejs",
];

const TOOLS_MINIMAL: &[&str] = &[
    "bat", "fzf", "htop", "btop", "ripgrep", "zoxide", "zsh", "direnv", "jq",
];

const TOOLS_VIA_SHELL_SCRIPT: &[(&str, &str, &[&str])] = &[
    ("https://starship.rs/install.sh", "starship", &["--yes"]),
    ("https://setup.atuin.sh", "atuin", &[]),
];

const LINUX_TOOLS_MINIMAL: &[&str] = &["ca-certificates", "curl", "unzip", "xsel", "wget", "gpg"];

const LINUX_TOOLS_FULL: &[&str] = &[
    "ca-certificates",
    "curl",
    "unzip",
    "xsel",
    "libssl-dev",
    "pkg-config",
    "gcc",
    "python3-dev",
    "python3-pip",
    "python3-setuptools",
    "wget",
    "gpg",
];

const CARGO_PKGS: &[&str] = &["bacon", "cargo-update", "cargo-nextest", "cargo-expand"];
const DOTFILES: &[&str] = &[
    "zshrc",
    "zshenv",
    "gitconfig",
    "zsh_plugins.zsh",
    "gitignore",
    "direnvrc",
    "tmux.conf",
];

const CONFIG_FILE_OR_DIR: &[&str] = &[
    "starship.toml",
    "zellij",
    "twm",
    "topgrade",
    "alacritty",
    "ghostty",
    "jj",
];

const CUSTOM_CONFIG_OR_DIR: &[(&str, &str)] = &[
    ("nvim", ".config/nvim"),
    ("claude", ".claude"),
    ("agents/skills", ".agents/skills"),
    ("codex/AGENTS.md", ".codex/AGENTS.md"),
    ("codex/AGENTS.md", ".config/opencode/AGENTS.md"),
    ("opencode", ".config/opencode"),
];

struct ManagedDirEntry {
    source: &'static str,
    target: &'static str,
    legacy_sources: &'static [&'static str],
}

const CUSTOM_CONFIG_DIR_ENTRIES: &[ManagedDirEntry] = &[ManagedDirEntry {
    source: "agents/skills",
    target: ".codex/skills",
    legacy_sources: &["claude/skills"],
}];

const MAC_ONLY_CUSTOM_CONFIG_OR_DIR: &[(&str, &str)] =
    &[("gpg-agent.conf", ".gnupg/gpg-agent.conf")];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncSymlinkOutcome {
    Linked,
    SkippedBrokenSource,
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = Bootstrap::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Bootstrap) -> Result<()> {
    if matches!(flags.mode, BootstrapMode::Full) {
        cmd!(sh, "rustup component add rustfmt clippy").run()?;
    }

    match Os::current() {
        Os::Linux => install::install_linux_tools(sh, flags.mode.clone())?,
        Os::MacOS => install::install_brew_and_tools(sh)?,
    }

    config(sh)
}

pub fn config(sh: &Shell) -> Result<()> {
    link::config(sh)
}

pub fn release(sh: &Shell, project: Option<String>) -> Result<()> {
    release::release(sh, project)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::ErrorKind;
    use std::os::unix::fs::symlink;

    use tempfile::tempdir;
    use xshell::Shell;

    use super::{link::sync_symlink, SyncSymlinkOutcome};

    #[test]
    fn replaces_broken_target_symlink() {
        let dir = tempdir().unwrap();
        let sh = Shell::new().unwrap();
        let source = dir.path().join("source");
        let missing = dir.path().join("missing");
        let target = dir.path().join("target");

        fs::create_dir(&source).unwrap();
        symlink(&missing, &target).unwrap();

        let outcome = sync_symlink(&sh, &source, &target).unwrap();

        assert_eq!(outcome, SyncSymlinkOutcome::Linked);
        assert_eq!(fs::read_link(&target).unwrap(), source);
    }

    #[test]
    fn skips_broken_source_symlink_and_removes_stale_target() {
        let dir = tempdir().unwrap();
        let sh = Shell::new().unwrap();
        let source = dir.path().join("source");
        let missing = dir.path().join("missing");
        let stale_source = dir.path().join("stale-source");
        let target = dir.path().join("target");

        fs::create_dir(&stale_source).unwrap();
        symlink(&missing, &source).unwrap();
        symlink(&stale_source, &target).unwrap();

        let outcome = sync_symlink(&sh, &source, &target).unwrap();

        assert_eq!(outcome, SyncSymlinkOutcome::SkippedBrokenSource);
        assert_eq!(
            fs::symlink_metadata(&target).unwrap_err().kind(),
            ErrorKind::NotFound
        );
    }

    #[test]
    fn relinks_valid_source_on_repeat_runs() {
        let dir = tempdir().unwrap();
        let sh = Shell::new().unwrap();
        let source = dir.path().join("source");
        let target = dir.path().join("target");

        fs::create_dir(&source).unwrap();

        let first = sync_symlink(&sh, &source, &target).unwrap();
        let second = sync_symlink(&sh, &source, &target).unwrap();

        assert_eq!(first, SyncSymlinkOutcome::Linked);
        assert_eq!(second, SyncSymlinkOutcome::Linked);
        assert_eq!(fs::read_link(&target).unwrap(), source);
    }
}
