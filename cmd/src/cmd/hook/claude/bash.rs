//! Checks on the Bash tool: CI waits held in the turn, background commands an
//! agent will wait on, the timeout of a background `homebased wait`, and the
//! reminder to register one

use clap::Parser;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::shell::{Command, Script};
use super::text::{has_marker, mentions_homebased_wait, starts_line_with_homebased_wait};
use super::{recommend, Decision, HookInput, Host};
use crate::cmd::main_cmd::{Cmd, MainCmd};
use crate::cmd::ship::ShipCmd;

/// The Bash tool's input
#[derive(Debug, Deserialize)]
struct BashInput {
    command: String,
    #[serde(default, deserialize_with = "only_true")]
    run_in_background: bool,
    /// milliseconds; anything but a whole number reads as unset
    #[serde(default, deserialize_with = "whole_number")]
    timeout: Option<u64>,
}

/// Reads `true` as true and any other value as false, as the jq check did
fn only_true<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    Ok(Value::deserialize(deserializer)? == Value::Bool(true))
}

/// Reads a JSON whole number, and any other value as unset, so a malformed
/// timeout never blocks a call
fn whole_number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    // a model sometimes quotes the number
    Ok(match Value::deserialize(deserializer)? {
        Value::String(text) => text.trim().parse().ok(),
        value => value.as_u64(),
    })
}

/// The Bash tool's result, of which only the background task id matters here
#[derive(Debug, Deserialize)]
struct BashResponse {
    #[serde(rename = "backgroundTaskId")]
    background_task_id: Option<String>,
}

pub fn background_reason() -> String {
    format!(
        "For a command you will wait on, such as a build, test suite, download, or data pull, submit it as a homebased task and run `{}` as a background Bash command with `timeout: {}`, then pass its task id to `t3_background_wait`, or run it as foreground Bash if it takes under about 10 minutes. For a process you will not wait on, such as a dev server, add a `# long-lived: <why>` comment to the command.",
        recommend::WAIT,
        recommend::WAIT_TIMEOUT_MS,
    )
}

pub fn wait_timeout_reason() -> String {
    format!(
        "Give a background `{}` `timeout: {}`, Claude Code's maximum. The wait picks its own limit: with no event it exits after 55 minutes, inside the hour-long prompt cache, and prints the command that starts it again. A shorter Bash timeout kills it before then, and leaving it out gives the 30-minute default. For work expected to take over 4 hours, add `--long` to the wait, which compacts this thread once the turn ends and waits up to 115 minutes.",
        recommend::WAIT,
        recommend::WAIT_TIMEOUT_MS,
    )
}

/// T3 Code files a background Bash under the `command` kind, which holds a
/// thread in Waiting only once it is registered with `t3_background_wait`, and a
/// long command run that way still dies with the 2-hour Bash timeout. Long work
/// goes to homebased, and the one background command allowed by default is the
/// `homebased wait --next` that waits on it. Other background Bash, for
/// processes nobody waits on such as dev servers, needs an opt-in
pub fn background_wait(input: &HookInput) -> Decision {
    let Some(bash) = input.tool_input::<BashInput>() else {
        return Decision::Allow;
    };

    if !bash.run_in_background
        || starts_line_with_homebased_wait(&bash.command)
        || has_marker(&bash.command, "long-lived:")
    {
        return Decision::Allow;
    }

    Decision::Deny(background_reason())
}

/// `homebased wait --next` exits on its own limit, chosen so each wake-up of
/// a T3 Claude session lands inside its hour-long prompt cache, or later after
/// `--long` compacted the thread. A Bash timeout below that, including Claude
/// Code's 30-minute default, kills the wait first and leaves the agent to
/// guess how to start it again
pub fn wait_timeout(input: &HookInput) -> Decision {
    let Some(bash) = input.tool_input::<BashInput>() else {
        return Decision::Allow;
    };

    let too_short = bash
        .timeout
        .is_none_or(|timeout| timeout < recommend::WAIT_TIMEOUT_MS);

    if bash.run_in_background && too_short && starts_line_with_homebased_wait(&bash.command) {
        return Decision::Deny(wait_timeout_reason());
    }

    Decision::Allow
}

