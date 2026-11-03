//! Every stdout line of a Monitor wakes the agent for a full turn that re-reads
//! the whole context, so waits that stream progress, per-job updates, or redrawn
//! `gh run watch` output cost hundreds of thousands of tokens per line. Waits on
//! homebased work go to a background `homebased wait --next` instead, because a
//! Monitor also expires after 30 minutes and wakes the agent each time, so only
//! streams the agent opts into are allowed

use serde::Deserialize;

use super::text::{has_marker, mentions_homebased_wait, redirects_stderr_first};
use super::{recommend, Decision, HookInput};

/// The Monitor tool's input; a WebSocket source has no `command`
#[derive(Debug, Deserialize)]
struct MonitorInput {
    command: Option<String>,
}

pub const REDIRECT_REASON: &str = "This command redirects in the wrong order: `2>&1 > file` sends stderr to the Monitor. Write `> file 2>&1`.";

pub fn homebased_wait_reason() -> String {
    format!(
        "A Monitor expires after 30 minutes and wakes you each time with nothing to report. Run `{}` as a background Bash command with `timeout: 7200000` instead, pass its task id to `t3_background_wait`, and end the turn.",
        recommend::WAIT_MANY
    )
}

pub fn wait_reason() -> String {
    format!(
        "A Monitor wakes you for a full turn on every stdout line, so a wait must print once, when the job ends. Submit the command as a homebased task and run `{}` as a background Bash command with `timeout: 7200000`, or run it as foreground Bash if it takes under about 10 minutes. To stream on purpose, for example each new error in a log, add a `# stream: <why>` comment to the command and filter it to the lines you would act on.",
        recommend::WAIT
    )
}

pub fn quiet(input: &HookInput) -> Decision {
    let Some(command) = input
        .tool_input::<MonitorInput>()
        .and_then(|monitor| monitor.command)
        .filter(|command| !command.is_empty())
    else {
        return Decision::Allow;
    };

    // `2>&1 > file` sends stderr, where cargo and most build tools write
    // progress, to the Monitor
    if redirects_stderr_first(&command) {
        return Decision::Deny(REDIRECT_REASON.to_owned());
    }

    if mentions_homebased_wait(&command) {
        return Decision::Deny(homebased_wait_reason());
    }

    // an explicit opt-in for a stream the agent wants, such as each new error in a log
    if has_marker(&command, "stream:") {
        return Decision::Allow;
    }

    Decision::Deny(wait_reason())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::tests::{decision, pre, Fixture};
    use super::super::{Decision, Event};
    use super::{homebased_wait_reason, wait_reason, REDIRECT_REASON};

    #[test]
    fn monitors_must_opt_into_streams() {
        let redirect = Decision::Deny(REDIRECT_REASON.to_owned());
        let homebased = Decision::Deny(homebased_wait_reason());
        let wait = Decision::Deny(wait_reason());

        let rows = [
            (
                json!({"description": "ws", "ws": {"url": "wss://x"}}),
                Decision::Allow,
            ),
            (json!({"command": ""}), Decision::Allow),
            (
                json!({"command": "tail -f log 2>&1 > out"}),
                redirect.clone(),
            ),
            (
                json!({"command": "tail -f log 2>&1 > f # stream: x"}),
                redirect.clone(),
            ),
            (
                json!({"command": "make 2>&1 >/dev/null # stream: x"}),
                redirect,
            ),
            (
                json!({"command": "homebased wait --next a"}),
                homebased.clone(),
            ),
            (json!({"command": "homebased wait"}), homebased.clone()),
            (
                json!({"command": "homebased wait --next a # stream: x"}),
                homebased,
            ),
            (json!({"command": "cargo build > out 2>&1"}), wait.clone()),
            (json!({"command": "make 2>&1 >&2"}), wait.clone()),
            (json!({"command": "gh run watch 1"}), wait),
            (
                json!({"command": "tail -f app.log | grep --line-buffered ERROR # stream: each error"}),
                Decision::Allow,
            ),
        ];

        let host = Fixture::new(true);
        for (tool_input, expected) in rows {
            let stdin = pre("Monitor", tool_input.clone());
            assert_eq!(
                decision(Event::PreToolUse, &stdin, &host),
                expected,
                "{tool_input}"
            );
        }
    }
}
