# Submit a task

## 1. Find the Codex thread id

The spec needs the UUID of the Codex thread that should receive the event. `homebased` accepts only a UUID, not a session name. Submit through the daemon on the machine that owns this thread. Set the spec's `machine` field to choose another Fleet machine to execute the child. Do not submit from the execution machine unless it also owns the Codex thread.

1. Use the thread id if the user or the harness already gave one.
2. Otherwise take the newest session file whose `cwd` matches this workspace and read `session_id` from its first line:

```bash
for f in $(ls -t ~/.codex/sessions/*/*/*/rollout-*.jsonl | head -20); do
  head -c 600 "$f" | grep -q "\"cwd\":\"$PWD\"" && { head -c 600 "$f" | grep -o '"session_id":"[^"]*"'; echo " $f"; break; }
done
```

Several sessions can share one cwd. Tell the user which id you chose. If you cannot find one, ask for it instead of guessing.

## 2. Choose the workload

| Work | `workload.type` |
| --- | --- |
| Long commands: `cargo build`, test suites, render jobs | `task` |
| GitHub CI watch: `gh pr checks … --watch` | `task` |
| A model must reason and produce a report | `agent` |

A `task` runs an argv array with no shell. A caller that needs shell syntax must request it explicitly, for example `["sh", "-lc", "..."]`.

## 3. Write the prompt file (agent only)

For an `agent` workload, write the prompt to a file, not inline JSON. `homebased` copies it into the task directory as `prompt.txt`, so a temp directory is fine:

```bash
dir=$(mktemp -d)
cat > "$dir/prompt.md" <<'PROMPT'
...
PROMPT
```

The prompt must stand alone. The worker has no access to this conversation.

- State the goal, the definition of done, the constraints, and the verification the worker must run.
- Give paths relative to `cwd` or absolute. Name the files the worker should read first.
- Say what to do if blocked: report `blocked` with the exact question. The worker cannot ask you mid-task.
- Do not add reporting instructions. `homebased` appends a fixed trailer that tells the worker how to call `homebased task report`. Leave `report_trailer` at its default `true`.

Task workloads do not create prompt evidence files.

## 4. Write the spec

Agent example:

```json
{
  "api_version": 1,
  "thread": "01a0ab97-a7aa-7463-a5b0-8d500e40e431",
  "name": "implement file browser",
  "cwd": "/home/praveen/code/project",
  "timeout": "1h",
  "workload": {
    "type": "agent",
    "agent": "claude",
    "model": "fable",
    "prompt_file": "/tmp/tmp.abc/prompt.md"
  }
}
```

Task example:

```json
{
  "api_version": 1,
  "thread": "01a0ab97-a7aa-7463-a5b0-8d500e40e431",
  "name": "cargo release build",
  "cwd": "/home/praveen/code/project",
  "timeout": "2h",
  "workload": {
    "type": "task",
    "command": ["cargo", "build", "--release"]
  }
}
```

GitHub CI watcher as a normal task:

```json
"workload": {
  "type": "task",
  "command": ["gh", "pr", "checks", "123", "--watch", "--interval", "30"]
}
```

| Field | Required | Notes |
| --- | --- | --- |
| `api_version` | yes | Always `1`. |
| `thread` | yes | Codex thread UUID from step 1. |
| `name` | yes | Short goal label for the dashboard and events. Name the work, not the agent or the CLI. Trimmed. Rejects blank names, line breaks, control characters, and names longer than 120 Unicode scalar values. Non-unique; task id remains the identity. |
| `cwd` | yes | For local tasks, an existing directory on this machine. For remote tasks, an absolute path or `~/` path on the execution machine. The child runs there. Other relative paths are invalid for remote tasks. |
| `machine` | no | Another Fleet machine that runs the child. Omit it to run locally. The field selects execution; the origin stays on the machine that accepted the submit. |
| `timeout` | no | Attention check. Humantime. Default `1h`, min `30m`. Set an amount that matches the work. Writes to `output.log` restart it; expiry sends `TASK_CHECK_DUE` and does not kill the child. |
| `workload` | yes | Internally tagged enum: `type` is `agent` or `task`. |

Agent-only fields under `workload`:

