---
name: hbd
description: Run long, unattended agent CLIs and general task commands through the homebasd (homebased) daemon and handle the HOMEBASED_EVENT callback that returns to this Codex thread. Use when the user invokes $hbd, when work should run in the background and report back later, when a HOMEBASED_EVENT message arrives, when checking, cancelling, or resubmitting homebased tasks, or when this session is itself a homebased worker. Do not use for work that finishes inside the current turn.
---

# Homebasd

`homebased` (repo `homebasd`, invoke as `$hbd`) is a user daemon that runs one detached child per supervised task and sends `HOMEBASED_EVENT` messages back to the submitting Codex thread. A task is either an `agent` workload (Codex, Claude, or Grok with a prompt and reporting trailer) or a `task` workload (arbitrary argv such as `cargo build --release` or `gh pr checks --watch`). The daemon owns the lifecycle end to end: queue, run, stream combined output to `output.log`, send a check reminder when the attention timeout expires, cancel only on explicit request, and deliver the terminal callback. The orchestrator submits a JSON spec, ends its turn, and acts when events arrive.

## Rules that hold everywhere

- Never run `codex queue` yourself, and never tell a worker to run it. Delivery belongs to `homebased`.
- Submit only with `homebased task submit --spec <file|->`. There are no per-field submit flags.
- Always set `name` to a short goal label. Do not name the task after the agent or the CLI.
- Always pass `--json` on data commands and parse the result. Every JSON object carries `api_version: 1`.
- Do not poll a running task in a loop. Submit, tell the user the task id, end the turn, and wait for events. Inspect on demand only.
- Use `homebased daemon stop` or `homebased daemon restart`, never raw `systemctl` or `launchctl`, so in-flight tasks are protected.
- Task ids are full UUIDs. Prefix matching does not exist.
- Delivery is at-least-once. Treat a repeated event for the same task and event name as a duplicate, not a new result.
- Prefer `workload.type: "task"` for long commands and CI watchers. Use `agent` only when a model must reason and produce a report.
- `timeout` is an attention timer (default 4h, minimum 2h). It sends `TASK_CHECK_DUE` and never kills the child.

## Route

Pick the first row that matches, then read only that file.

| Situation | Read |
| --- | --- |
| `HOMEBASED_TASK_ID` is set in this session's environment | [worker.md](references/worker.md). You are the worker, not the orchestrator. |
| A message starting with `HOMEBASED_EVENT ` arrived | [events.md](references/events.md) |
| Starting background work, writing a spec, choosing agent or task, timeout, or finding the thread id | [submit.md](references/submit.md) |
| Listing, showing, reading logs, or cancelling tasks | [inspect.md](references/inspect.md) |
| A command exited non-zero, `daemon_unavailable`, or the socket is down | [errors.md](references/errors.md) |
| `homebased` is missing, the daemon is not installed, or the binary was rebuilt | [setup.md](references/setup.md) |

## Minimal flow

```bash
homebased --json daemon status                      # socket must be "up"
homebased --json task submit --spec "$spec" --dry-run   # validate, see child argv
homebased --json task submit --spec "$spec"         # returns {"id": "<uuid>", "status": "queued"}
```

Then end the turn. Events arrive later as one line: `HOMEBASED_EVENT {...}`.
