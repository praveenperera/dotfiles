---
name: homebased
description: Run long, unattended agent CLIs and general task commands through the homebased daemon and handle the HOMEBASED_EVENT callback that returns to this Codex thread. Use when the user invokes $homebased or $hbd, when work should run in the background and report back later, when a HOMEBASED_EVENT message arrives, when checking, cancelling, or resubmitting homebased tasks, or when this session is itself a homebased worker. Do not use for work that finishes inside the current turn.
---

# Homebased

`homebased` (invoke as `$homebased` or `$hbd`) is a user daemon that runs one detached child per supervised task and sends `HOMEBASED_EVENT` messages to the submitting Codex thread. A task is either an `agent` workload (Codex, Claude, Grok, or OpenCode with a prompt and reporting trailer) or a `task` workload (arbitrary argv such as `cargo build --release` or `gh pr checks --watch`). The daemon owns the lifecycle end to end: queue, run, stream combined output to `output.log`, send a check reminder when output stays idle for the timeout, cancel only on explicit request, and deliver events. The orchestrator submits a JSON spec, ends its turn, and acts when events arrive.

## Rules that hold everywhere

- Never run `codex queue` yourself, and never tell a worker to run it. Delivery belongs to `homebased`.
- Submit through the daemon on the machine that owns the Codex thread. To run the child elsewhere, enable Fleet and set `machine` in the JSON spec. The submitting machine remains the origin and sends callbacks to the original thread; the selected Fleet machine executes the child.
- Put task fields in the JSON spec and submit with `homebased task submit --spec <file|->`. Use `--request-id <uuid>` when a caller needs a stable retry identity.
- Always set `name` to a short goal label. Do not name the task after the agent or the CLI.
- Always pass `--json` on data commands and parse the result. Every JSON object carries `api_version: 1`.
- Event delivery is at-least-once. For new events, deduplicate by `(task, seq)`; for a legacy event without `seq`, use `(task, event)`.
- For GPU resource work, read [resource-loans.md](references/resource-loans.md). Check the exact pending actions at start, after compaction, and before an independent background launch. A delivered notice is not completion. An unavailable authority is not an empty action list.
- Use `homebased message send` for a direct Codex-thread message. Read [messages.md](references/messages.md) for destination, source, and retry rules.
- Do not poll a running task in a loop. Submit, tell the user the task id, end the turn, and wait for events. Inspect on demand only.
- Use `homebased daemon stop` or `homebased daemon restart`, never raw `systemctl` or `launchctl`, so in-flight tasks are protected.
- On Praveen's machines, every daemon install or reinstall must set `HOMEBASED_WEB_LISTEN=0.0.0.0:7677`. The dashboard is off unless this is set. Do not omit it, and do not replace the LAN bind with loopback.
- Task ids are full UUIDs. Prefix matching does not exist.
- Prefer `workload.type: "task"` for long commands and CI watchers. Use `agent` only when a model must reason and produce a report.
- Set `timeout` to match the work (default 1h, min 30m). Quiet `output.log` for that long sends `TASK_CHECK_DUE`; it never kills the child.
- Claude streams JSON output by default. Set `--output-format` in `extra_args` only when the task needs another format.
- OpenCode runs in standalone mode with JSON output and receives the prompt on stdin. Its `model` may be a provider-qualified value such as `provider/model#variant`; its child-only full work permissions do not change persistent OpenCode configuration.

## Route

Pick the first row that matches, then read only that file.

| Situation | Read |
| --- | --- |
| `HOMEBASED_TASK_ID` is set in this session's environment | [worker.md](references/worker.md). You are the worker, not the orchestrator. |
| A message starting with `HOMEBASED_EVENT ` arrived | [events.md](references/events.md) |
| Starting background work, writing a spec, choosing agent or task, timeout, or finding the thread id | [submit.md](references/submit.md) |
| Listing, showing, reading logs, or cancelling tasks | [inspect.md](references/inspect.md) |
| Configuring Fleet or discovering machines | [fleet.md](references/fleet.md) |
| Sending a direct Codex-thread message | [messages.md](references/messages.md) |
| A command exited non-zero, `daemon_unavailable`, or the socket is down | [errors.md](references/errors.md) |
| Supervising shared GPU work or a resource loan | [resource-loans.md](references/resource-loans.md) |
| `homebased` is missing, the daemon is not installed, or the binary was rebuilt | [setup.md](references/setup.md) |

## Minimal flow

```bash
homebased --json daemon status                         # socket must be "up"
homebased --json task submit --spec "$spec" --dry-run # validate; a remote spec checks its executor
homebased --json task submit --spec "$spec"           # returns task and request UUIDs
```

Then end the turn. Events arrive later as one line: `HOMEBASED_EVENT {...}`.
