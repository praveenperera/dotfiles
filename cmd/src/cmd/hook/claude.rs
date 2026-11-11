//! Claude Code hooks, wired up in `claude/settings.json` as
//! `cmd hook claude <event>`
//!
//! Each event reads the hook input on stdin and dispatches on `tool_name` to the
//! checks that apply. A check returns a [`Decision`]; the hook prints the JSON
//! Claude Code expects for it, or nothing to allow the call
//!
//! A hook must never block the agent by accident, so input it does not
//! understand, a read error, or a panic all allow the call. Each event runs once
//! per tool call, so this path starts no async runtime and forks nothing except
//! `ps` for the rare checks that need the process tree

mod agent;
mod bash;
mod monitor;
mod shell;
mod text;

use std::io::Read;
use std::path::PathBuf;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The hook events `claude/settings.json` sends here
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Event {
    PreToolUse,
    PostToolUse,
    SessionStart,
}

/// What a check decided about a call
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    /// Block the tool call and show the agent why
    Deny(String),
    /// Let the call through and add this text to the agent's context
    Context(String),
}

impl Decision {
    /// Combines checks that each ran as a separate hook before, so a call two of
    /// them deny shows both reasons
    fn all(decisions: impl IntoIterator<Item = Decision>) -> Decision {
        let reasons = decisions
            .into_iter()
            .filter_map(|decision| match decision {
                Decision::Deny(reason) => Some(reason),
                _ => None,
            })
            .collect::<Vec<_>>();

        if reasons.is_empty() {
            Decision::Allow
        } else {
            Decision::Deny(reasons.join("\n\n"))
        }
    }
}

/// The fields of the hook input these checks read
///
/// Claude Code sends more, such as `session_id`, `cwd`, and `hook_event_name`.
/// Each tool's `tool_input` is read by the check for that tool, so one tool's
/// unexpected shape cannot break another's
#[derive(Debug, Default, Deserialize)]
pub struct HookInput {
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_input: Value,
    #[serde(default)]
    pub tool_response: Value,
}

