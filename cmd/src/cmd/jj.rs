use clap::{Parser, Subcommand};
use eyre::{Context as _, Result};
use log::{debug, info};
use std::ffi::OsString;
use xshell::{cmd, Shell};

#[derive(Debug, Clone, Parser)]
pub struct Jj {
    #[command(subcommand)]
    pub subcommand: JjCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum JjCmd {
    /// Sync the current stack with remote master
    #[command(visible_alias = "ss")]
    StackSync,

    /// Display the current stack as a tree
    #[command(visible_alias = "t")]
    Tree,
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    debug!("jj args: {args:?}");
    let flags = Jj::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Jj) -> Result<()> {
    match flags.subcommand {
        JjCmd::StackSync => stack_sync(sh),
        JjCmd::Tree => tree(sh),
    }
}

fn stack_sync(sh: &Shell) -> Result<()> {
    info!("Fetching from remote...");
    cmd!(sh, "jj git fetch").run().wrap_err("failed to fetch")?;

    // find the first bookmark after master in the stack
    let revset = "(master::@) & bookmarks()";
    let template = r#"bookmarks ++ "\n""#;
    let output = cmd!(
        sh,
        "jj log -r {revset} --reversed --no-graph -T {template} --limit 2"
    )
    .read()
    .wrap_err("failed to get bookmarks in stack")?;

    let bookmarks: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
    debug!("bookmarks in stack: {:?}", bookmarks);

    // get the second bookmark (first after master)
    let Some(next_bookmark) = bookmarks.get(1) else {
        info!("No bookmarks found after master in the stack");
        return Ok(());
    };

    let next_bookmark = next_bookmark.trim();
    info!("Rebasing stack starting at {next_bookmark} onto master...");
    cmd!(sh, "jj rebase -s {next_bookmark} -d master")
        .run()
        .wrap_err("failed to rebase")?;

    // delete local bookmarks that were deleted on origin (merged PRs)
    let tracked = cmd!(sh, "jj bookmark list --tracked")
        .read()
        .wrap_err("failed to list tracked bookmarks")?;

    for line in tracked.lines() {
        if line.contains("[deleted]") {
            if let Some(bookmark) = line.split_whitespace().next() {
                info!("Deleting merged bookmark: {bookmark}");
                cmd!(sh, "jj bookmark delete {bookmark}")
                    .run()
                    .wrap_err_with(|| format!("failed to delete bookmark {bookmark}"))?;
            }
        }
    }

    info!("Pushing {next_bookmark}...");
    cmd!(sh, "jj git push --bookmark {next_bookmark}")
        .run()
        .wrap_err("failed to push")?;

    info!("Stack sync complete");
    Ok(())
}

fn tree(sh: &Shell) -> Result<()> {
    let output = cmd!(sh, "jj stack")
        .read()
        .wrap_err("failed to get stack")?;

    for (i, line) in output.lines().filter(|l| !l.is_empty()).enumerate() {
        let indent = "    ".repeat(i);
        println!("{indent}└── {line}");
    }

    Ok(())
}
