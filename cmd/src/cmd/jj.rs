use crate::jj_lib_helpers::JjRepo;
use clap::{Parser, Subcommand};
use colored::Colorize;
use eyre::{bail, Context as _, Result};
use log::debug;
use regex::Regex;
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

    /// Rebase a revision onto trunk (auto-detected master/main/trunk)
    #[command(visible_aliases = ["ro", "up"])]
    RebaseOnto {
        /// Source revision to rebase (default: @)
        #[arg(default_value = "@")]
        revision: String,

        /// Also move the trunk bookmark to the rebased revision
        #[arg(short, long)]
        update: bool,
    },

    /// Split hunks from a commit non-interactively
    #[command(visible_alias = "sh")]
    SplitHunk {
        /// Commit message for the new commit (required unless --preview)
        #[arg(short, long)]
        message: Option<String>,

        /// Revision to split (default: @)
        #[arg(short, long, default_value = "@")]
        revision: String,

        /// File to select hunks from
        #[arg(long)]
        file: Option<String>,

        /// Line ranges to select (e.g., "10-20,30-40")
        #[arg(long)]
        lines: Option<String>,

        /// Hunk indices to select (e.g., "0,2,5")
        #[arg(long)]
        hunks: Option<String>,

        /// Regex pattern to match hunk content
        #[arg(long)]
        pattern: Option<String>,

        /// Preview hunks with indices (don't split)
        #[arg(long)]
        preview: bool,

        /// Exclude matched hunks instead of including
        #[arg(long)]
        invert: bool,

        /// Show what would be committed without committing
        #[arg(long)]
        dry_run: bool,
    },
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
        JjCmd::RebaseOnto { revision, update } => rebase_onto(sh, &revision, update),
        JjCmd::SplitHunk {
            message,
            revision,
            file,
            lines,
            hunks,
            pattern,
            preview,
            invert,
            dry_run,
        } => split_hunk(
            sh, message, &revision, file, lines, hunks, pattern, preview, invert, dry_run,
        ),
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

