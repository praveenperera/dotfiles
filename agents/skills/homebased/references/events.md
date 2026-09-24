# Handle a HOMEBASED_EVENT

The message is one line: the literal prefix `HOMEBASED_EVENT ` followed by one JSON object. Strip the prefix and parse the rest. Do not depend on JSON key order.

| Key | Meaning |
| --- | --- |
| `api_version` | Always `1`. |
| `event` | One of the names in the table below. |
| `task` | Task UUID. Use it for `task show`, `task log`, and de-duplication. |
| `seq` | Per-task event sequence. Present on new events. Use with `task` as the event identity. It is separate from the `seq` on each worker report. |
| `origin_machine` | Stable UUID of the machine that owns the Codex thread and sends the callback. Present on new events. |
| `execution_machine` | Stable UUID of the machine that ran the child. Present on new events. |
| `name` | Submitted task name. Omitted only for tasks stored before name was required. |
| `display_name` | Non-empty server-derived label: the submitted name, or a workload fallback for unnamed stored rows. |
| `workload` | Discriminated union: `{"type":"agent","agent":"…","model":null\|string}` or `{"type":"task","command":[…]}`. |
| `thread` | The thread the event was addressed to. |
| `cwd` | The child's working directory. |
| `evidence` | Absolute task directory on the execution machine. Agent tasks contain prompt evidence, `output.log`, and `exit.json`. Task workloads omit prompt files. Origin callback logs stay on the origin machine. |
| `reports` | Worker reports in `seq` order, each `{"seq", "outcome", "summary"}`. `[]` when the worker never reported. |
| `process` | Tagged exit payload, or `null` for interim events. |
| `timeout_secs` | Present on `TASK_CHECK_DUE`: the configured output-inactivity timeout. |
| `next_action` | Suggested step. Follow it unless the reports say otherwise. |

`process` values: `{"kind":"exit","code":n}`, `{"kind":"signal","signal":n}`, `{"kind":"cancelled"}`, `{"kind":"spawn_failed","message":"…"}`, `{"kind":"runner_lost"}`. Output inactivity never appears as a process result.

## Event table

| `event` | `next_action` | What happened | Do this |
| --- | --- | --- | --- |
| `TASK_REPORTED` | `read_report` | Worker sent an interim report with `--notify`. The process is still running; `process` is `null`. | Read the single report. If it is `blocked`, prepare the answer, but do not resubmit yet: the exit event still follows and the worker may have continued. |
| `TASK_CHECK_DUE` | `inspect_task` | The child was still live after it wrote no output for the full inactivity timeout. `process` is `null`, and status is unchanged. | Inspect current status and recent logs because output can resume after the event. If the evidence does not show whether the task can make progress, tell the user that the state is uncertain and leave it running. Cancel or intervene only when evidence requires it. A terminal event still follows later. |
| `TASK_SUCCEEDED` | `review_output` | Exit 0 and the last report was `succeeded`, or there were no reports. | Read the last summary. With `reports: []` (normal for task workloads) read `output.log` before trusting the result. Verify the work in `cwd` before telling the user it is done. |
| `TASK_BLOCKED` | `answer_and_resubmit` | The last report was `blocked`. The process has exited. | Answer the question. If it needs the user, ask them. Then submit a new task whose prompt contains the answer (see submit.md). |
| `TASK_FAILED` | `inspect_log` | Last report `failed`, non-zero exit, signal, or spawn failure. | Run `homebased task log <id> --tail 200`. Decide: fix the prompt and resubmit, raise the check timeout, fix the environment for `spawn_failed`, or report to the user. |
| `TASK_CANCELLED` | `none` | `task cancel` or `daemon stop --yes` ended it. | Nothing unless the user wants it rerun. |
| `TASK_LOST` | `inspect_log` | The worker process disappeared without writing `exit.json`, for example after a machine reboot or a `kill -9`. | Check `homebased --json daemon status` and the log. Resubmit if the work is incomplete. |

## Duplicates

Delivery is at-least-once. For new events, use `(task, seq)` as the identity. If the same pair arrives again, it is a redelivery; acknowledge it and take no new action. Do not use the event name alone: one task can have several `TASK_REPORTED` callbacks. New callback sequence values can skip because state changes and non-notifying reports also use the per-task sequence.

Older events do not have `seq`. For those messages, use `(task, event)` as the fallback identity. A `TASK_CHECK_DUE` or `TASK_REPORTED` followed by a different exit event for the same task is the normal multi-message case, not a duplicate. The exit event carries the full `reports` list.

## Origin and execution machines

The execution machine owns the child process, reports, logs, and event outbox. The origin machine owns the task route, the Codex thread, and callback delivery. Only the origin machine runs `codex queue`. It uses the `PATH`, `HOME`, current directory, and resolved Codex path saved at submission. It does not use the executor's environment or `cwd`.

The executor keeps an event until the origin confirms that it stored the event. It retries when the origin is not reachable. A transport outage does not change process status or stop the child. The origin also saves each callback before it tries `codex queue`, and resumes pending delivery after restart.

Callback delivery is at-least-once. The origin reserves up to three queue-command attempts per callback event. A crash after Codex accepts a message but before Homebased records success can cause a duplicate. After a permanent error or three failed attempts, Homebased records that event as `delivery_failed` and continues with later events. Callback failure does not change the process result. `task show` reports failed event sequences in `failed_events`.

## Reading the evidence

```bash
homebased --json task show <id>          # status, workload, check timeout, exit_reason, reports, output_log, last_event
homebased task log <id> --tail 200       # reads the executor log through the local daemon
```

`task show --json` needs the local daemon socket. It returns `last_event` with the same object as the message, so you can recover an event you missed. For a remote task, the daemon asks the executor for the log. See [inspect.md](inspect.md) for offline behavior.