pub fn ci_wait_reason() -> String {
    format!(
        "Do not wait on CI or a release run inside the turn. Submit the wait as a homebased task and end the turn: `{} [--greptile]` for a PR, or `{}` for a workflow run such as a release. In a T3 thread, then run `{}` as a background Bash command with `timeout: 7200000` and pass its task id to `t3_background_wait`. Both print one line when done; read it with `{}`. For a one-shot status check, drop the watch or sleep.",
        recommend::WATCH_PR_TASK,
        recommend::WATCH_RUN_TASK,
        recommend::WAIT,
        recommend::TASK_LOG,
    )
}

/// A CI or release wait run in the turn, such as `gh run watch` or a sleep loop
/// around `gh pr checks`, holds the thread for as long as CI takes, and agents
/// chain 10-minute foreground chunks to get around the Bash timeout. CI time is
/// unknown up front, so unlike builds there is no foreground carve-out
///
/// Only commands that run count, so `grep`, `echo`, `git commit -m`, or a
/// heredoc body that names a wait is fine
pub fn ci_wait(input: &HookInput) -> Decision {
    let Some(bash) = input.tool_input::<BashInput>() else {
        return Decision::Allow;
    };

    let script = Script::parse(&bash.command);

    // a wait handed to homebased is one of its arguments, never a command that
    // runs here, so a submission needs no exemption and cannot cover another
    // command on the same line
    let polls = script.runs(|command| command.program() == "sleep") && script.runs(calls_gh_ci);
    if script.runs(watches_ci) || polls {
        return Decision::Deny(ci_wait_reason());
    }

    Decision::Allow
}

/// A command that blocks until CI or a run ends
fn watches_ci(command: &Command) -> bool {
    match command.program() {
        "gh" => {
            command.starts_with(&["run", "watch"])
                || (command.starts_with(&["pr", "checks"])
                    && command.args().any(|arg| arg.starts_with("--watch")))
        }
        // parsed with cmd's own parser, so aliases such as `cmd sh wp 5` match too
        "cmd" => {
            let words = std::iter::once("cmd").chain(command.args());
            matches!(
                Cmd::try_parse_from(words),
                Ok(Cmd {
                    subcommand: MainCmd::Ship {
                        subcommand: ShipCmd::WatchPr(_) | ShipCmd::WatchRun(_)
                    }
                })
            )
        }
        _ => false,
    }
}

/// A `gh` call that reads CI state, which a `sleep` beside it turns into a poll
///
/// Other `gh` calls in a sleep loop, such as waiting out the API rate limit,
/// an async merge, or a push retry, wait on GitHub but not on CI
fn calls_gh_ci(command: &Command) -> bool {
    if command.program() != "gh" {
        return false;
    }

    let names_ci = || {
        command
            .args()
            .any(|arg| CI_API_PATHS.iter().any(|path| arg.contains(path)))
    };
    let mut args = command.args();
    match (args.next(), args.next()) {
        (Some("run"), Some("view" | "list")) | (Some("pr"), Some("checks")) => true,
        (Some("pr"), Some("view")) | (Some("api"), _) => names_ci(),
        _ => false,
    }
}

/// Parts of a `gh api` path or `gh pr view` field that read CI status
const CI_API_PATHS: &[&str] = &[
    "check-runs",
    "check-suites",
    "actions/runs",
    "actions/jobs",
    "/status",
    "statusCheckRollup",
];

