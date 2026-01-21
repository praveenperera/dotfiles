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
    StackSync {
        /// Push the first bookmark after syncing
        #[arg(short, long)]
        push: bool,
    },

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
        JjCmd::StackSync { push } => stack_sync(sh, push),
        JjCmd::Tree => tree(sh),
    }
}

fn stack_sync(sh: &Shell, push: bool) -> Result<()> {
    info!("Fetching from remote...");
    cmd!(sh, "jj git fetch").run().wrap_err("failed to fetch")?;

    // find the root(s) of the stack (first commit(s) after master)
    let roots_output = cmd!(
        sh,
        "jj log -r 'roots(master..@)' --no-graph -T 'change_id.short() ++ \"\\n\"'"
    )
    .read()
    .wrap_err("failed to find stack roots")?;

    let roots: Vec<&str> = roots_output.lines().filter(|l| !l.is_empty()).collect();
    debug!("stack roots: {:?}", roots);

    if roots.is_empty() {
        info!("No commits after master, nothing to rebase");
        return Ok(());
    }

    // rebase from each root (usually just one)
    for root in &roots {
        info!("Rebasing stack from {root} onto master...");
        cmd!(sh, "jj rebase -s {root} -d master")
            .run()
            .wrap_err_with(|| format!("failed to rebase from {root}"))?;
    }

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

    if push {
        // find and push the first bookmark in the rebased stack
        let revset = "(master..@) & bookmarks()";
        let template = r#"bookmarks ++ "\n""#;
        let output = cmd!(
            sh,
            "jj log -r {revset} --reversed --no-graph -T {template} --limit 1"
        )
        .read()
        .wrap_err("failed to get bookmarks")?;

        if let Some(bookmark) = output.lines().find(|l| !l.is_empty()) {
            let bookmark = bookmark.trim();
            info!("Pushing {bookmark}...");
            cmd!(sh, "jj git push --bookmark {bookmark}")
                .run()
                .wrap_err("failed to push")?;
        } else {
            info!("No bookmarks found to push");
        }
    }

    info!("Stack sync complete");
    Ok(())
}

fn tree(sh: &Shell) -> Result<()> {
    let revset = "descendants(roots(trunk()..@))";
    let template = r#"change_id.shortest(4) ++ " " ++ if(description, description.first_line(), "(no description)")"#;

    let output = cmd!(sh, "jj log -r {revset} --reversed --no-graph -T {template}")
        .read()
        .wrap_err("failed to get stack")?;

    for (i, line) in output.lines().filter(|l| !l.is_empty()).enumerate() {
        let indent = "    ".repeat(i);
        println!("{indent}└── {line}");
    }

    Ok(())
}
