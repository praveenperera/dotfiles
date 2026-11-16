use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

// the ship skill reads these lines and exit codes, so they run through the binary
fn config() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../agents/skills/ship/repos.toml")
}

fn plan(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cmd"))
        .args(["ship", "plan", "--config"])
        .arg(config())
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn prints_the_plan_and_how_this_host_runs_it() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));

    // (words as one skill argument, host, lines it must print, text it must not)
    let cases: &[(&str, &str, &[&str], &[&str])] = &[
        (
            "soul high",
            "t3-claude",
            &["plan: sol 6.1 high review → commit → push → deploy → clean\n"],
            &[],
        ),
        // a lone `+` word after `--` is a word, not an option
        (
            "+settle",
            "t3-claude",
            &["plan: commit → push → deploy → clean → settle\n"],
            &["skipped:"],
        ),
        (
            "+settle soul",
            "none",
            &[
                "plan: sol 6.1 xhigh review → commit → push → deploy → clean\n",
                "skipped: settle (not in a T3 thread)\n",
                "host: none\n",
                "sol via: the Codex review command",
                "wait: submit through homebased and end the turn",
            ],
            &["t3_background_wait"],
        ),
        (
            "+settle soul",
            "t3-claude",
            &[
                "host: t3-claude\n",
                "sol via: delegate_task",
                "pass its task id to t3_background_wait",
            ],
            &[],
        ),
        // a Codex thread cannot register a background wait
        (
            "+settle",
            "t3-codex",
            &[
                "plan: commit → push → deploy → clean → settle\n",
                "wait: submit through homebased and end the turn",
            ],
            &["t3_background_wait"],
        ),
    ];

    for &(words, host, want, unwanted) in cases {
        let args = [
            "--repo",
            "praveenperera/homelab",
            "--host",
            host,
            "--",
            words,
        ];
        let output = plan(here, &args);
        let text = stdout(&output);

        assert!(
            output.status.success(),
            "{words} {host}: {}",
            stderr(&output)
        );

        assert!(
            text.starts_with("ship: praveenperera/homelab (repos.toml)\n"),
            "{text}"
        );

        for line in want {
            assert!(
                text.contains(line),
                "{words} {host}: no {line:?} in\n{text}"
            );
        }

        for line in unwanted {
            assert!(!text.contains(line), "{words} {host}: {line:?} in\n{text}");
        }
    }
}

#[test]
fn unresolvable_words_exit_2_with_one_line() {
    let args = [
        "--repo",
        "praveenperera/homelab",
        "--host",
        "none",
        "--",
        "delpoy",
    ];
    let output = plan(Path::new(env!("CARGO_MANIFEST_DIR")), &args);

    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert_eq!(
        stderr(&output),
        "ship: unknown word 'delpoy'; steps are: sol, commit, push, pr, greptile, merge, deploy, publish, clean, settle\n"
    );
}

/// A git checkout of homelab in a temporary directory, without commits
struct Checkout(tempfile::TempDir);

impl Checkout {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            let status = Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .status()
                .unwrap();

            assert!(status.success(), "git {args:?}");
        };

        git(&["init", "-q"]);
        git(&[
            "remote",
            "add",
            "origin",
            "git@github.com:praveenperera/homelab.git",
        ]);

        Self(dir)
    }

    fn plan(&self, args: &[&str]) -> Output {
        plan(self.0.path(), args)
    }

    fn plan_path(&self) -> PathBuf {
        // git reports the root with symlinks resolved, such as /private/var on macOS
        let root = self.0.path().canonicalize().unwrap();
        root.join("_scratch/ship/feature-x/plan.json")
    }

    fn saved(&self) -> Value {
        serde_json::from_str(&fs::read_to_string(self.plan_path()).unwrap()).unwrap()
    }
}

#[test]
fn one_run_prints_and_saves_the_plan_and_done_steps_keep_plan_order() {
    let checkout = Checkout::new();

    let output = checkout.plan(&["--task", "feature/x", "--host", "none", "--"]);
    let text = stdout(&output);

    assert!(
        text.contains("plan: commit → push → deploy → clean\n"),
        "{text}"
    );

    assert!(
        text.ends_with(&format!("saved: {}\n", checkout.plan_path().display())),
        "{text}"
    );

    let saved = checkout.saved();
    assert_eq!(
        saved["steps"],
        serde_json::json!(["commit", "push", "deploy", "clean"])
    );

    assert_eq!(saved["done"], serde_json::json!([]));

    // --json prints the plan without the done list it saves
    let output = checkout.plan(&["--json", "--host", "none", "--"]);
    let json = serde_json::from_slice::<Value>(&output.stdout).unwrap();
    let mut without_done = saved.clone();
    without_done.as_object_mut().unwrap().remove("done");
    assert_eq!(json, without_done);

    let output = checkout.plan(&["--task", "feature/x", "--done", "push", "commit"]);
    assert_eq!(stdout(&output), "done: commit, push\n");
    assert_eq!(
        checkout.saved()["done"],
        serde_json::json!(["commit", "push"])
    );
}