/// T3 lists a thread under Waiting only after the background command that runs
/// `homebased wait` is registered with `t3_background_wait`; the command alone
/// leaves it running. Agents kept skipping that step, and a hook cannot call an
/// MCP tool, so this reminds the agent with the task id right after it starts
pub fn register_background_wait(input: &HookInput, host: &impl Host) -> Decision {
    let Some(bash) = input.tool_input::<BashInput>() else {
        return Decision::Allow;
    };

    // checked before the T3 check, which walks the process tree, since this runs
    // after every Bash call
    if !mentions_homebased_wait(&bash.command) {
        return Decision::Allow;
    }

    let task_id = BashResponse::deserialize(&input.tool_response)
        .ok()
        .and_then(|response| response.background_task_id)
        .filter(|id| !id.is_empty());

    match task_id {
        Some(id) if host.in_t3() => Decision::Context(format!(
            "Call `t3_background_wait` with taskIds [{}] now, before ending the turn, so T3 lists the thread under Waiting.",
            serde_json::to_string(&id).unwrap_or_default()
        )),
        _ => Decision::Allow,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::recommend;
    use super::super::tests::{decision, pre, Fixture};
    use super::super::{Decision, Event};
    use super::{background_reason, ci_wait_reason, wait_timeout_reason};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Want {
        Allow,
        Ci,
        Background,
        Both,
    }

    fn want(want: Want) -> Decision {
        match want {
            Want::Allow => Decision::Allow,
            Want::Ci => Decision::Deny(ci_wait_reason()),
            Want::Background => Decision::Deny(background_reason()),
            Want::Both => {
                Decision::Deny(format!("{}\n\n{}", background_reason(), ci_wait_reason()))
            }
        }
    }

    fn check(rows: &[(&str, Want)], background: bool) {
        let host = Fixture::new(true);
        for &(command, expected) in rows {
            let stdin = pre(
                "Bash",
                // these rows are about what may run in the background, not the
                // wait's timeout, which has its own test
                json!({"command": command, "run_in_background": background, "timeout": recommend::WAIT_TIMEOUT_MS}),
            );
            assert_eq!(
                decision(Event::PreToolUse, &stdin, &host),
                want(expected),
                "{command:?}"
            );
        }
    }

    #[test]
    fn ci_waits_are_denied() {
        use Want::{Allow, Ci};

        check(
            &[
                ("cmd ship watch-pr 5", Ci),
                ("GH_TOKEN=x cmd ship watch-run release-cmd HEAD", Ci),
                ("bash -c 'cmd ship watch-run release-cmd HEAD'", Ci),
                ("sh -lc \"cmd ship watch-pr 5\"", Ci),
                ("~/.local/bin/cmd ship watch-pr 978", Ci),
                ("cmd sh wp 5", Ci),
                ("cmd ship wr release-cmd HEAD --repo o/r", Ci),
                (
                    "cmd sh watch-pr https://github.com/o/r/pull/5 --greptile",
                    Ci,
                ),
                ("while true; do gh pr checks 5; sleep 30; done", Ci),
                ("gh run watch 123", Ci),
                ("gh pr checks 5 --watch", Ci),
                ("gh pr checks 5 --watch --fail-fast", Ci),
                ("timeout 600 gh run watch 123", Ci),
                ("cd repo && gh run watch 1", Ci),
                ("true || gh run watch 1", Ci),
                ("gh run watch 1 | tail -1", Ci),
                ("cargo build 2>&1 && gh run watch 1", Ci),
                ("if true; then gh run watch 1; fi", Ci),
                ("if false; then :; else gh run watch 1; fi", Ci),
                ("sleep 60; gh api repos/o/r/actions/runs/1", Ci),
                ("for i in 1 2 3; do sleep 30 && gh run view 5; done", Ci),
                (
                    "bash -lc 'sleep 30; gh pr view 5 --json statusCheckRollup'",
                    Ci,
                ),
                ("sleep 30; gh api repos/o/r/commits/abc/check-runs", Ci),
                ("(gh run watch 1)", Ci),
                ("echo `gh run watch 1`", Ci),
                ("echo \"$(gh run watch 1)\"", Ci),
                ("zsh -c \"gh run watch 1\"", Ci),
                ("cat <<EOF\nnotes\nEOF\ngh run watch 1", Ci),
                ("cmd ship watch-pr 5 \\\n  --greptile", Ci),
                // positions the old regex missed
                ("/opt/homebrew/bin/gh run watch 1", Ci),
                ("timeout -s KILL 600 gh run watch 1", Ci),
                ("{ gh run watch 1; }", Ci),
                ("bash -ec 'gh run watch 1'", Ci),
                ("x=1; until gh pr checks 5; do sleep 30; done", Ci),
                ("/usr/bin/timeout 60 gh run watch 1", Ci),
                ("env GH_TOKEN=x gh run watch 1", Ci),
                ("env -u GH_TOKEN gh run watch 1", Ci),
                ("nohup gh run watch 1 &", Ci),
                // a homebased submission covers only itself
                ("homebased task run -- x && gh run watch 1", Ci),
                ("homebased task run -- true; gh run watch 1", Ci),
                // a `<<` terminator must be the whole line, so the wait is body text
                ("cat <<EOF\nbody\n EOF\ngh run watch 1\nEOF", Allow),
                // `<<-` strips leading tabs from the terminator
                ("cat <<-EOF\n\tbody\n\tEOF\ngh run watch 1", Ci),
            ],
            false,
        );
    }

    #[test]
    fn mentions_and_opt_ins_are_allowed() {
        use Want::Allow;

        check(
            &[
                (
                    "homebased --json task run --name x --timeout 12h -- cmd ship watch-pr 5",
                    Allow,
                ),
                ("homebased task submit --name x -- gh run watch 1", Allow),
                ("grep -rn \"cmd ship watch-pr\" .", Allow),
                ("echo 'cmd ship watch-pr 5' > notes.txt", Allow),
                (
                    "git commit -m \"Port watch scripts to cmd ship watch-pr\"",
                    Allow,
                ),
                ("FOO=bar cargo test ship", Allow),
                ("git rm a/watch_pr.sh b/watch_run.sh", Allow),
                ("gh pr checks 5", Allow),
                ("gh run view 123 --log-failed", Allow),
                ("sleep 5 && cargo test", Allow),
                ("cmd ship plan", Allow),
                ("cmd sh --help", Allow),
                ("cmd ship watch-pr --help", Allow),
                ("cmd fl dfu", Allow),
                // a sleep around a `gh` call that does not read CI, from real
                // calls the old rule denied
                ("sleep 100; date; gh api rate_limit --jq .resources.core", Allow),
                (
                    "for i in 1 2 3 4 5 6; do s=$(gh pr view 962 --json state -q .state); [ \"$s\" = MERGED ] && break; sleep 5; done",
                    Allow,
                ),
                (
                    "for i in 1 2 3; do if git push -u origin x; then break; else sleep 15; fi; done; gh pr create --draft",
                    Allow,
                ),
                (
                    "cat <<'EOF' > notes.md\ngh run watch 1\nsleep 30\nEOF",
                    Allow,
                ),
                ("cat <<-EOF > notes.md\n\tgh run watch 1\n\tEOF", Allow),
                (
                    "git commit -F - <<EOF\nWait with sleep 30 and gh pr checks\nEOF",
                    Allow,
                ),
                ("git log --grep \"gh run watch\"", Allow),
                ("gh pr checks 5 | grep -- --watch", Allow),
                ("echo ok # gh run watch 1", Allow),
                ("rg 'sleep 30' && gh pr view 5", Allow),
                ("cat <<<\"gh run watch 1\"", Allow),
                ("bash scripts/release.sh", Allow),
                ("echo 'x' | bash -c 'echo gh run watch'", Allow),
                // false positives of the old regex
                ("git add watch.sh sleep.txt && gh pr view 5", Allow),
                ("git add a.sh sleep && gh pr view 5", Allow),
                ("git commit -m \"then gh run watch is gone\"", Allow),
                ("git commit -m \"timeout 5 gh run watch\"", Allow),
                ("echo \"do sleep 1\"; gh pr view 5", Allow),
                ("printf '%s\\n' \"a; sleep 5\" && gh pr view 5", Allow),
                // a wait on another machine does not hold this turn's shell
                ("ssh host gh run watch 1", Allow),
            ],
            false,
        );
    }

    #[test]
    fn background_bash_needs_homebased_wait_or_opt_in() {
        use Want::{Allow, Background, Both};

        check(
            &[
                ("cargo build --release", Background),
                ("homebased wait --next abc123", Allow),
                ("  homebased   wait --next a b", Allow),
                ("cd x\nhomebased wait --next a", Allow),
                ("npm run dev # long-lived: dev server", Allow),
                ("pnpm dev #long-lived:x", Allow),
                // only a line that starts with the wait counts
                ("cd x && homebased wait --next a", Background),
                ("homebased wait", Background),
                ("gh run watch 1", Both),
                (
                    "homebased --json task run -- cmd ship watch-pr 5",
                    Background,
                ),
            ],
            true,
        );

        check(&[("cargo build --release", Allow)], false);

        // only a JSON `true` runs in the background, as with jq before
        let host = Fixture::new(true);
        let stdin = pre(
            "Bash",
            json!({"command": "cargo build", "run_in_background": "true"}),
        );
        assert_eq!(decision(Event::PreToolUse, &stdin, &host), Decision::Allow);
    }

    #[test]
    fn background_homebased_wait_leaves_its_limit_to_homebased() {
        let host = Fixture::new(true);
        let max = recommend::WAIT_TIMEOUT_MS;
        let too_short = Decision::Deny(wait_timeout_reason());

        let rows = [
            (
                json!({"command": "homebased wait --next a", "run_in_background": true, "timeout": max}),
                Decision::Allow,
            ),
            (
                json!({"command": "homebased wait --next --long a b", "run_in_background": true, "timeout": max}),
                Decision::Allow,
            ),
            (
                json!({"command": "homebased wait --next a", "run_in_background": true, "timeout": "7200000"}),
                Decision::Allow,
            ),
            (
                json!({"command": "homebased wait --next a", "run_in_background": true, "timeout": 3_300_000}),
                too_short.clone(),
            ),
            // Claude Code's 30-minute default would kill the wait first
            (
                json!({"command": "homebased wait --next a", "run_in_background": true}),
                too_short,
            ),
            // a foreground call is capped at 10 minutes by Claude Code itself
            (
                json!({"command": "homebased wait --next a", "timeout": 600_000}),
                Decision::Allow,
            ),
            (
                json!({"command": "npm run dev # long-lived: dev server", "run_in_background": true, "timeout": 600_000}),
                Decision::Allow,
            ),
        ];

        for (input, expected) in rows {
            let stdin = pre("Bash", input.clone());
            assert_eq!(
                decision(Event::PreToolUse, &stdin, &host),
                expected,
                "{input}"
            );
        }
    }

    #[test]
    fn background_homebased_wait_is_registered_in_t3() {
        let post = |command: &str, response: serde_json::Value| {
            json!({
                "hook_event_name": "PostToolUse",
                "tool_name": "Bash",
                "tool_input": {"command": command, "run_in_background": true},
                "tool_response": response,
            })
            .to_string()
        };
        let reminder = |id: &str| {
            Decision::Context(format!(
                "Call `t3_background_wait` with taskIds [\"{id}\"] now, before ending the turn, so T3 lists the thread under Waiting."
            ))
        };

        let t3 = Fixture::new(true);
        let plain = Fixture::new(false);
        let id =
            json!({"stdout": "", "stderr": "", "interrupted": false, "backgroundTaskId": "bx1"});

        let rows = [
            (
                &t3,
                post("homebased wait --next a", id.clone()),
                reminder("bx1"),
            ),
            (
                &t3,
                post("cd x && homebased wait --next a", id.clone()),
                reminder("bx1"),
            ),
            (
                &t3,
                post("homebased wait --next a", json!({"stdout": "done"})),
                Decision::Allow,
            ),
            (
                &t3,
                post("homebased wait --next a", json!({"backgroundTaskId": ""})),
                Decision::Allow,
            ),
            (
                &t3,
                post("homebased wait --next a", json!("text")),
                Decision::Allow,
            ),
            (&t3, post("cargo build", id.clone()), Decision::Allow),
            (
                &plain,
                post("homebased wait --next a", id.clone()),
                Decision::Allow,
            ),
            (
                &t3,
                post(
                    "homebased wait --next a",
                    json!({"backgroundTaskId": "a\"b"}),
                ),
                reminder("a\\\"b"),
            ),
        ];

        for (host, stdin, expected) in rows {
            assert_eq!(
                decision(Event::PostToolUse, &stdin, host),
                expected,
                "{stdin}"
            );
        }
    }
}
