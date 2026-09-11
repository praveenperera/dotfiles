# Codex native collaboration tools

Call these as direct tools (`functions.collaboration.<name>`). They are absent from `functions.exec` and from `ALL_TOOLS`. An empty exec filter does not mean spawn is missing. Prove availability with a direct `collaboration.spawn_agent` call. A schema or field error means the tool exists.

`list_agents` showing only `/root` is the starting tree. `followup_task` cannot create an agent.

## Spawn

Call `collaboration.spawn_agent`.

Required:

- `task_name`: lowercase letters, digits, and underscores only
- `message`: the full self-contained task contract

Optional:

- `fork_turns`: `"none"` for a fresh worker in this workflow; `"all"` or a positive integer string only when the user requests inherited history
- `model` and `reasoning_effort`: only when the user, `AGENTS.md`, or an active skill requests them, and only with `fork_turns` of `"none"` or a positive integer
- `agent_type`: omit unless a configured role is required

Unknown fields fail. Current Codex v2 accepts `message`, `task_name`, `agent_type`, `model`, `reasoning_effort`, `fork_turns`, and `fork_context`; `fork_context` is rejected at runtime, so use `fork_turns`.

A successful spawn returns `{"task_name":"/root/<task_name>"}`. Use that path for later collaboration calls.

## After spawn

- `collaboration.followup_task`: new work on an existing path
- `collaboration.send_message`: message without starting a turn
- `collaboration.wait_agent`, `interrupt_agent`, `list_agents`: as needed

## CLI fallback

Use [codex-cli.md](codex-cli.md) only when a direct `collaboration.spawn_agent` call is rejected as unregistered or unknown. A schema or field error is not that failure.