impl HookInput {
    /// The tool input as `T`, or `None` when it has another shape
    fn tool_input<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        T::deserialize(&self.tool_input).ok()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HookOutput {
    hook_specific_output: HookSpecificOutput,
}

#[derive(Debug, Serialize)]
#[serde(tag = "hookEventName", rename_all_fields = "camelCase")]
enum HookSpecificOutput {
    PreToolUse {
        permission_decision: PermissionDecision,
        permission_decision_reason: String,
    },
    PostToolUse {
        additional_context: String,
    },
    SessionStart {
        additional_context: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum PermissionDecision {
    Deny,
}

/// What the checks may ask of the machine, so tests can run them against fixtures
pub trait Host {
    /// Whether this session runs in a T3 Code thread
    fn in_t3(&self) -> bool;

    /// The home directory, which holds `~/.claude`
    fn home(&self) -> Option<PathBuf>;

    /// The project root, `$CLAUDE_PROJECT_DIR`
    fn project_dir(&self) -> PathBuf;
}

/// The real machine
pub struct System;

impl Host for System {
    fn in_t3(&self) -> bool {
        crate::process::in_t3_session()
    }

    fn home(&self) -> Option<PathBuf> {
        crate::fsutil::home_dir().ok()
    }

    fn project_dir(&self) -> PathBuf {
        std::env::var_os("CLAUDE_PROJECT_DIR").map_or_else(|| PathBuf::from("."), PathBuf::from)
    }
}

/// Commands the deny reasons recommend, kept in one place so a test can check
/// each one passes these hooks and parses as the CLI it names
pub mod recommend {
    pub const WATCH_PR_TASK: &str =
        r#"homebased --json task run --name "CI for #<n>" --timeout 12h -- cmd ship watch-pr <n>"#;
    pub const WATCH_RUN_TASK: &str = r#"homebased --json task run --name "<workflow> run" --timeout 2h -- cmd ship watch-run <workflow> <sha>"#;
    pub const WAIT: &str = "homebased wait --next <id>";
    pub const WAIT_MANY: &str = "homebased wait --next <id> [<id> ...]";
    pub const TASK_LOG: &str = "homebased task log <id> --tail 5";

    /// Bash `timeout` for a background `homebased wait`, 55 minutes. T3 sessions
    /// cache prompts for an hour, so a wait that ends or times out inside that
    /// window wakes the agent on a warm cache; one that runs past it rewrites the
    /// whole context, about $0.90 of Opus input at 150K tokens, while re-arming
    /// the wait costs about $0.13
    pub const WAIT_TIMEOUT_MS: u64 = 3_300_000;

    /// Bash `timeout` for a wait on work expected to take over 4 hours, the most
    /// Claude Code allows. Past about 4 hours, re-arming every 55 minutes costs
    /// more than compacting once, so the agent compacts first ([`COMPACT_SELF`])
    /// and each later wake-up rebuilds only the small compacted context
    pub const LONG_WAIT_TIMEOUT_MS: u64 = 7_200_000;

    /// Compacts the calling T3 thread once its turn ends: run mid-turn, it
    /// queues the compaction behind the running turn. Nothing wakes the thread
    /// after a queued compaction, which suits a long wait, since the wait wakes
    /// it later
    pub const COMPACT_SELF: &str = "homebased --json thread compact";

    /// The comment that opts a background wait into [`LONG_WAIT_TIMEOUT_MS`]
    pub const LONG_WAIT_MARKER: &str = "long-wait:";
}

/// Runs `event` on stdin and prints its output; never fails, so a hook bug
/// allows the call instead of blocking it
pub fn run(event: Event) {
    let output = std::panic::catch_unwind(|| {
        let mut stdin = String::new();
        std::io::stdin().read_to_string(&mut stdin).ok()?;
        respond(event, &stdin, &System)
    });

    if let Ok(Some(output)) = output {
        println!("{output}");
    }
}

/// The JSON to print for `stdin`, or `None` to allow the call
pub fn respond(event: Event, stdin: &str, host: &impl Host) -> Option<String> {
    // SessionStart reads nothing from its input, so a change there cannot break it
    let input = match event {
        Event::SessionStart => HookInput::default(),
        _ => serde_json::from_str(stdin).ok()?,
    };

    let hook_specific_output = match (event, decide(event, &input, host)) {
        (_, Decision::Allow) => return None,
        (Event::PreToolUse, Decision::Deny(reason)) => HookSpecificOutput::PreToolUse {
            permission_decision: PermissionDecision::Deny,
            permission_decision_reason: reason,
        },
        (Event::PostToolUse, Decision::Context(context)) => HookSpecificOutput::PostToolUse {
            additional_context: context,
        },
        (Event::SessionStart, Decision::Context(context)) => HookSpecificOutput::SessionStart {
            additional_context: context,
        },
        // no check returns these pairs, and an event that cannot carry one allows
        (Event::PreToolUse, Decision::Context(_))
        | (Event::PostToolUse | Event::SessionStart, Decision::Deny(_)) => return None,
    };

    serde_json::to_string(&HookOutput {
        hook_specific_output,
    })
    .ok()
}

/// Dispatches to the checks for `event` and the input's tool
pub fn decide(event: Event, input: &HookInput, host: &impl Host) -> Decision {
    let tool = input.tool_name.as_deref().unwrap_or_default();

    match (event, tool) {
        (Event::PreToolUse, "Bash") => Decision::all([
            bash::background_wait(input),
            bash::wait_timeout(input),
            bash::ci_wait(input),
        ]),
        (Event::PreToolUse, "Monitor") => monitor::quiet(input),
        (Event::PreToolUse, "Agent" | "mcp__t3-code__delegate_task") => {
            agent::model_and_effort(input, host)
        }
        (Event::PreToolUse, "mcp__t3-code__watch_pull_request") => no_pr_watch(),
        (Event::PostToolUse, "Bash") => bash::register_background_wait(input, host),
        (Event::SessionStart, _) => t3_context(host),
        _ => Decision::Allow,
    }
}

/// T3's watch_pull_request wakes the thread for every check and bot comment on
/// the PR, such as each Vercel preview, so a CI wait costs a full turn of
/// re-read context per event. The T3 system prompt asks for it whenever a merge
/// waits on checks, so wording alone did not stop agents from reaching for it
fn no_pr_watch() -> Decision {
    Decision::Deny(no_pr_watch_reason())
}

fn no_pr_watch_reason() -> String {
    format!(
        "watch_pull_request wakes this thread for every check and bot comment on the PR. Wait through homebased instead, which wakes it once: `{}` (add `--greptile` to also wait for Greptile's review of the head commit), then run `{}` as a background Bash command with `timeout: {}`, pass its task id to `t3_background_wait`, and end the turn. When it ends, read its one-line result with `{}`. This overrides the T3 instruction to call watch_pull_request; link_pull_request is still fine.",
        recommend::WATCH_PR_TASK,
        recommend::WAIT,
        recommend::WAIT_TIMEOUT_MS,
        recommend::TASK_LOG,
    )
}

/// Loads the T3-only rules in `~/.claude/t3.md` only when T3 runs this session,
/// so CLAUDE.md states one default instead of asking the agent to work out
/// whether it is in T3. SessionStart runs again after a resume or compaction,
/// which keeps the rules in context
fn t3_context(host: &impl Host) -> Decision {
    if !host.in_t3() {
        return Decision::Allow;
    }

    host.home()
        .and_then(|home| std::fs::read_to_string(home.join(".claude/t3.md")).ok())
        .map_or(Decision::Allow, Decision::Context)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use clap::Parser;
    use serde_json::{json, Value};

    use super::{agent, bash, monitor, recommend, respond, Decision, Event, Host};
    use crate::cmd::main_cmd::Cmd;

    /// A machine with fixture agents and rules under a temporary home
    pub(super) struct Fixture {
        pub in_t3: bool,
        pub home: tempfile::TempDir,
        pub project: tempfile::TempDir,
    }

    impl Fixture {
        pub fn new(in_t3: bool) -> Self {
            let fixture = Self {
                in_t3,
                home: tempfile::tempdir().unwrap(),
                project: tempfile::tempdir().unwrap(),
            };

            fixture.write_home(".claude/t3.md", "# T3 rules\n\nline two\n");
            fixture
        }

        pub fn write_home(&self, path: &str, contents: &str) {
            write(&self.home.path().join(path), contents);
        }

        pub fn write_project(&self, path: &str, contents: &str) {
            write(&self.project.path().join(path), contents);
        }
    }

    fn write(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    impl Host for Fixture {
        fn in_t3(&self) -> bool {
            self.in_t3
        }

        fn home(&self) -> Option<PathBuf> {
            Some(self.home.path().to_owned())
        }

        fn project_dir(&self) -> PathBuf {
            self.project.path().to_owned()
        }
    }

    pub(super) fn pre(tool: &str, tool_input: Value) -> String {
        json!({
            "session_id": "s",
            "transcript_path": "/tmp/t.jsonl",
            "cwd": "/tmp",
            "permission_mode": "default",
            "hook_event_name": "PreToolUse",
            "tool_name": tool,
            "tool_input": tool_input,
            "tool_use_id": "toolu_1",
        })
        .to_string()
    }

    /// The decision `respond` printed, read back from its JSON
    pub(super) fn decision(event: Event, stdin: &str, host: &impl Host) -> Decision {
        let Some(output) = respond(event, stdin, host) else {
            return Decision::Allow;
        };

        let output: Value = serde_json::from_str(&output).unwrap();
        let specific = &output["hookSpecificOutput"];
        let event_name = match event {
            Event::PreToolUse => "PreToolUse",
            Event::PostToolUse => "PostToolUse",
            Event::SessionStart => "SessionStart",
        };
        assert_eq!(specific["hookEventName"], event_name, "{output}");

        if specific["permissionDecision"] == "deny" {
            Decision::Deny(
                specific["permissionDecisionReason"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            )
        } else {
            Decision::Context(specific["additionalContext"].as_str().unwrap().to_owned())
        }
    }

    #[test]
    fn unknown_input_allows() {
        let host = Fixture::new(true);
        let cases = [
            "",
            "not json",
            "[]",
            r#"{"tool_name": "Bash"}"#,
            r#"{"tool_name": "Bash", "tool_input": {"command": 5}}"#,
            r#"{"tool_name": "Bash", "tool_input": "gh run watch 1"}"#,
            r#"{"tool_name": "Agent", "tool_input": {"model": 5}}"#,
            r#"{"tool_name": "Monitor", "tool_input": {"command": ["x"]}}"#,
            r#"{"tool_name": "Read", "tool_input": {"file_path": "/x"}}"#,
            r#"{"tool_name": 7}"#,
        ];

        for stdin in cases {
            assert_eq!(
                decision(Event::PreToolUse, stdin, &host),
                Decision::Allow,
                "{stdin}"
            );
            assert_eq!(
                decision(Event::PostToolUse, stdin, &host),
                Decision::Allow,
                "{stdin}"
            );
        }
    }

    #[test]
    fn watch_pull_request_is_always_denied() {
        let host = Fixture::new(true);
        let stdin = pre(
            "mcp__t3-code__watch_pull_request",
            json!({"url": "https://github.com/o/r/pull/1"}),
        );

        assert!(matches!(
            decision(Event::PreToolUse, &stdin, &host),
            Decision::Deny(_)
        ));
    }

    #[test]
    fn session_start_loads_t3_rules_only_in_t3() {
        let stdin = r#"{"hook_event_name": "SessionStart", "source": "startup"}"#;

        let t3 = Fixture::new(true);
        assert_eq!(
            decision(Event::SessionStart, stdin, &t3),
            Decision::Context("# T3 rules\n\nline two\n".to_owned())
        );
        // the input is not read, so even garbage still loads the rules
        assert!(matches!(
            decision(Event::SessionStart, "garbage", &t3),
            Decision::Context(_)
        ));

        let plain = Fixture::new(false);
        assert_eq!(
            decision(Event::SessionStart, stdin, &plain),
            Decision::Allow
        );

        let no_rules = Fixture::new(true);
        std::fs::remove_file(no_rules.home.path().join(".claude/t3.md")).unwrap();
        assert_eq!(
            decision(Event::SessionStart, stdin, &no_rules),
            Decision::Allow
        );
    }

    #[test]
    fn two_denies_show_both_reasons() {
        let host = Fixture::new(true);
        let stdin = pre(
            "Bash",
            json!({"command": "gh run watch 1", "run_in_background": true}),
        );

        let Decision::Deny(reason) = decision(Event::PreToolUse, &stdin, &host) else {
            panic!("expected a deny");
        };
        assert!(
            reason.starts_with("For a command you will wait on"),
            "{reason}"
        );
        assert!(reason.contains("\n\nDo not wait on CI"), "{reason}");
    }

    /// Each command a deny reason recommends must pass these hooks, and the
    /// `cmd` part must parse, so a renamed subcommand or flag fails here
    #[test]
    fn recommended_commands_pass_and_parse() {
        let host = Fixture::new(true);
        let fill = |command: &str| {
            command
                .replace("<n>", "5")
                .replace("<workflow>", "release-cmd")
                .replace("<sha>", "HEAD")
                .replace(" [<id> ...]", " b")
                .replace("<id>", "a")
        };

        for task in [
            recommend::WATCH_PR_TASK.to_owned(),
            format!("{} --greptile", recommend::WATCH_PR_TASK),
            recommend::WATCH_RUN_TASK.to_owned(),
        ] {
            let command = fill(&task);
            let stdin = pre("Bash", json!({"command": command}));
            assert_eq!(
                decision(Event::PreToolUse, &stdin, &host),
                Decision::Allow,
                "{command}"
            );

            let (_, cmd) = command.split_once(" -- ").unwrap();
            let words = cmd.split_whitespace().collect::<Vec<_>>();
            assert_eq!(words[0], "cmd");
            Cmd::try_parse_from(&words).unwrap_or_else(|error| panic!("{cmd}: {error}"));
        }

        for wait in [recommend::WAIT, recommend::WAIT_MANY] {
            let stdin = pre(
                "Bash",
                json!({"command": fill(wait), "run_in_background": true}),
            );
            assert_eq!(
                decision(Event::PreToolUse, &stdin, &host),
                Decision::Allow,
                "{wait}"
            );
        }

        // every recommendation the reasons name is one of these constants
        let reasons = [
            bash::ci_wait_reason(),
            bash::background_reason(),
            bash::wait_timeout_reason(),
            monitor::wait_reason(),
            monitor::homebased_wait_reason(),
            super::no_pr_watch_reason(),
            agent::MODEL_REASON.to_owned(),
        ];
        let known = [
            recommend::WATCH_PR_TASK,
            recommend::WATCH_RUN_TASK,
            recommend::WAIT,
            recommend::WAIT_MANY,
            recommend::TASK_LOG,
            recommend::COMPACT_SELF,
        ];

        for reason in reasons {
            for quoted in reason.split('`').skip(1).step_by(2) {
                if quoted.starts_with("homebased") {
                    let base = quoted.trim_end_matches(" [--greptile]");
                    assert!(known.contains(&base), "{quoted:?} in {reason:?}");
                }
            }
        }
    }
}