fn rebase_onto(sh: &Shell, revision: &str, update: bool) -> Result<()> {
    let trunk = detect_trunk_branch(sh)?;
    println!(
        "{}{}{}{}",
        "Rebasing ".dimmed(),
        revision.cyan(),
        " onto ".dimmed(),
        trunk.cyan()
    );
    cmd!(sh, "jj rebase --source {revision} -o {trunk} --skip-emptied")
        .run()
        .wrap_err("rebase failed")?;

    if update {
        println!(
            "{}{}{}{}",
            "Setting ".dimmed(),
            trunk.cyan(),
            " to ".dimmed(),
            revision.cyan()
        );
        cmd!(sh, "jj bookmark set {trunk} -r {revision}")
            .run()
            .wrap_err("failed to set bookmark")?;
    }

    println!("{}", "Done".green());
    Ok(())
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

fn tree(_sh: &Shell, full: bool) -> Result<()> {
    use std::collections::HashSet;

    let jj_repo = JjRepo::load(None)?;

    // get the working copy change_id
    let working_copy = jj_repo.working_copy_commit()?;
    let working_copy_id = jj_repo.shortest_change_id(&working_copy, 4)?;

    // get all commits in descendants(roots(trunk()..@))
    let commits = jj_repo.eval_revset("descendants(roots(trunk()..@))")?;

    #[derive(Clone)]
    struct TreeCommit {
        rev: String,
        bookmarks: String,
        description: String,
        parent_revs: Vec<String>,
        is_working_copy: bool,
    }

    // build commit map with shortest change IDs
    let mut commit_map: HashMap<String, TreeCommit> = HashMap::new();

    for commit in &commits {
        let rev = jj_repo.shortest_change_id(commit, 4)?;
        let bookmarks = jj_repo.bookmarks_at(commit).join(" ");
        let description = JjRepo::description_first_line(commit);

        // get parent change IDs
        let parents = jj_repo.parent_commits(commit)?;
        let parent_revs: Vec<String> = parents
            .iter()
            .filter_map(|p| jj_repo.shortest_change_id(p, 4).ok())
            .collect();

        let is_working_copy = rev == working_copy_id;

        commit_map.insert(
            rev.clone(),
            TreeCommit {
                rev,
                bookmarks,
                description,
                parent_revs,
                is_working_copy,
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
    let is_visible = |commit: &TreeCommit| -> bool {
        full || !commit.bookmarks.is_empty() || commit.is_working_copy
    };

    // count hidden commits between visible ancestors and a commit
    fn count_hidden_between(
        commit_map: &HashMap<String, TreeCommit>,
        children_map: &HashMap<String, Vec<String>>,
        from: &str,
        to: &str,
        is_visible_fn: &dyn Fn(&TreeCommit) -> bool,
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
        commit_map: &HashMap<String, TreeCommit>,
        children_map: &HashMap<String, Vec<String>>,
        is_visible_fn: &dyn Fn(&TreeCommit) -> bool,
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
        commit_map: &HashMap<String, TreeCommit>,
        children_map: &HashMap<String, Vec<String>>,
        prefix: &str,
        is_last: bool,
        full: bool,
        hidden_count: usize,
        is_visible_fn: &dyn Fn(&TreeCommit) -> bool,
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

fn clean(_sh: &Shell) -> Result<()> {
    let jj_repo = JjRepo::load(None)?;

    // get all commits except root
    let commits = jj_repo.eval_revset("all() ~ root()")?;

    let mut nonempty_divergent = Vec::new();
    let mut empty_divergent = Vec::new();

    for commit in &commits {
        // check if divergent by looking for multiple commits with same change_id
        let change_id_hex = commit.change_id().reverse_hex();
        let divergent_revset = format!("{}+", &change_id_hex[..12]);
        let same_change_commits = jj_repo.eval_revset(&divergent_revset).unwrap_or_default();
        let is_divergent = same_change_commits.len() > 1;

        if is_divergent {
            let is_empty = jj_repo.is_commit_empty(commit).unwrap_or(false);
            let short_id = jj_repo.shortest_change_id(commit, 8)?;

            if is_empty {
                empty_divergent.push((short_id, commit.clone()));
            } else {
                nonempty_divergent.push(short_id);
            }
        }
    }

    if !nonempty_divergent.is_empty() {
        println!(
            "{} non-empty divergent commits need manual resolution: {}",
            "Warning:".yellow(),
            nonempty_divergent.join(" ")
        );
    }

    if empty_divergent.is_empty() {
        if nonempty_divergent.is_empty() {
            println!("{}", "No divergent commits found".dimmed());
        }
        return Ok(());
    }

    let ids: Vec<_> = empty_divergent.iter().map(|(id, _)| id.as_str()).collect();
    println!(
        "{}{}",
        "Abandoning empty divergent commits: ".dimmed(),
        ids.join(" ")
    );

    for (_, commit) in &empty_divergent {
        jj_repo.abandon(commit)?;
    }

    Ok(())
}

// ============================================================================
// Split Hunk Implementation
// ============================================================================

#[derive(Debug, Clone)]
struct DiffHunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
struct DiffLine {
    kind: DiffLineKind,
    content: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DiffLineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug)]
struct FileDiff {
    path: String,
    is_new: bool,
    is_deleted: bool,
    is_binary: bool,
    hunks: Vec<DiffHunk>,
}

/// Parse git diff output into structured FileDiff objects
fn parse_diff_output(output: &str) -> Vec<FileDiff> {
    let mut files = Vec::new();
    let mut current_file: Option<FileDiff> = None;
    let mut current_hunk: Option<DiffHunk> = None;

    let hunk_header_re = Regex::new(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@").unwrap();

    for line in output.lines() {
        // new file header
        if line.starts_with("diff --git ") {
            // save previous file
            if let Some(mut file) = current_file.take() {
                if let Some(hunk) = current_hunk.take() {
                    file.hunks.push(hunk);
                }
                files.push(file);
            }

            // extract path from "diff --git a/path b/path"
            let path = line
                .strip_prefix("diff --git a/")
                .and_then(|s| s.split(" b/").next())
                .unwrap_or("")
                .to_string();

            current_file = Some(FileDiff {
                path,
                is_new: false,
                is_deleted: false,
                is_binary: false,
                hunks: Vec::new(),
            });
            continue;
        }

        let Some(ref mut file) = current_file else {
            continue;
        };

        // check for new/deleted/binary markers (only before hunk content starts)
        if current_hunk.is_none() {
            if line.starts_with("new file mode") {
                file.is_new = true;
                continue;
            }
            if line.starts_with("deleted file mode") {
                file.is_deleted = true;
                continue;
            }
            if line.starts_with("Binary files") || line.starts_with("GIT binary patch") {
                file.is_binary = true;
                continue;
            }
        }

        // hunk header
        if let Some(caps) = hunk_header_re.captures(line) {
            // save previous hunk
            if let Some(hunk) = current_hunk.take() {
                file.hunks.push(hunk);
            }

            let old_start = caps.get(1).map_or(1, |m| m.as_str().parse().unwrap_or(1));
            let old_count = caps.get(2).map_or(1, |m| m.as_str().parse().unwrap_or(1));
            let new_start = caps.get(3).map_or(1, |m| m.as_str().parse().unwrap_or(1));
            let new_count = caps.get(4).map_or(1, |m| m.as_str().parse().unwrap_or(1));

            current_hunk = Some(DiffHunk {
                old_start,
                old_count,
                new_start,
                new_count,
                lines: Vec::new(),
            });
            continue;
        }

        // diff content lines
        if let Some(ref mut hunk) = current_hunk {
            let (kind, content) = if let Some(rest) = line.strip_prefix('+') {
                (DiffLineKind::Added, rest.to_string())
            } else if let Some(rest) = line.strip_prefix('-') {
                (DiffLineKind::Removed, rest.to_string())
            } else if let Some(rest) = line.strip_prefix(' ') {
                (DiffLineKind::Context, rest.to_string())
            } else if line.starts_with('\\') {
                // "\ No newline at end of file" - skip
                continue;
            } else {
                continue;
            };

            hunk.lines.push(DiffLine { kind, content });
        }
    }

    // save final file and hunk
    if let Some(mut file) = current_file {
        if let Some(hunk) = current_hunk {
            file.hunks.push(hunk);
        }
        files.push(file);
    }

    files
}

/// Preview hunks with indices
fn preview_hunks(sh: &Shell, revision: &str, file_filter: Option<&str>) -> Result<()> {
    let diff_output = if let Some(file) = file_filter {
        cmd!(sh, "jj diff -r {revision} --git {file}")
            .read()
            .wrap_err("failed to get diff")?
    } else {
        cmd!(sh, "jj diff -r {revision} --git")
            .read()
            .wrap_err("failed to get diff")?
    };

    if diff_output.trim().is_empty() {
        println!("{}", "No changes in revision".dimmed());
        return Ok(());
    }

    let files = parse_diff_output(&diff_output);
    let mut global_index = 0;

    for file in &files {
        println!(
            "\n{} {}",
            "Hunks in".dimmed(),
            file.path.cyan()
        );

        if file.is_binary {
            println!("  {}", "[binary file]".yellow());
            continue;
        }

        if file.is_new {
            println!("  {}", "(new file)".green());
        } else if file.is_deleted {
            println!("  {}", "(deleted file)".red());
        }

        for hunk in &file.hunks {
            let change_type = categorize_hunk(hunk);
            println!(
                "\n{}{}{}  {} (lines {}-{}):",
                "[".dimmed(),
                global_index.to_string().yellow(),
                "]".dimmed(),
                change_type,
                hunk.new_start,
                hunk.new_start + hunk.new_count.saturating_sub(1)
            );

            // show a few lines of context
            let max_preview_lines = 6;
            for (shown, line) in hunk.lines.iter().enumerate() {
                if shown >= max_preview_lines {
                    let remaining = hunk.lines.len() - shown;
                    if remaining > 0 {
                        println!("    {} more lines...", format!("+{remaining}").dimmed());
                    }
                    break;
                }
                let prefix = match line.kind {
                    DiffLineKind::Added => "+".green(),
                    DiffLineKind::Removed => "-".red(),
                    DiffLineKind::Context => " ".normal(),
                };
                println!("    {}{}", prefix, line.content.dimmed());
            }

            global_index += 1;
        }
    }

    let total_hunks: usize = files.iter().map(|f| f.hunks.len()).sum();
    println!("\n{} {} hunks", "Total:".dimmed(), total_hunks);

    Ok(())
}

fn categorize_hunk(hunk: &DiffHunk) -> colored::ColoredString {
    let has_added = hunk.lines.iter().any(|l| l.kind == DiffLineKind::Added);
    let has_removed = hunk.lines.iter().any(|l| l.kind == DiffLineKind::Removed);

    match (has_added, has_removed) {
        (true, true) => "modified".yellow(),
        (true, false) => "added".green(),
        (false, true) => "removed".red(),
        (false, false) => "context".dimmed(),
    }
}

/// Parse line ranges like "10-20,30-40" into Vec<(start, end)>
fn parse_line_ranges(input: &str) -> Result<Vec<(usize, usize)>> {
    let mut ranges = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start, end)) = part.split_once('-') {
            let start: usize = start.trim().parse().wrap_err("invalid line range start")?;
            let end: usize = end.trim().parse().wrap_err("invalid line range end")?;
            ranges.push((start, end));
        } else {
            let line: usize = part.parse().wrap_err("invalid line number")?;
            ranges.push((line, line));
        }
    }
    Ok(ranges)
}

/// Parse hunk indices like "0,2,5" into Vec<usize>
fn parse_hunk_indices(input: &str) -> Result<Vec<usize>> {
    input
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().parse::<usize>().wrap_err("invalid hunk index"))
        .collect()
}

/// Check if a hunk overlaps with any of the given line ranges
fn hunk_overlaps_lines(hunk: &DiffHunk, ranges: &[(usize, usize)]) -> bool {
    let hunk_start = hunk.new_start;
    let hunk_end = hunk.new_start + hunk.new_count.saturating_sub(1);

    ranges
        .iter()
        .any(|&(start, end)| hunk_start <= end && hunk_end >= start)
}

/// Check if a hunk matches the given pattern
fn hunk_matches_pattern(hunk: &DiffHunk, pattern: &Regex) -> bool {
    hunk.lines
        .iter()
        .any(|line| pattern.is_match(&line.content))
}

/// Select which hunks to include based on criteria
fn select_hunks(
    files: &[FileDiff],
    hunk_indices: Option<&[usize]>,
    line_ranges: Option<&[(usize, usize)]>,
    pattern: Option<&Regex>,
    invert: bool,
) -> Vec<(usize, usize)> {
    // (file_index, hunk_index)
    let mut selected = Vec::new();
    let mut global_index = 0;

    for (file_idx, file) in files.iter().enumerate() {
        if file.is_binary {
            continue;
        }

        for (hunk_idx, hunk) in file.hunks.iter().enumerate() {
            // if no criteria, select all
            let mut matches =
                hunk_indices.is_none() && line_ranges.is_none() && pattern.is_none();

            if let Some(indices) = hunk_indices {
                if indices.contains(&global_index) {
                    matches = true;
                }
            }

            if let Some(ranges) = line_ranges {
                if hunk_overlaps_lines(hunk, ranges) {
                    matches = true;
                }
            }

            if let Some(pat) = pattern {
                if hunk_matches_pattern(hunk, pat) {
                    matches = true;
                }
            }

            // apply invert
            if invert {
                matches = !matches;
            }

            if matches {
                selected.push((file_idx, hunk_idx));
            }

            global_index += 1;
        }
    }

    selected
}

/// Apply selected hunks to parent content to produce new content
fn apply_selected_hunks(parent_content: &str, hunks: &[&DiffHunk]) -> String {
    if hunks.is_empty() {
        return parent_content.to_string();
    }

    let parent_lines: Vec<&str> = parent_content.lines().collect();
    let mut result = Vec::new();
    let mut parent_idx = 0;

    for hunk in hunks {
        // copy unchanged lines before this hunk
        let hunk_start = hunk.old_start.saturating_sub(1);
        while parent_idx < hunk_start && parent_idx < parent_lines.len() {
            result.push(parent_lines[parent_idx].to_string());
            parent_idx += 1;
        }

        // apply hunk: add context and added lines, skip removed lines
        for line in &hunk.lines {
            match line.kind {
                DiffLineKind::Context | DiffLineKind::Added => {
                    result.push(line.content.clone());
                }
                DiffLineKind::Removed => {
                    // skip removed lines, but advance parent_idx for tracking
                }
            }
        }

        // skip over the old lines that this hunk replaced
        parent_idx = hunk.old_start.saturating_sub(1) + hunk.old_count;
    }

    // copy remaining lines after last hunk
    while parent_idx < parent_lines.len() {
        result.push(parent_lines[parent_idx].to_string());
        parent_idx += 1;
    }

    let mut output = result.join("\n");
    // preserve trailing newline if parent had one
    if parent_content.ends_with('\n') {
        output.push('\n');
    }
    output
}

#[allow(clippy::too_many_arguments)]
fn split_hunk(
    sh: &Shell,
    message: Option<String>,
    revision: &str,
    file_filter: Option<String>,
    lines: Option<String>,
    hunks: Option<String>,
    pattern: Option<String>,
    preview: bool,
    invert: bool,
    dry_run: bool,
) -> Result<()> {
    // preview mode
    if preview {
        return preview_hunks(sh, revision, file_filter.as_deref());
    }

    // require message unless preview
    let message = match message {
        Some(m) => m,
        None => bail!("--message is required (use --preview to see hunks first)"),
    };

    // get diff
    let diff_output = if let Some(ref file) = file_filter {
        cmd!(sh, "jj diff -r {revision} --git {file}")
            .read()
            .wrap_err("failed to get diff")?
    } else {
        cmd!(sh, "jj diff -r {revision} --git")
            .read()
            .wrap_err("failed to get diff")?
    };

    if diff_output.trim().is_empty() {
        bail!("no changes in revision {revision}");
    }

    let files = parse_diff_output(&diff_output);

    // parse selection criteria
    let hunk_indices = hunks.as_ref().map(|h| parse_hunk_indices(h)).transpose()?;
    let line_ranges = lines.as_ref().map(|l| parse_line_ranges(l)).transpose()?;
    let pattern_re = pattern
        .as_ref()
        .map(|p| Regex::new(p).wrap_err("invalid pattern regex"))
        .transpose()?;

    // select hunks
    let selected = select_hunks(
        &files,
        hunk_indices.as_deref(),
        line_ranges.as_deref(),
        pattern_re.as_ref(),
        invert,
    );

    if selected.is_empty() {
        bail!("no hunks matched the selection criteria");
    }

    // group selected hunks by file
    let mut hunks_by_file: HashMap<usize, Vec<usize>> = HashMap::new();
    for (file_idx, hunk_idx) in &selected {
        hunks_by_file.entry(*file_idx).or_default().push(*hunk_idx);
    }

    // show what will be committed
    println!("{}", "Selected hunks:".dimmed());
    for (file_idx, hunk_indices) in &hunks_by_file {
        let file = &files[*file_idx];
        println!("  {} (hunks: {:?})", file.path.cyan(), hunk_indices);
    }

    if dry_run {
        println!("\n{}", "--dry-run: no changes made".yellow());
        return Ok(());
    }

    // create new commit from parent of revision
    let parent_rev = format!("{revision}-");

    // create new empty commit from parent
    println!("{}", "Creating new commit...".dimmed());
    cmd!(sh, "jj new {parent_rev} -m {message}")
        .run()
        .wrap_err("failed to create new commit")?;

    // for each file with selected hunks, apply them
    for (file_idx, hunk_indices) in &hunks_by_file {
        let file = &files[*file_idx];

        if file.is_binary {
            println!(
                "{} skipping binary file: {}",
                "Warning:".yellow(),
                file.path
            );
            continue;
        }

        // get parent content
        let file_path = &file.path;
        let parent_content = if file.is_new {
            String::new()
        } else {
            cmd!(sh, "jj file show -r {parent_rev} {file_path}")
                .read()
                .unwrap_or_default()
        };

        // get selected hunks for this file
        let selected_hunks: Vec<&DiffHunk> = hunk_indices
            .iter()
            .filter_map(|&idx| file.hunks.get(idx))
            .collect();

        // apply hunks to get new content
        let new_content = apply_selected_hunks(&parent_content, &selected_hunks);

        // write the new content
        if file.is_deleted && selected_hunks.len() == file.hunks.len() {
            // all hunks selected for a deleted file = delete the file
            // jj automatically handles this when we write empty content
        }

        // write to the file in the working copy
        std::fs::write(&file.path, &new_content)
            .wrap_err_with(|| format!("failed to write {}", file.path))?;
    }

    // squash the new commit into its working copy changes
    println!("{}", "Squashing changes...".dimmed());
    cmd!(sh, "jj squash")
        .run()
        .wrap_err("failed to squash changes")?;

    // now edit the original revision to restore remaining changes
    println!("{}", "Restoring remaining changes...".dimmed());
    cmd!(sh, "jj edit {revision}")
        .run()
        .wrap_err("failed to edit original revision")?;

    // for each file, restore the original content that wasn't selected
    for (file_idx, selected_hunk_indices) in &hunks_by_file {
        let file = &files[*file_idx];

        if file.is_binary {
            continue;
        }

        let file_path = &file.path;

        // get the full new content (all hunks applied)
        let _full_new_content = if file.is_new {
            cmd!(sh, "jj file show -r {revision} {file_path}")
                .read()
                .unwrap_or_default()
        } else if file.is_deleted {
            String::new()
        } else {
            cmd!(sh, "jj file show -r {revision} {file_path}")
                .read()
                .unwrap_or_default()
        };

        // get parent content
        let parent_content = if file.is_new {
            String::new()
        } else {
            cmd!(sh, "jj file show -r {parent_rev} {file_path}")
                .read()
                .unwrap_or_default()
        };

        // get remaining (non-selected) hunks
        let remaining_hunks: Vec<&DiffHunk> = file
            .hunks
            .iter()
            .enumerate()
            .filter(|(idx, _)| !selected_hunk_indices.contains(idx))
            .map(|(_, h)| h)
            .collect();

        if remaining_hunks.is_empty() {
            // all hunks were selected, restore to parent state
            if file.is_new {
                // new file fully selected = remove from original
                let _ = std::fs::remove_file(&file.path);
            } else {
                std::fs::write(&file.path, &parent_content)
                    .wrap_err_with(|| format!("failed to restore {}", file.path))?;
            }
        } else {
            // apply only remaining hunks
            let remaining_content = apply_selected_hunks(&parent_content, &remaining_hunks);
            std::fs::write(&file.path, &remaining_content)
                .wrap_err_with(|| format!("failed to write remaining changes to {}", file.path))?;
        }
    }

    // squash the restored changes
    cmd!(sh, "jj squash")
        .run()
        .wrap_err("failed to squash remaining changes")?;

    println!("{}", "Split complete".green());
    Ok(())
}
