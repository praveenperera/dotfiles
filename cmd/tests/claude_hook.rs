use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

// Claude Code reads a hook's stdout as JSON on exit 0 and treats exit 2 as a
// block, so every input must exit 0 and print either nothing or one JSON object
fn hook(event: &str, stdin: &str) -> Output {
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_cmd"))
        .args(["hook", "claude", event])
        .env("HOME", home.path())
        .env("CLAUDE_PROJECT_DIR", home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn hook_prints_one_decision_or_nothing() {
    let deny_ci = r#"{"tool_name": "Bash", "tool_input": {"command": "gh run watch 1"}}"#;
    let deny_model = r#"{"tool_name": "Agent", "tool_input": {"prompt": "x"}}"#;
    let allowed = r#"{"tool_name": "Bash", "tool_input": {"command": "cargo test"}}"#;

    let cases = [
        ("pre-tool-use", deny_ci, Some("Do not wait on CI")),
        ("pre-tool-use", deny_model, Some("Name the model")),
        ("pre-tool-use", allowed, None),
        ("pre-tool-use", "not json", None),
        ("pre-tool-use", "", None),
        ("post-tool-use", allowed, None),
        // the fixture home has no t3.md, so this allows in and out of T3
        ("session-start", r#"{"source": "startup"}"#, None),
    ];

    for (event, stdin, deny) in cases {
        let output = hook(event, stdin);
        assert!(output.status.success(), "{event} {stdin}: {output:?}");

        let stdout = String::from_utf8(output.stdout).unwrap();
        match deny {
            None => assert!(stdout.is_empty(), "{event} {stdin}: {stdout}"),
            Some(reason) => {
                let json: Value = serde_json::from_str(&stdout).unwrap();
                let specific = &json["hookSpecificOutput"];
                assert_eq!(specific["hookEventName"], "PreToolUse");
                assert_eq!(specific["permissionDecision"], "deny");
                let text = specific["permissionDecisionReason"].as_str().unwrap();
                assert!(text.starts_with(reason), "{text}");
            }
        }
    }
}

#[test]
fn answers_from_a_deleted_directory() {
    // a session left in a removed worktree has no working directory, where
    // xshell refuses to start, so these two must not need a shell
    let home = tempfile::tempdir().unwrap();
    let gone = home.path().join("gone");
    let deny_ci = r#"{"tool_name": "Bash", "tool_input": {"command": "gh run watch 1"}}"#;

    for (args, stdin, want) in [
        ("statusline", r#"{"cwd": "/"}"#, "/"),
        ("hook claude pre-tool-use", deny_ci, "\"deny\""),
    ] {
        let mut child = Command::new("sh")
            .args([
                "-c",
                r#"mkdir "$1" && cd "$1" && rmdir "$1" && exec "$2" $3"#,
                "sh",
            ])
            .arg(&gone)
            .arg(env!("CARGO_BIN_EXE_cmd"))
            .arg(args)
            .env("HOME", home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();

        assert!(output.status.success(), "{args}: {output:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains(want), "{args}: {stdout}");
    }
}
