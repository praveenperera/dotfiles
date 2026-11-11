//! Checks on Agent and T3's delegate_task: a call must name its model, because
//! an unnamed subagent silently inherits the parent's model (usually Opus), and
//! must not run Opus, Sonnet, or Haiku at max effort, which burns plan usage
//! that a subagent rarely turns into correctness. Other models, such as Fable
//! or another provider's, may run at max

use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use super::{Decision, HookInput, Host};

/// The fields of Agent's and delegate_task's inputs these checks read
#[derive(Debug, Deserialize)]
struct SubagentInput {
    model: Option<String>,
    subagent_type: Option<String>,
    effort: Option<Value>,
    target: Option<Target>,
}

/// delegate_task's `target`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Target {
    model: Option<String>,
    provider_instance_id: Option<String>,
    options: Option<TargetOptions>,
}

/// delegate_task's `target.options`, an object or a list of `{id, value}`
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TargetOptions {
    Map(Map<String, Value>),
    List(Vec<TargetOption>),
}

#[derive(Debug, Deserialize)]
struct TargetOption {
    id: String,
    value: Value,
}

impl TargetOptions {
    fn get(&self, id: &str) -> Option<&Value> {
        match self {
            Self::Map(map) => map.get(id),
            Self::List(list) => list
                .iter()
                .find(|option| option.id == id)
                .map(|option| &option.value),
        }
    }
}

/// A value jq's `//` would not skip, which is anything but null and false
fn set(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| !matches!(value, Value::Null | Value::Bool(false)))
}

impl SubagentInput {
    /// The first effort set, as the jq chain
    /// `.effort // .target.options.effort // .target.options.reasoningEffort` read it
    fn effort(&self) -> Option<&Value> {
        let options = self
            .target
            .as_ref()
            .and_then(|target| target.options.as_ref());
        set(self.effort.as_ref())
            .or_else(|| set(options.and_then(|options| options.get("effort"))))
            .or_else(|| set(options.and_then(|options| options.get("reasoningEffort"))))
    }

    /// Whether the subagent runs Opus, Sonnet, or Haiku. A call that names no
    /// model inherits the parent's, which is a Claude model in the sessions
    /// this hook runs in, unless delegate_task picks another provider
    fn runs_capped_model(&self) -> bool {
        let target = self.target.as_ref();
        let model = self
            .model
            .as_deref()
            .or_else(|| target.and_then(|target| target.model.as_deref()))
            .filter(|model| !model.is_empty());

        match model {
            Some(model) => {
                let model = model.to_ascii_lowercase();
                ["opus", "sonnet", "haiku"]
                    .iter()
                    .any(|family| model.contains(family))
            }
            None => target
                .and_then(|target| target.provider_instance_id.as_deref())
                .is_none_or(|provider| provider.to_ascii_lowercase().contains("claude")),
        }
    }
}

pub const MAX_EFFORT_REASON: &str =
    "Run Opus, Sonnet, and Haiku subagents at xhigh effort or below, not max. If the user explicitly asked for max, tell them this hook blocks it.";

pub const MODEL_REASON: &str = "Name the model on this Agent call (haiku, sonnet, opus, or fable), chosen for what the task needs. Leaving it out inherits the parent model.";

pub fn model_and_effort(input: &HookInput, host: &impl Host) -> Decision {
    let Some(call) = input.tool_input::<SubagentInput>() else {
        return Decision::Allow;
    };

    if call.effort().is_some_and(|effort| effort == "max") && call.runs_capped_model() {
        return Decision::Deny(MAX_EFFORT_REASON.to_owned());
    }

    // the model rule covers Agent only; delegate_task picks its model through `target`
    if input.tool_name.as_deref() != Some("Agent") {
        return Decision::Allow;
    }

    if call.model.as_deref().is_some_and(|model| !model.is_empty()) {
        return Decision::Allow;
    }

    let agent_type = call.subagent_type.as_deref().unwrap_or_default();

    // forks always inherit the parent model and ignore the parameter
    if agent_type == "fork" {
        return Decision::Allow;
    }

    // a custom agent whose frontmatter pins a model already made the choice
    if !agent_type.is_empty() && pins_model(agent_type, host) {
        return Decision::Allow;
    }

    Decision::Deny(MODEL_REASON.to_owned())
}

