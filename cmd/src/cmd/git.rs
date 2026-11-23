//! Git helpers behind the `gm`, `gclean`, and `gupc` shell aliases

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use colored::Colorize;
use eyre::{eyre, Result};
use xshell::{cmd, Shell};

mod commit_paths;

#[derive(Debug, Clone, Parser)]
pub struct Git {
    #[command(subcommand)]
    pub subcommand: GitCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum GitCmd {
    /// Delete local branches whose upstream is gone, first removing the
    /// worktrees that have them checked out
    ///
    /// A worktree with modified or untracked files is kept along with its
    /// branch, as are the main checkout and the worktree this runs from, and
    /// every kept worktree is listed at the end
    #[command(visible_alias = "c")]
    Clean,

    /// Switch to the default branch: master, main, or trunk, whichever exists
    #[command(visible_alias = "m")]
    Main,

    /// Commit only the given files or folders, then sync with upstream and push,
    /// without touching anyone else's uncommitted work in the same checkout
    ///
    /// For checkouts that many agent threads write into at once. Untracked files
    /// that upstream adds identically move to _scratch/commit-paths-moved/, and
    /// differing ones abort before anything is committed. Local commits are
    /// rebased onto upstream in a temporary worktree (a generated INDEX.md
    /// conflict is fixed by rerunning .github/scripts/index.py --write), then the
    /// checkout moves with `git reset --keep`, which refuses rather than
    /// overwrite local changes. On the default branch every unpushed commit is
    /// pushed; on another branch only when this run's commit is the only one
    #[command(visible_alias = "cp")]
    CommitPaths(commit_paths::CommitPaths),
}

pub fn run_with_flags(sh: &Shell, flags: Git) -> Result<()> {
    match flags.subcommand {
        GitCmd::Clean => clean(sh),
        GitCmd::Main => switch_to_default(sh),
        GitCmd::CommitPaths(args) => commit_paths::run(sh, args),
    }
}

const DEFAULT_BRANCHES: [&str; 3] = ["master", "main", "trunk"];

fn switch_to_default(sh: &Shell) -> Result<()> {
    let branch = DEFAULT_BRANCHES
        .into_iter()
        .find(|branch| local_branch_exists(sh, branch))
        .ok_or_else(|| eyre!("no local {} branch", DEFAULT_BRANCHES.join(", ")))?;

    cmd!(sh, "git switch {branch}").quiet().run()?;
    Ok(())
}

fn local_branch_exists(sh: &Shell, branch: &str) -> bool {
    // `show-ref --heads <name>` also matches `feature/<name>`, so verify the full ref
    let reference = format!("refs/heads/{branch}");

    cmd!(sh, "git show-ref --verify --quiet {reference}")
        .quiet()
        .ignore_status()
        .output()
        .is_ok_and(|output| output.status.success())
}

fn clean(sh: &Shell) -> Result<()> {
    cmd!(sh, "git fetch --prune").quiet().run()?;
    cmd!(sh, "git worktree prune").quiet().run()?;

    let worktrees = Worktrees::list(sh)?;
    let mut kept = Vec::new();

    for branch in gone_branches(sh)? {
        if let Some(worktree) = clean_branch(sh, &worktrees, branch)? {
            kept.push(worktree);
        }
    }

    print_kept(&kept);
    Ok(())
}

/// Deletes `branch` and its worktree, or returns the worktree it kept
fn clean_branch(sh: &Shell, worktrees: &Worktrees, branch: String) -> Result<Option<Kept>> {
    match worktrees.cleanup_for(&branch) {
        Cleanup::Keep(path, reason) => {
            return Ok(Some(Kept {
                branch,
                path: path.to_owned(),
                reason,
            }));
        }
        Cleanup::RemoveWorktree(path) => {
            let output = cmd!(sh, "git worktree remove {path}")
                .quiet()
                .ignore_status()
                .output()?;

            // git's refusal ends with a hint to --force, so only its reason is shown
            if !output.status.success() {
                let reason = KeepReason::from_git_error(&String::from_utf8_lossy(&output.stderr));

                return Ok(Some(Kept {
                    branch,
                    path: path.to_owned(),
                    reason,
                }));
            }

            println!("Removed worktree {}", path.display());
        }
        Cleanup::DeleteBranch => {}
    }

    cmd!(sh, "git branch -D {branch}").quiet().run()?;
    Ok(None)
}

/// Lists the kept worktrees last, so they are not lost among the deletions
fn print_kept(kept: &[Kept]) {
    if kept.is_empty() {
        return;
    }

    let noun = if kept.len() == 1 {
        "worktree"
    } else {
        "worktrees"
    };
    let header = format!("Kept {} {noun} on merged branches:", kept.len());
    let branch_width = kept.iter().map(|kept| kept.branch.len()).max().unwrap_or(0);
    let reasons: Vec<String> = kept.iter().map(|kept| kept.reason.to_string()).collect();
    let reason_width = reasons.iter().map(String::len).max().unwrap_or(0);

    println!("\n{}", header.yellow().bold());

    for (kept, reason) in kept.iter().zip(&reasons) {
        // pad before coloring, since the escape codes would count toward the width
        let branch = format!("{:branch_width$}", kept.branch);
        let reason = format!("{reason:reason_width$}");

        println!(
            "  {}  {}  {}",
            branch.bold(),
            reason.yellow(),
            kept.path.display()
        );
    }
}

/// A worktree left in place, along with its branch
struct Kept {
    branch: String,
    path: PathBuf,
    reason: KeepReason,
}

/// Why a merged branch's worktree was left in place
#[derive(Debug, PartialEq, Eq)]
enum KeepReason {
    MainCheckout,
    Current,
    Unsaved,
    Locked,
    /// Any other refusal from `git worktree remove`, in git's words
    Refused(String),
}

impl KeepReason {
    fn from_git_error(stderr: &str) -> Self {
        if stderr.contains("modified or untracked files") {
            return Self::Unsaved;
        }

        if stderr.contains("locked") {
            return Self::Locked;
        }

        let message = stderr.lines().next().unwrap_or_default().trim();
        Self::Refused(message.trim_start_matches("fatal: ").to_owned())
    }
}

impl fmt::Display for KeepReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MainCheckout => f.write_str("main checkout"),
            Self::Current => f.write_str("you are in it"),
            Self::Unsaved => f.write_str("modified or untracked files"),
            Self::Locked => f.write_str("locked"),
            Self::Refused(message) => f.write_str(message),
        }
    }
}

