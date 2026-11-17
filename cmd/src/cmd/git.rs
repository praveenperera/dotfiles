//! Git helpers behind the `gm`, `gclean`, and `gupc` shell aliases

use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use eyre::{eyre, Result};
use xshell::{cmd, Shell};

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
    /// branch, as are the main checkout and the worktree this runs from
    #[command(visible_alias = "c")]
    Clean,

    /// Switch to the default branch: master, main, or trunk, whichever exists
    #[command(visible_alias = "m")]
    Main,
}

pub fn run_with_flags(sh: &Shell, flags: Git) -> Result<()> {
    match flags.subcommand {
        GitCmd::Clean => clean(sh),
        GitCmd::Main => switch_to_default(sh),
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

    for branch in gone_branches(sh)? {
        clean_branch(sh, &worktrees, &branch)?;
    }

    Ok(())
}

fn clean_branch(sh: &Shell, worktrees: &Worktrees, branch: &str) -> Result<()> {
    match worktrees.cleanup_for(branch) {
        Cleanup::Keep(reason) => {
            println!("Kept {branch}: {reason}");
            return Ok(());
        }
        Cleanup::RemoveWorktree(path) => {
            // git prints why it refused, such as modified or untracked files
            if cmd!(sh, "git worktree remove {path}")
                .quiet()
                .run()
                .is_err()
            {
                println!("Kept {branch}: worktree {} was not removed", path.display());
                return Ok(());
            }

            println!("Removed worktree {}", path.display());
        }
        Cleanup::DeleteBranch => {}
    }

    cmd!(sh, "git branch -D {branch}").quiet().run()?;
    Ok(())
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
    /// It is checked out where removing it would pull the floor out
    Keep(&'static str),
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
            return Cleanup::Keep("checked out in the main checkout");
        }

        // git may record a path through a symlink, such as macOS's /var
        if self.current.as_deref() == Some(canonical(&worktree.path).as_path()) {
            return Cleanup::Keep("checked out in this worktree");
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