/// Whether a custom agent definition named `agent_type`, in `~/.claude/agents`
/// or the project's `.claude/agents`, sets `model:` in its frontmatter
fn pins_model(agent_type: &str, host: &impl Host) -> bool {
    let dirs = host
        .home()
        .map(|home| home.join(".claude/agents"))
        .into_iter()
        .chain([host.project_dir().join(".claude/agents")]);

    dirs.filter_map(|dir| std::fs::read_dir(dir).ok())
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .any(|path| frontmatter_pins_model(&path, agent_type))
}

fn frontmatter_pins_model(path: &Path, agent_type: &str) -> bool {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return false;
    };

    let mut lines = contents.lines();
    if lines.next() != Some("---") {
        return false;
    }

    let frontmatter = lines.take_while(|&line| line != "---").collect::<Vec<_>>();
    let value = |line: &str, key: &str| {
        line.strip_prefix(key)
            .map(|rest| rest.trim_start_matches(' ').to_owned())
    };

    let named = frontmatter
        .iter()
        .any(|line| value(line, "name:").is_some_and(|name| name == agent_type));
    let pinned = frontmatter
        .iter()
        .any(|line| value(line, "model:").is_some_and(|model| !model.is_empty()));

    named && pinned
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::super::tests::{decision, pre, Fixture};
    use super::super::{Decision, Event};
    use super::{MAX_EFFORT_REASON, MODEL_REASON};

    fn fixture() -> Fixture {
        let fixture = Fixture::new(true);
        fixture.write_home(
            ".claude/agents/pinned-home.md",
            "---\nname: pinned-home\nmodel: haiku\n---\nbody\n",
        );
        fixture.write_project(
            ".claude/agents/pinned-project.md",
            "---\nname: pinned-project\ndescription: x\nmodel: sonnet\n---\n",
        );
        fixture.write_home(
            ".claude/agents/unpinned.md",
            "---\nname: unpinned\n---\nmodel: opus\n",
        );
        fixture.write_home(
            ".claude/agents/body-only.md",
            "---\ndescription: x\n---\nname: body-only\nmodel: opus\n",
        );
        fixture.write_home(
            ".claude/agents/no-fence.md",
            "name: no-fence\nmodel: opus\n",
        );
        fixture.write_home(
            ".claude/agents/empty-model.md",
            "---\nname: empty-model\nmodel: \n---\n",
        );
        fixture.write_home(
            ".claude/agents/not-markdown.txt",
            "---\nname: not-markdown\nmodel: opus\n---\n",
        );
        fixture
    }

    fn check(tool: &str, rows: &[(Value, Option<&str>)]) {
        let host = fixture();
        for (tool_input, reason) in rows {
            let stdin = pre(tool, tool_input.clone());
            let expected =
                reason.map_or(Decision::Allow, |reason| Decision::Deny(reason.to_owned()));
            assert_eq!(
                decision(Event::PreToolUse, &stdin, &host),
                expected,
                "{tool}: {tool_input}"
            );
        }
    }

    #[test]
    fn agent_calls_name_a_model_below_max_effort() {
        let model = Some(MODEL_REASON);
        let max = Some(MAX_EFFORT_REASON);

        check(
            "Agent",
            &[
                (json!({"description": "x", "prompt": "y"}), model),
                (json!({"prompt": "y", "model": "sonnet"}), None),
                (json!({"prompt": "y", "model": "fable"}), None),
                (json!({"prompt": "y", "model": ""}), model),
                (
                    json!({"prompt": "y", "model": "sonnet", "effort": "max"}),
                    max,
                ),
                (
                    json!({"prompt": "y", "model": "haiku", "effort": "xhigh"}),
                    None,
                ),
                (
                    json!({"prompt": "y", "model": "opus", "effort": "max"}),
                    max,
                ),
                (
                    json!({"prompt": "y", "model": "haiku", "effort": "max"}),
                    max,
                ),
                (
                    json!({"prompt": "y", "model": "fable", "effort": "max"}),
                    None,
                ),
                (json!({"prompt": "y", "subagent_type": "fork"}), None),
                (
                    json!({"prompt": "y", "subagent_type": "fork", "effort": "max"}),
                    max,
                ),
                // built-in agents have no frontmatter here
                (json!({"prompt": "y", "subagent_type": "Explore"}), model),
                (json!({"prompt": "y", "subagent_type": "pinned-home"}), None),
                (
                    json!({"prompt": "y", "subagent_type": "pinned-project"}),
                    None,
                ),
                (
                    json!({"prompt": "y", "model": "", "subagent_type": "pinned-home"}),
                    None,
                ),
                (json!({"prompt": "y", "subagent_type": "unpinned"}), model),
                (json!({"prompt": "y", "subagent_type": "body-only"}), model),
                (json!({"prompt": "y", "subagent_type": "no-fence"}), model),
                (
                    json!({"prompt": "y", "subagent_type": "empty-model"}),
                    model,
                ),
                (
                    json!({"prompt": "y", "subagent_type": "not-markdown"}),
                    model,
                ),
                (json!({"prompt": "y", "subagent_type": "pinned"}), model),
            ],
        );
    }

    #[test]
    fn delegate_task_caps_claude_models_at_xhigh() {
        let max = Some(MAX_EFFORT_REASON);

        check(
            "mcp__t3-code__delegate_task",
            &[
                (json!({"task": "y"}), None),
                (json!({"task": "y", "effort": "max"}), max),
                (
                    json!({"task": "y", "target": {"options": {"effort": "max"}}}),
                    max,
                ),
                (
                    json!({"task": "y", "target": {"options": {"reasoningEffort": "max"}}}),
                    max,
                ),
                (
                    json!({"task": "y", "target": {"options": {"effort": "xhigh"}}}),
                    None,
                ),
                (
                    json!({"task": "y", "target": {"options": {"effort": "high", "reasoningEffort": "max"}}}),
                    None,
                ),
                (
                    json!({"task": "y", "target": {"options": {"effort": false, "reasoningEffort": "max"}}}),
                    max,
                ),
                // the list form the current delegate_task schema also takes
                (
                    json!({"task": "y", "target": {"options": [{"id": "effort", "value": "max"}]}}),
                    max,
                ),
                (
                    json!({"task": "y", "target": {"model": "gpt-5.5", "options": [{"id": "reasoningEffort", "value": "max"}]}}),
                    None,
                ),
                (json!({"task": "y", "target": {"options": []}}), None),
                (
                    json!({"task": "y", "target": {"model": "claude-sonnet-5-5", "options": {"effort": "max"}}}),
                    max,
                ),
                (
                    json!({"task": "y", "target": {"providerInstanceId": "claudeAgent", "options": {"effort": "max"}}}),
                    max,
                ),
                (
                    json!({"task": "y", "target": {"model": "claude-fable-5-1", "options": {"effort": "max"}}}),
                    None,
                ),
                (
                    json!({"task": "y", "target": {"providerInstanceId": "codex", "model": "gpt-6.1-terra", "options": [{"id": "reasoningEffort", "value": "max"}]}}),
                    None,
                ),
                (
                    json!({"task": "y", "target": {"providerInstanceId": "codex", "options": {"reasoningEffort": "max"}}}),
                    None,
                ),
            ],
        );
    }
}
