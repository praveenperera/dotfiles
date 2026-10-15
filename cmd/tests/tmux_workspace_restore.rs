use std::fs;
use std::path::Path;
use std::process::{Command, Output};

// resurrect's post-restore hook runs this on every fresh tmux server, so it must succeed
// quietly when there is no managed workspace to rebuild
fn restore(state_home: &Path, tmux_dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cmd"))
        .args(["tmux", "workspace", "restore"])
        .env("XDG_STATE_HOME", state_home)
        .env("TMUX_TMPDIR", tmux_dir)
        .env_remove("TMUX")
        .output()
        .unwrap()
}

#[test]
fn restore_without_saved_workspace_is_a_noop() {
    let state_home = tempfile::tempdir().unwrap();
    let tmux_dir = tempfile::tempdir().unwrap();

    let output = restore(state_home.path(), tmux_dir.path());

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert!(!state_home
        .path()
        .join("cmd/fleet-tmux/workspace.json")
        .exists());
}

#[test]
fn restore_with_missing_workspace_session_is_a_noop() {
    let state_home = tempfile::tempdir().unwrap();
    let tmux_dir = tempfile::tempdir().unwrap();
    let state_dir = state_home.path().join("cmd/fleet-tmux");
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(state_dir.join("workspace-id"), "fleet").unwrap();

    let output = restore(state_home.path(), tmux_dir.path());

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert!(!state_dir.join("workspace.json").exists());
}
