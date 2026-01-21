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
    Tree {
        /// Show all commits, including those without bookmarks
        #[arg(short, long)]
        full: bool,
    },
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    debug!("jj args: {args:?}");
    let flags = Jj::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Jj) -> Result<()> {
    match flags.subcommand {
        JjCmd::StackSync { push } => stack_sync(sh, push),
        JjCmd::Tree { full } => tree(sh, full),
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

fn tree(sh: &Shell, full: bool) -> Result<()> {
    use colored::Colorize;

    let revset = "descendants(roots(trunk()..@))";
    // tab-separated: full_rev, is_working_copy, bookmarks, description
    let template = r#"change_id.shortest(4) ++ "\t" ++ if(working_copies, "true", "false") ++ "\t" ++ bookmarks.join(" ") ++ "\t" ++ if(description, description.first_line(), "") ++ "\n""#;

    let output = cmd!(sh, "jj log -r {revset} --reversed --no-graph -T {template}")
        .read()
        .wrap_err("failed to get stack")?;

    let mut visible_index = 0;
    for line in output.lines().filter(|l| !l.is_empty()) {
        let parts: Vec<&str> = line.splitn(4, '\t').collect();
        if parts.len() < 4 {
            continue;
        }

        let full_rev = parts[0];
        let is_working_copy = parts[1] == "true";
        let bookmarks = parts[2];
        let description = parts[3];

        let has_bookmark = !bookmarks.is_empty();

        // skip commits without bookmarks unless --full or working copy
        if !full && !has_bookmark && !is_working_copy {
            continue;
        }

        let (prefix, suffix) = full_rev.split_at(2.min(full_rev.len()));
        let colored_rev = format!("{}{}", prefix.purple(), suffix.dimmed());

        let name = if bookmarks.is_empty() {
            if is_working_copy {
                format!("{} ({})", "@".cyan(), colored_rev)
            } else {
                colored_rev
            }
        } else {
            format!("{} ({})", bookmarks.cyan(), colored_rev)
        };

        let desc = if description.is_empty() {
            if is_working_copy {
                "(working copy)".dimmed().to_string()
            } else {
                "(no description)".dimmed().to_string()
            }
        } else {
            description.dimmed().to_string()
        };

        let indent = "    ".repeat(visible_index);
        println!("{indent}└── {name} -- {desc}");
        visible_index += 1;
    }

    Ok(())
}
