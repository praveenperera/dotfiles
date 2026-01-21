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

    // get the working copy change_id
    let working_copy_id = cmd!(sh, "jj log -r @ --no-graph -T change_id.shortest(4)")
        .read()
        .wrap_err("failed to get working copy")?;

    // tab-separated: rev, bookmarks, description
    let template = r#"change_id.shortest(4) ++ "\t" ++ bookmarks.join(" ") ++ "\t" ++ if(description, description.first_line(), "") ++ "\n""#;

    // main stack: direct ancestry to @
    let main_revset = "trunk()..@";
    let main_output = cmd!(sh, "jj log -r {main_revset} --reversed --no-graph -T {template}")
        .read()
        .wrap_err("failed to get main stack")?;

    // divergent branches: descendants from stack root, excluding ancestors of @
    let divergent_revset = "descendants(roots(trunk()..@)) ~ ancestors(@)";
    let divergent_output = cmd!(sh, "jj log -r {divergent_revset} --reversed --no-graph -T {template}")
        .read()
        .wrap_err("failed to get divergent branches")?;

    #[derive(Clone)]
    struct Commit {
        rev: String,
        bookmarks: String,
        description: String,
        is_working_copy: bool,
    }

    let parse_commits = |output: &str, working_copy_id: &str| -> Vec<Commit> {
        output
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(3, '\t').collect();
                if parts.len() < 3 {
                    return None;
                }
                Some(Commit {
                    rev: parts[0].to_string(),
                    bookmarks: parts[1].to_string(),
                    description: parts[2].to_string(),
                    is_working_copy: parts[0] == working_copy_id,
                })
            })
            .collect()
    };

    let main_commits = parse_commits(&main_output, &working_copy_id);
    let divergent_commits = parse_commits(&divergent_output, &working_copy_id);

    // calculate commit counts for filtered mode (commits until next bookmark/working copy)
    let calc_commit_counts = |commits: &[Commit], full: bool| -> Vec<usize> {
        if full {
            vec![1; commits.len()]
        } else {
            let mut counts = vec![0; commits.len()];
            let mut current_count = 0;
            let mut last_visible_idx: Option<usize> = None;

            for (i, commit) in commits.iter().enumerate() {
                let is_visible = !commit.bookmarks.is_empty() || commit.is_working_copy;
                current_count += 1;

                if is_visible {
                    if let Some(prev_idx) = last_visible_idx {
                        counts[prev_idx] = current_count - 1;
                    }
                    last_visible_idx = Some(i);
                    current_count = 1;
                }
            }
            // handle the last visible commit
            if let Some(prev_idx) = last_visible_idx {
                counts[prev_idx] = current_count;
            }
            counts
        }
    };

    let format_rev = |rev: &str| -> String {
        let (prefix, suffix) = rev.split_at(2.min(rev.len()));
        format!("{}{}", prefix.purple(), suffix.dimmed())
    };

    let has_visible_commits = |commits: &[Commit], full: bool| -> bool {
        commits.iter().any(|c| full || !c.bookmarks.is_empty() || c.is_working_copy)
    };

    let print_tree = |commits: &[Commit], counts: &[usize], full: bool, base_indent: usize| {
        let mut visible_index = 0;
        for (i, commit) in commits.iter().enumerate() {
            let has_bookmark = !commit.bookmarks.is_empty();

            // skip commits without bookmarks unless --full or working copy
            if !full && !has_bookmark && !commit.is_working_copy {
                continue;
            }

            let colored_rev = format_rev(&commit.rev);

            let count = counts[i];
            let count_str = if !full && count > 1 {
                format!(" +{count}")
            } else {
                String::new()
            };

            // @ marker right before the name
            let at_marker = if commit.is_working_copy { "@ " } else { "" };

            // for commits with bookmarks: show name, desc, rev
            // for commits without: show (rev), desc (no duplicate rev at end)
            let (name, show_rev_suffix) = if commit.bookmarks.is_empty() {
                (format!("{at_marker}({}){count_str}", colored_rev), false)
            } else {
                (format!("{at_marker}{}{count_str}", commit.bookmarks.cyan()), true)
            };

            let desc = if commit.description.is_empty() {
                if commit.is_working_copy {
                    "(working copy)".dimmed().to_string()
                } else {
                    "(no description)".dimmed().to_string()
                }
            } else {
                commit.description.dimmed().to_string()
            };

            let tree_indent = "    ".repeat(base_indent + visible_index);
            if show_rev_suffix {
                println!("{tree_indent}└── {name}  {desc}  {colored_rev}");
            } else {
                println!("{tree_indent}└── {name}  {desc}");
            }
            visible_index += 1;
        }
    };

    // print main stack
    let main_counts = calc_commit_counts(&main_commits, full);
    print_tree(&main_commits, &main_counts, full, 0);

    // print divergent branches if any visible commits
    if has_visible_commits(&divergent_commits, full) {
        // find where divergent branches split from main stack
        // get the divergent root's parent (exact branch point)
        let divergent_roots_revset = "roots(descendants(roots(trunk()..@)) ~ ancestors(@))";
        let parent_template =
            r#"self.parents().map(|p| p.change_id().shortest(4)).join(" ")"#;
        let exact_parent = cmd!(
            sh,
            "jj log -r {divergent_roots_revset} --no-graph -T {parent_template} --limit 1"
        )
        .read()
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

        // find nearest bookmarked ancestor for context in filtered view
        let bookmarked_ancestor = exact_parent.as_ref().and_then(|parent| {
            let ancestor_revset = format!("ancestors({parent}) & bookmarks()");
            let ancestor_template = r#"change_id.shortest(4) ++ "\t" ++ bookmarks.join(" ")"#;
            cmd!(
                sh,
                "jj log -r {ancestor_revset} --no-graph -T {ancestor_template} --limit 1"
            )
            .read()
            .ok()
            .and_then(|s| {
                let parts: Vec<&str> = s.trim().splitn(2, '\t').collect();
                if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                    Some((parts[0].to_string(), parts[1].trim().to_string()))
                } else {
                    None
                }
            })
        });

        println!();
        match (bookmarked_ancestor, exact_parent) {
            (Some((bm_rev, bookmark)), Some(exact)) if bm_rev != exact => {
                // branch point is different from bookmarked ancestor, show both
                println!(
                    "{} (after {} {}, from {})",
                    "Divergent:".yellow(),
                    bookmark.cyan(),
                    format_rev(&bm_rev),
                    format_rev(&exact)
                );
            }
            (Some((_, bookmark)), Some(exact)) => {
                // branch point IS the bookmarked commit
                println!(
                    "{} (from {} {})",
                    "Divergent:".yellow(),
                    bookmark.cyan(),
                    format_rev(&exact)
                );
            }
            (None, Some(exact)) => {
                // no bookmarked ancestor, just show exact
                println!("{} (from {})", "Divergent:".yellow(), format_rev(&exact));
            }
            _ => {
                println!("{}", "Divergent:".yellow());
            }
        }
        let divergent_counts = calc_commit_counts(&divergent_commits, full);
        print_tree(&divergent_commits, &divergent_counts, full, 0);
    }

    Ok(())
}