/// Local branches whose upstream was deleted, usually after its PR merged
fn gone_branches(sh: &Shell) -> Result<Vec<String>> {
    // branch names cannot hold a tab, so it separates the fields
    let format = "--format=%(refname:short)\t%(upstream:track)";
    let refs = cmd!(sh, "git for-each-ref {format} refs/heads")
        .quiet()
        .read()?;

    Ok(parse_gone_branches(&refs))
}

fn parse_gone_branches(refs: &str) -> Vec<String> {
    refs.lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(_, track)| *track == "[gone]")
        .map(|(branch, _)| branch.to_owned())
        .collect()
}

/// What cleaning does with a branch whose upstream is gone
#[derive(Debug, PartialEq, Eq)]
enum Cleanup<'a> {
    /// No worktree has it checked out
    DeleteBranch,
    /// A linked worktree has it checked out, so that goes before the branch
    RemoveWorktree(&'a Path),
    /// It is checked out where it must stay
    Keep(&'a Path, KeepReason),
}

/// The repository's worktrees and the one this command runs from
struct Worktrees {
    /// In `git worktree list` order, so the main checkout comes first
    entries: Vec<Worktree>,
    current: Option<PathBuf>,
}

impl Worktrees {
    fn list(sh: &Shell) -> Result<Self> {
        let porcelain = cmd!(sh, "git worktree list --porcelain").quiet().read()?;

        // a bare repository has no top level, and then no worktree is current
        let current = cmd!(sh, "git rev-parse --show-toplevel")
            .quiet()
            .ignore_stderr()
            .read()
            .ok()
            .map(|path| canonical(Path::new(&path)));

        Ok(Self {
            entries: Worktree::parse_all(&porcelain),
            current,
        })
    }

    fn cleanup_for(&self, branch: &str) -> Cleanup<'_> {
        let Some((index, worktree)) = self
            .entries
            .iter()
            .enumerate()
            .find(|(_, worktree)| worktree.branch.as_deref() == Some(branch))
        else {
            return Cleanup::DeleteBranch;
        };

        if index == 0 {
            return Cleanup::Keep(&worktree.path, KeepReason::MainCheckout);
        }

        // git may record a path through a symlink, such as macOS's /var
        if self.current.as_deref() == Some(canonical(&worktree.path).as_path()) {
            return Cleanup::Keep(&worktree.path, KeepReason::Current);
        }

        Cleanup::RemoveWorktree(&worktree.path)
    }
}

/// One entry of `git worktree list --porcelain`
#[derive(Debug, PartialEq, Eq)]
struct Worktree {
    path: PathBuf,
    /// `None` for a detached HEAD or a bare repository
    branch: Option<String>,
}

impl Worktree {
    fn parse_all(porcelain: &str) -> Vec<Self> {
        porcelain.split("\n\n").filter_map(Self::parse).collect()
    }

    fn parse(block: &str) -> Option<Self> {
        let mut path = None;
        let mut branch = None;

        for line in block.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                path = Some(PathBuf::from(value));
            } else if let Some(value) = line.strip_prefix("branch refs/heads/") {
                branch = Some(value.to_owned());
            }
        }

        Some(Self {
            path: path?,
            branch,
        })
    }
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}