#[test]
fn done_rejects_a_step_outside_the_plan_and_needs_a_task() {
    let checkout = Checkout::new();
    checkout.plan(&["--task", "feature/x", "--host", "none", "--"]);

    let output = checkout.plan(&["--task", "feature/x", "--done", "merge"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stderr(&output), "ship: not in the plan: merge\n");
    assert_eq!(checkout.saved()["done"], serde_json::json!([]));

    let output = checkout.plan(&["--done", "commit"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stderr(&output), "ship: --done needs --task\n");
}

#[test]
fn status_mode_leaves_the_saved_plan_alone() {
    let checkout = Checkout::new();
    checkout.plan(&["--task", "feature/x", "--host", "none", "--"]);
    checkout.plan(&["--task", "feature/x", "--done", "commit"]);

    let output = checkout.plan(&["--task", "feature/x", "--host", "none", "--", "?"]);
    assert!(stdout(&output).contains("mode: status only, change nothing\n"));
    assert!(!stdout(&output).contains("saved:"));
    assert_eq!(checkout.saved()["done"], serde_json::json!(["commit"]));
}

#[test]
fn done_keeps_the_fields_and_order_of_a_plan_saved_by_an_older_version() {
    let checkout = Checkout::new();
    let old = r#"{
  "steps": [
    "commit",
    "push",
    "clean"
  ],
  "repo": "praveenperera/homelab",
  "commands": {}
}
"#;

    let path = checkout.plan_path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, old).unwrap();

    let output = checkout.plan(&["--task", "feature/x", "--done", "clean"]);
    assert_eq!(stdout(&output), "done: clean\n");

    let want = old.replace(
        "\"commands\": {}\n",
        "\"commands\": {},\n  \"done\": [\n    \"clean\"\n  ]\n",
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), want);
}

#[test]
fn cove_plan_shows_its_pre_push_check() {
    let args = ["--repo", "bitcoinppl/cove", "--host", "none", "--"];
    let output = plan(Path::new(env!("CARGO_MANIFEST_DIR")), &args);
    let text = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        text.contains(
            "pre-push: push with `cmd ship push -- <git push args>`, which runs:\n  just pre-push\n"
        ),
        "{text}"
    );

    let args = [
        "--repo",
        "bitcoinppl/cove",
        "--json",
        "--host",
        "none",
        "--",
    ];
    let output = plan(Path::new(env!("CARGO_MANIFEST_DIR")), &args);
    let json = serde_json::from_slice::<Value>(&output.stdout).unwrap();
    assert_eq!(json["pre_push"], serde_json::json!(["just pre-push"]));
}

/// A one-commit checkout of `me/app` whose repos.toml entry checks for an `ok`
/// file at the root, with an empty bare repository to push to
struct PushRig {
    checkout: tempfile::TempDir,
    remote: tempfile::TempDir,
}

impl PushRig {
    fn new() -> Self {
        let checkout = tempfile::tempdir().unwrap();
        let remote = tempfile::tempdir().unwrap();

        git(remote.path(), &["init", "-q", "--bare"]);
        git(checkout.path(), &["init", "-q", "-b", "master"]);
        git(
            checkout.path(),
            &["remote", "add", "origin", "git@github.com:me/app.git"],
        );

        let table = r#"owners = ["me"]

[default]
steps = ["commit", "push", "clean"]

[external]
steps = ["sol", "commit", "push", "pr", "clean"]

[repos."me/app"]
steps = ["commit", "push", "clean"]
pre_push = ["test -f ok"]
"#;
        fs::write(checkout.path().join("repos.toml"), table).unwrap();
        fs::create_dir(checkout.path().join("sub")).unwrap();
        fs::write(checkout.path().join("sub/file"), "x").unwrap();
        git(checkout.path(), &["add", "sub/file"]);
        git(
            checkout.path(),
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "-m",
                "init",
            ],
        );

        Self { checkout, remote }
    }

    /// `cmd ship push` from a subdirectory, pushing HEAD to the bare remote's master
    fn push(&self) -> Output {
        let target = self.remote.path().to_str().unwrap();
        Command::new(env!("CARGO_BIN_EXE_cmd"))
            .args(["ship", "push", "--config"])
            .arg(self.checkout.path().join("repos.toml"))
            .args(["--", "-q", target, "HEAD:refs/heads/master"])
            .current_dir(self.checkout.path().join("sub"))
            .output()
            .unwrap()
    }

    /// The bare remote's master commit, if it has one
    fn pushed(&self) -> Option<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.remote.path())
            .args(["rev-parse", "--verify", "-q", "refs/heads/master"])
            .output()
            .unwrap();

        output
            .status
            .success()
            .then(|| stdout(&output).trim().to_owned())
    }

    fn head(&self) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.checkout.path())
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();

        stdout(&output).trim().to_owned()
    }
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .unwrap();

    assert!(status.success(), "git {args:?}");
}

#[test]
fn a_failing_pre_push_check_stops_the_push() {
    let rig = PushRig::new();

    let output = rig.push();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "pre-push: test -f ok\nship: pre-push check `test -f ok` failed (exit 1); nothing was pushed\n"
    );
    assert_eq!(rig.pushed(), None);

    // the check runs from the checkout root, so the subdirectory finds the file
    fs::write(rig.checkout.path().join("ok"), "").unwrap();
    let output = rig.push();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(rig.pushed(), Some(rig.head()));
}