| Field | Required | Notes |
| --- | --- | --- |
| `agent` | yes | `codex`, `claude`, `grok`, or `opencode`. |
| `prompt` or `prompt_file` | exactly one | For local tasks, a relative `prompt_file` resolves against `cwd`. For remote tasks, use an absolute path on the origin machine. The CLI reads that file and sends its text; the executor never receives the origin path. Prefer `prompt_file`. |
| `model` | no | Passed through unchanged: `-m` for codex and grok, `--model` for claude and opencode. OpenCode accepts provider-qualified values such as `provider/model#variant`; Homebased does not maintain a model allowlist. |
| `extra_args` | no | Array of strings added after the unattended flags. Exact spellings of Homebased-managed standalone switches are reserved tokens: Homebased treats every exact match as that switch, not as another option's value, and emits each at most once. Other tokens keep their order and spelling. Claude defaults to `--output-format stream-json --verbose`. An explicit `--output-format` in either `--output-format VALUE` or `--output-format=VALUE` form replaces the format default; `stream-json` still gets `--verbose` unless `extra_args` already has it. OpenCode defaults to `--format json` and allows an explicit format, but reserves the agent, `cwd`, model, prompt, session, server, and standalone controls. |
| `report_trailer` | no | Default `true`. Set `false` only when the worker must not be told to report. |

Task-only fields under `workload`:

| Field | Required | Notes |
| --- | --- | --- |
| `command` | yes | Non-empty argv. Index 0 is a non-empty program. Later args may be empty. No NUL bytes. No shell. |

Unknown fields and cross-variant fields fail with `invalid_spec` and a JSON pointer. Run `homebased task schema` to print the JSON Schema when in doubt.

The origin and executor have different roles. For a local task, the child uses the submitter's captured `PATH` and `HOME`. For a remote task, the execution machine uses its own daemon environment, `HOME`, installed agents, and toolchain. The executor checks its own `cwd` and expands `~/` using its own home directory. It receives prompt text, not the origin's prompt-file path.

The origin stores the callback context before it sends remote work: the submitter's `PATH` and `HOME`, its absolute current directory, and the resolved local `codex` path. Only the origin uses this context to run `codex queue` for the original thread. The executor does not receive this context. If the saved origin directory or Codex executable later becomes unavailable, callback delivery fails under the event policy in [events.md](events.md); Homebased does not switch to the executor's environment.

The child receives `HOMEBASED_TASK_ID` and `HOMEBASED_HOME` from the execution machine. OpenCode also gets child-only `OPENCODE_PERMISSION={"*":"allow"}`, a generated primary agent through `OPENCODE_CONFIG_CONTENT`, and `PWD` equal to the execution `cwd`. These values do not change persistent configuration or other workloads. For local tasks, submit from a shell where the program and toolchain are on `PATH`. Local `gh` authentication remains available through `HOME`.

## 5. Dry run, then submit

```bash
homebased --json task submit --spec "$dir/spec.json" --dry-run
homebased --json task submit --spec "$dir/spec.json"
```

The dry run validates the spec, resolves the executable and `cwd`, and prints the normalized spec, child argv, and stdin policy. A remote dry run asks the selected executor to validate and expand its invocation. It returns the executor's `execution_cwd`. It spawns nothing and creates no task or request record. Task argv is exact. Agent argv may contain a documented `<task-id>` evidence-path placeholder for Grok. OpenCode dry runs include only a safe `managed_environment` policy and generated-agent description; they do not include inherited configuration or credentials.

Request UUIDs apply only to remote submissions (specs with `machine`). The CLI makes a new request UUID for each remote submit by default. Pass `--request-id <uuid>` to choose and retain the identity before the first request, so the caller can retry after it loses the response. A local spec rejects `--request-id`; omit `machine` to submit locally.

```bash
request_id="$(uuidgen)"
homebased --json task submit --spec "$dir/spec.json" --request-id "$request_id" --dry-run
homebased --json task submit --spec "$dir/spec.json" --request-id "$request_id"
```

The dry run does not store the request UUID. The real remote submit saves one request-to-task mapping before it sends work. If the response is lost or has `submission_outcome_unknown`, retry with the same request UUID and unchanged spec. Homebased resolves the saved outcome; it does not submit the absent task again. Changed content with the same request UUID returns `submission_conflict`. Use a new UUID only for a new submission after a definitive rejection. Local submit responses omit `request_id` because local tasks do not have this retry mapping.

The real submit returns after acceptance, before the child finishes:

```json
{"api_version": 1, "id": "01a0b06f-306c-749e-aa9e-9e1a619ee915", "task_id": "01a0b06f-306c-749e-aa9e-9e1a619ee915", "request_id": "01a0b06f-306c-749e-aa9e-9e1a619ee916", "status": "queued"}
```

## 6. End the turn

Tell the user the task id, the workload, the inactivity timeout, and that results arrive as `HOMEBASED_EVENT` messages in the submitting thread. Do not wait, sleep, or poll. On `TASK_CHECK_DUE`, inspect status and recent logs. If the task is still live but the evidence does not show whether it can make progress, tell the user that the state is uncertain and leave the task running. Cancel or intervene only when evidence requires it. Several tasks may run at once; there is no concurrency bound, so keep the count sensible for the execution machine.

## Resubmitting after a blocked or failed task

There is no resume. Write a new prompt that includes the answer or the fix, name the previous task's `evidence` directory so the worker can read its `output.log`, and submit a new spec.
