use clap::{Parser, Subcommand};
use colored::Colorize;
use eyre::{Context as _, Result};
use log::debug;
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;
use xshell::{cmd, Shell};

/// Calculate minimum unique prefix length for each revision
fn calc_unique_prefix_lengths(revs: &[&str]) -> HashMap<String, usize> {
    let mut result = HashMap::new();
    for rev in revs {
        let mut prefix_len = 1;
        for other in revs {
            if rev == other {
                continue;
            }
            let common_len = rev
                .chars()
                .zip(other.chars())
                .take_while(|(a, b)| a == b)
                .count();
            prefix_len = prefix_len.max(common_len + 1);
        }
        result.insert(rev.to_string(), prefix_len.min(rev.len()));
    }
    result
}

#[derive(Debug, Clone, Parser)]
pub struct Jj {
    #[command(subcommand)]
    pub subcommand: JjCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum JjCmd {
    /// Sync the current stack with remote trunk (master/main/trunk)
    #[command(visible_alias = "ss")]
    StackSync {
        /// Push the first bookmark after syncing
        #[arg(short, long)]
        push: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Display the current stack as a tree
    #[command(visible_alias = "t")]
    Tree {
        /// Show all commits, including those without bookmarks
        #[arg(short, long)]
        full: bool,
    },

    /// Clean up empty divergent commits
    #[command(visible_alias = "c")]
    Clean,
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    debug!("jj args: {args:?}");
    let flags = Jj::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Jj) -> Result<()> {
    match flags.subcommand {
        JjCmd::StackSync { push, force } => stack_sync(sh, push, force),
        JjCmd::Tree { full } => tree(sh, full),
        JjCmd::Clean => clean(sh),
    }
}

/// Detect the trunk branch name (master, main, or trunk)
fn detect_trunk_branch(sh: &Shell) -> Result<String> {
    let output = cmd!(sh, "jj log -r trunk() --no-graph -T local_bookmarks --limit 1")
        .read()
        .wrap_err("failed to detect trunk branch")?;

    let trunk = output
        .split_whitespace()
        .next()
        .unwrap_or("master")
        .to_string();

    Ok(trunk)
}

fn stack_sync(sh: &Shell, push: bool, force: bool) -> Result<()> {
    println!("{}", "Fetching from remote...".dimmed());
    cmd!(sh, "jj git fetch").run().wrap_err("failed to fetch")?;

    // detect and sync trunk bookmark
    let trunk = detect_trunk_branch(sh)?;
    println!("{}{}", "Syncing ".dimmed(), trunk);
    cmd!(sh, "jj bookmark set {trunk} -r {trunk}@origin")
        .run()
        .wrap_err("failed to sync trunk bookmark")?;

    // find the root(s) of the stack
    let roots_revset = format!("roots({trunk}..@)");
    let roots_output = cmd!(
        sh,
        "jj log -r {roots_revset} --no-graph -T 'change_id.short() ++ \"\\n\"'"
    )
    .read()
    .wrap_err("failed to find stack roots")?;

    let roots: Vec<&str> = roots_output.lines().filter(|l| !l.is_empty()).collect();
    debug!("stack roots: {:?}", roots);

    if roots.is_empty() {
        println!(
            "{}{}{}",
            "No commits after ".dimmed(),
            trunk,
            ", nothing to rebase".dimmed()
        );
        return Ok(());
    }

    // show confirmation unless --force
    if !force {
        println!("Will rebase the following commits on top of {}:", trunk.cyan());
        for root in &roots {
            let desc = cmd!(sh, "jj log -r {root} --no-graph -T description.first_line()")
                .read()
                .unwrap_or_default();
            println!("  {}  {}", root.purple(), desc.dimmed());
            println!(
                "  {}",
                format!("jj rebase --source (-s) {root} --onto (-o) {trunk} --skip-emptied").dimmed()
            );
        }
        print!("Continue? [y/N] ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("{}", "Aborted".yellow());
            return Ok(());
        }
    }

    // rebase from each root (usually just one)
    // --skip-emptied handles merged commits by abandoning ones that became empty
    for root in &roots {
        println!("{}{}...", "Rebasing stack from ".dimmed(), root);
        cmd!(sh, "jj rebase --source {root} --onto {trunk} --skip-emptied")
            .run()
            .wrap_err_with(|| format!("failed to rebase from {root}"))?;
    }

    // clean up bookmarks marked as deleted on remote (after rebase so --skip-emptied can work)
    let tracked = cmd!(sh, "jj bookmark list --tracked")
        .read()
        .wrap_err("failed to list tracked bookmarks")?;

    for line in tracked.lines() {
        if line.contains("[deleted]") {
            if let Some(bookmark) = line.split_whitespace().next() {
                println!("{}{}", "Deleting merged bookmark: ".dimmed(), bookmark);
                cmd!(sh, "jj bookmark delete {bookmark}")
                    .run()
                    .wrap_err_with(|| format!("failed to delete bookmark {bookmark}"))?;
            }
        }
    }

    if push {
        // find and push the first bookmark in the rebased stack
        let revset = format!("({trunk}..@) & bookmarks()");
        let template = r#"bookmarks ++ "\n""#;
        let output = cmd!(
            sh,
            "jj log -r {revset} --reversed --no-graph -T {template} --limit 1"
        )
        .read()
        .wrap_err("failed to get bookmarks")?;

        if let Some(bookmark) = output.lines().find(|l| !l.is_empty()) {
            let bookmark = bookmark.trim();
            println!("{}{}...", "Pushing ".dimmed(), bookmark);
            cmd!(sh, "jj git push --bookmark {bookmark}")
                .run()
                .wrap_err("failed to push")?;
        } else {
            println!("{}", "No bookmarks found to push".dimmed());
        }
    }

    println!("{}", "Stack sync complete".green());
    Ok(())
}

fn tree(sh: &Shell, full: bool) -> Result<()> {
    use std::collections::HashSet;

    // get the working copy change_id
    let working_copy_id = cmd!(sh, "jj log -r @ --no-graph -T change_id.shortest(4)")
        .read()
        .wrap_err("failed to get working copy")?;

    // single revset for all commits: descendants of roots of trunk()..@
    let revset = "descendants(roots(trunk()..@))";

    // tab-separated: rev, bookmarks, description, parent_revs
    let template = r#"change_id.shortest(4) ++ "\t" ++ bookmarks.join(" ") ++ "\t" ++ if(description, description.first_line(), "") ++ "\t" ++ self.parents().map(|p| p.change_id().shortest(4)).join(",") ++ "\n""#;

    let output = cmd!(sh, "jj log -r {revset} --reversed --no-graph -T {template}")
        .read()
        .wrap_err("failed to get commits")?;

    #[derive(Clone)]
    struct Commit {
        rev: String,
        bookmarks: String,
        description: String,
        parent_revs: Vec<String>,
        is_working_copy: bool,
    }

    // parse commits into a map
    let mut commit_map: HashMap<String, Commit> = HashMap::new();
    for line in output.lines().filter(|l| !l.is_empty()) {
        let parts: Vec<&str> = line.splitn(4, '\t').collect();
        if parts.len() < 4 {
            continue;
        }
        let rev = parts[0].to_string();
        let parent_revs: Vec<String> = parts[3]
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        commit_map.insert(
            rev.clone(),
            Commit {
                rev,
                bookmarks: parts[1].to_string(),
                description: parts[2].to_string(),
                parent_revs,
                is_working_copy: parts[0] == working_copy_id,
            },
        );
    }

    if commit_map.is_empty() {
        return Ok(());
    }

    // build children map
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    for commit in commit_map.values() {
        for parent in &commit.parent_revs {
            children_map
                .entry(parent.clone())
                .or_default()
                .push(commit.rev.clone());
        }
    }

    // find roots (commits whose parents aren't in our set)
    let revs_in_set: HashSet<&str> = commit_map.keys().map(|s| s.as_str()).collect();
    let mut roots: Vec<String> = commit_map
        .values()
        .filter(|c| c.parent_revs.iter().all(|p| !revs_in_set.contains(p.as_str())))
        .map(|c| c.rev.clone())
        .collect();
    roots.sort();

    // calculate minimum unique prefix lengths for all revisions
    let all_revs: Vec<&str> = commit_map.keys().map(|s| s.as_str()).collect();
    let prefix_lengths = calc_unique_prefix_lengths(&all_revs);

    let format_rev = |rev: &str| -> String {
        let len = prefix_lengths.get(rev).copied().unwrap_or(2);
        let (prefix, suffix) = rev.split_at(len.min(rev.len()));
        format!("{}{}", prefix.purple(), suffix.dimmed())
    };

    // determine visibility for filtered mode
    let is_visible = |commit: &Commit| -> bool {
        full || !commit.bookmarks.is_empty() || commit.is_working_copy
    };

    // count hidden commits between visible ancestors and a commit
    fn count_hidden_between(
        commit_map: &HashMap<String, Commit>,
        children_map: &HashMap<String, Vec<String>>,
        from: &str,
        to: &str,
        is_visible_fn: &dyn Fn(&Commit) -> bool,
    ) -> usize {
        // BFS from `from` to `to`, counting non-visible commits in between
        let mut count = 0;
        let mut current = from.to_string();

        while let Some(children) = children_map.get(&current) {
            // find the child that leads to `to`
            let next = children.iter().find(|c| {
                if *c == to {
                    return true;
                }
                // check if `to` is a descendant of this child
                let mut stack = vec![c.as_str()];
                let mut visited = HashSet::new();
                while let Some(n) = stack.pop() {
                    if n == to {
                        return true;
                    }
                    if visited.insert(n) {
                        if let Some(grandchildren) = children_map.get(n) {
                            stack.extend(grandchildren.iter().map(|s| s.as_str()));
                        }
                    }
                }
                false
            });

            match next {
                Some(n) if n == to => break,
                Some(n) => {
                    if let Some(c) = commit_map.get(n) {
                        if !is_visible_fn(c) {
                            count += 1;
                        }
                    }
                    current = n.clone();
                }
                None => break,
            }
        }
        count
    }

    // check if a rev or any of its descendants are visible
    fn has_visible_descendant(
        rev: &str,
        commit_map: &HashMap<String, Commit>,
        children_map: &HashMap<String, Vec<String>>,
        is_visible_fn: &dyn Fn(&Commit) -> bool,
    ) -> bool {
        if let Some(commit) = commit_map.get(rev) {
            if is_visible_fn(commit) {
                return true;
            }
        }
        if let Some(children) = children_map.get(rev) {
            for child in children {
                if has_visible_descendant(child, commit_map, children_map, is_visible_fn) {
                    return true;
                }
            }
        }
        false
    }

    // recursive tree printing
    #[allow(clippy::too_many_arguments)]
    fn print_subtree(
        rev: &str,
        commit_map: &HashMap<String, Commit>,
        children_map: &HashMap<String, Vec<String>>,
        prefix: &str,
        is_last: bool,
        full: bool,
        hidden_count: usize,
        is_visible_fn: &dyn Fn(&Commit) -> bool,
        format_rev_fn: &dyn Fn(&str) -> String,
    ) {
        let commit = match commit_map.get(rev) {
            Some(c) => c,
            None => return,
        };

        let visible = is_visible_fn(commit);

        // get children with visible descendants
        let children: Vec<&String> = children_map
            .get(rev)
            .map(|c| {
                c.iter()
                    .filter(|child| {
                        has_visible_descendant(child, commit_map, children_map, is_visible_fn)
                    })
                    .collect()
            })
            .unwrap_or_default();

        // if not visible, pass through to children with accumulated hidden count
        if !visible {
            for (i, child) in children.iter().enumerate() {
                let is_last_child = i == children.len() - 1;
                print_subtree(
                    child,
                    commit_map,
                    children_map,
                    prefix,
                    is_last && is_last_child,
                    full,
                    hidden_count + 1,
                    is_visible_fn,
                    format_rev_fn,
                );
            }
            return;
        }

        // print this commit
        let connector = if is_last { "└── " } else { "├── " };
        let colored_rev = format_rev_fn(rev);

        let count_str = if !full && hidden_count > 0 {
            format!(" +{hidden_count}")
        } else {
            String::new()
        };

        let at_marker = if commit.is_working_copy { "@ " } else { "" };

        let (name, show_rev_suffix) = if commit.bookmarks.is_empty() {
            (format!("{at_marker}({}){count_str}", colored_rev), false)
        } else {
            (
                format!("{at_marker}{}{count_str}", commit.bookmarks.cyan()),
                true,
            )
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

        if show_rev_suffix {
            println!("{prefix}{connector}{name}  {desc}  {colored_rev}");
        } else {
            println!("{prefix}{connector}{name}  {desc}");
        }

        // calculate new prefix for children
        let child_prefix = if is_last {
            format!("{prefix}    ")
        } else {
            format!("{prefix}│   ")
        };

        // print children
        for (i, child) in children.iter().enumerate() {
            let is_last_child = i == children.len() - 1;

            // count hidden commits between this visible commit and the child
            let child_hidden = if full {
                0
            } else {
                count_hidden_between(commit_map, children_map, rev, child, is_visible_fn)
            };

            print_subtree(
                child,
                commit_map,
                children_map,
                &child_prefix,
                is_last_child,
                full,
                child_hidden,
                is_visible_fn,
                format_rev_fn,
            );
        }
    }

    // print each root as a separate tree
    for (i, root) in roots.iter().enumerate() {
        let is_last_root = i == roots.len() - 1;
        print_subtree(
            root,
            &commit_map,
            &children_map,
            "",
            is_last_root,
            full,
            0,
            &is_visible,
            &format_rev,
        );
    }

    Ok(())
}

fn clean(sh: &Shell) -> Result<()> {
    let revset = "all() ~ root()";

    // find non-empty divergent commits (need manual resolution)
    let nonempty_template = r#"if(divergent && !empty, change_id.short() ++ " ", "")"#;
    let nonempty = cmd!(sh, "jj log -r {revset} -T {nonempty_template} --no-graph")
        .read()
        .unwrap_or_default()
        .split_whitespace()
        .map(String::from)
        .collect::<Vec<_>>();

    if !nonempty.is_empty() {
        println!(
            "{} non-empty divergent commits need manual resolution: {}",
            "Warning:".yellow(),
            nonempty.join(" ")
        );
    }

    // find empty divergent commits
    let empty_template = r#"if(divergent && empty, change_id.short() ++ " ", "")"#;
    let empty = cmd!(sh, "jj log -r {revset} -T {empty_template} --no-graph")
        .read()
        .unwrap_or_default()
        .split_whitespace()
        .map(String::from)
        .collect::<Vec<_>>();

    if empty.is_empty() {
        if nonempty.is_empty() {
            println!("{}", "No divergent commits found".dimmed());
        }
        return Ok(());
    }

    println!("{}{}", "Abandoning empty divergent commits: ".dimmed(), empty.join(" "));
    for rev in &empty {
        cmd!(sh, "jj abandon {rev}").run()?;
    }

    Ok(())
}
