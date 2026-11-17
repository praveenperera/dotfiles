use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::tempdir;

fn git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_lines(cwd: &Path, args: &[&str]) -> Vec<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

// git records worktree paths resolved, such as /private/var for macOS's /var
fn canonical_display(path: &Path) -> String {
    fs::canonicalize(path).unwrap().display().to_string()
}

fn cmd_git_clean(cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cmd"))
        .args(["git", "clean"])
        .env("NO_COLOR", "1")
        .current_dir(cwd)
        .output()
        .unwrap()
}

// removing a worktree deletes files, so this pins which ones survive
#[test]
fn removes_merged_worktrees_but_keeps_unsaved_work() {
    let root = tempdir().unwrap();
    let root = root.path();
    let main = root.join("main");

    git(
        root,
        &["init", "-q", "--bare", "-b", "master", "origin.git"],
    );
    git(root, &["clone", "-q", "origin.git", "main"]);
    git(
        &main,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ],
    );
    git(&main, &["push", "-q", "origin", "master"]);

    for branch in [
        "clean",
        "modified",
        "untracked",
        "here",
        "open",
        "merged-no-worktree",
    ] {
        git(&main, &["branch", branch]);
        git(&main, &["push", "-q", "-u", "origin", branch]);

        if branch != "merged-no-worktree" {
            let path = format!("../wt-{branch}");
            git(&main, &["worktree", "add", "-q", &path, branch]);
        }
    }

    // build output is ignored, so it does not count as unsaved work
    fs::write(root.join("wt-clean/.gitignore"), "target/\n").unwrap();
    git(&root.join("wt-clean"), &["add", ".gitignore"]);
    git(
        &root.join("wt-clean"),
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            "ignore",
        ],
    );
    fs::create_dir(root.join("wt-clean/target")).unwrap();
    fs::write(root.join("wt-clean/target/out"), "built").unwrap();

    fs::write(root.join("wt-modified/f"), "staged").unwrap();
    git(&root.join("wt-modified"), &["add", "f"]);
    fs::write(root.join("wt-untracked/u"), "new").unwrap();

    git(
        &main,
        &[
            "push",
            "-q",
            "origin",
            "--delete",
            "clean",
            "modified",
            "untracked",
            "here",
            "merged-no-worktree",
        ],
    );

    let output = cmd_git_clean(&root.join("wt-here"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(!root.join("wt-clean").exists(), "{stdout}");

    // the kept worktrees come last, one aligned line each, in branch order
    let summary: Vec<&str> = stdout
        .lines()
        .skip_while(|line| !line.starts_with("Kept "))
        .collect();
    let wt = |name: &str| canonical_display(&root.join(name));
    let expected = [
        "Kept 3 worktrees on merged branches:".to_owned(),
        format!(
            "  here       you are in it                {}",
            wt("wt-here")
        ),
        format!(
            "  modified   modified or untracked files  {}",
            wt("wt-modified")
        ),
        format!(
            "  untracked  modified or untracked files  {}",
            wt("wt-untracked")
        ),
    ];
    assert_eq!(summary, expected, "{stdout}");

    assert_eq!(
        fs::read_to_string(root.join("wt-modified/f")).unwrap(),
        "staged"
    );
    assert_eq!(
        fs::read_to_string(root.join("wt-untracked/u")).unwrap(),
        "new"
    );

    let branches = git_lines(
        &main,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    );
    assert_eq!(
        branches,
        ["here", "master", "modified", "open", "untracked"]
    );
}
