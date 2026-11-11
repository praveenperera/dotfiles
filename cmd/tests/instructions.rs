use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::{tempdir, TempDir};

const FRAGMENTS: &[&str] = &["common.md", "claude.md", "codex.md"];
const GENERATED: &[&str] = &["claude/CLAUDE.md", "agents/AGENTS.md"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn instructions(root: &Path, action: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cmd"))
        .args(["instructions", action, "--root"])
        .arg(root)
        .output()
        .unwrap()
}

/// Copies the checked-in fragments and generated files into a scratch dotfiles root
fn scratch_copy() -> TempDir {
    let dir = tempdir().unwrap();
    let repo = repo_root();
    fs::create_dir_all(dir.path().join("agents/instructions")).unwrap();
    fs::create_dir_all(dir.path().join("claude")).unwrap();

    for fragment in FRAGMENTS {
        let path = format!("agents/instructions/{fragment}");
        fs::copy(repo.join(&path), dir.path().join(&path)).unwrap();
    }

    for generated in GENERATED {
        fs::copy(repo.join(generated), dir.path().join(generated)).unwrap();
    }

    dir
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn checked_in_files_match_a_render_of_the_fragments() {
    let repo = repo_root();
    let check = instructions(&repo, "check");
    assert!(check.status.success(), "{}", stderr(&check));

    // a render into a copy must reproduce the checked-in bytes exactly
    let dir = scratch_copy();
    for generated in GENERATED {
        fs::remove_file(dir.path().join(generated)).unwrap();
    }

    let render = instructions(dir.path(), "render");
    assert!(render.status.success(), "{}", stderr(&render));

    for generated in GENERATED {
        assert_eq!(
            read(dir.path(), generated),
            read(&repo, generated),
            "{generated}"
        );
    }
}

#[test]
fn fragment_edit_fails_check_until_rerendered() {
    let dir = scratch_copy();
    let root = dir.path();

    let common = root.join("agents/instructions/common.md");
    let text = fs::read_to_string(&common).unwrap();
    let edited = text.replace(
        "# Testing\n\n",
        "# Testing\n\n- Shared rule from the test.\n",
    );
    assert_ne!(edited, text, "common.md lost its Testing section");
    fs::write(&common, edited).unwrap();

    let codex = root.join("agents/instructions/codex.md");
    let mut text = fs::read_to_string(&codex).unwrap();
    text.push_str("\n# Codex Only\n\n- Codex rule from the test.\n");
    fs::write(&codex, text).unwrap();

    let check = instructions(root, "check");
    assert!(!check.status.success());
    let message = stderr(&check);
    assert!(message.contains("claude/CLAUDE.md"), "{message}");
    assert!(message.contains("agents/AGENTS.md"), "{message}");
    assert!(message.contains("cmd instructions render"), "{message}");

    let render = instructions(root, "render");
    assert!(render.status.success(), "{}", stderr(&render));

    let check = instructions(root, "check");
    assert!(check.status.success(), "{}", stderr(&check));

    let claude = read(root, "claude/CLAUDE.md");
    let agents = read(root, "agents/AGENTS.md");
    assert!(claude.contains("# Testing\n\n- Shared rule from the test.\n"));
    assert!(agents.contains("# Testing\n\n- Shared rule from the test.\n"));
    assert!(agents.contains("# Codex Only\n\n- Codex rule from the test.\n"));
    assert!(!claude.contains("Codex rule from the test"));
}

#[test]
fn hand_edit_to_a_generated_file_fails_check() {
    let dir = scratch_copy();
    let root = dir.path();

    let claude = root.join("claude/CLAUDE.md");
    let mut text = fs::read_to_string(&claude).unwrap();
    text.push_str("- Edited by hand.\n");
    fs::write(&claude, text).unwrap();

    let check = instructions(root, "check");
    assert!(!check.status.success());
    let message = stderr(&check);
    assert!(message.contains("claude/CLAUDE.md"), "{message}");
    assert!(!message.contains("agents/AGENTS.md"), "{message}");
}

#[test]
fn checkout_with_a_missing_fragment_fails_instead_of_using_another_checkout() {
    let dir = scratch_copy();
    let root = dir.path();
    fs::remove_file(root.join("agents/instructions/common.md")).unwrap();

    // no --root, so the root comes from the current directory
    let check = Command::new(env!("CARGO_BIN_EXE_cmd"))
        .args(["instructions", "check"])
        .current_dir(root.join("claude"))
        .output()
        .unwrap();

    assert!(!check.status.success());
    let message = stderr(&check);
    assert!(message.contains("common.md"), "{message}");
    assert!(
        message.contains(&root.canonicalize().unwrap().display().to_string()),
        "{message}"
    );
}
