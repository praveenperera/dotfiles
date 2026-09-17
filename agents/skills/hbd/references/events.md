# Handle a HOMEBASED_EVENT

The message is one line: the literal prefix `HOMEBASED_EVENT ` followed by one JSON object. Strip the prefix and parse the rest. Keys, in order:

| Key | Meaning |
| --- | --- |
| `api_version` | Always `1`. |
| `event` | One of the names in the table below. |
| `task` | Task UUID. Use it for `task show`, `task log`, and de-duplication. |
| `workload` | Discriminated union: `{"type":"agent","agent":"…","model":null\|string}` or `{"type":"task","command":[…]}`. |
| `thread` | The thread the event was addressed to. |
| `cwd` | The child's working directory. |
| `evidence` | Absolute task directory. Agent tasks contain `prompt.txt`, `output.log`, `exit.json`, `callback.log`. Task workloads omit prompt files. |
| `reports` | Worker reports in `seq` order, each `{"seq", "outcome", "summary"}`. `[]` when the worker never reported. |
| `process` | Tagged exit payload, or `null` for interim events. |
| `timeout_secs` | Present on `TASK_CHECK_DUE`: the configured attention timeout. |
| `next_action` | Suggested step. Follow it unless the reports say otherwise. |

`process` values: `{"kind":"exit","code":n}`, `{"kind":"signal","signal":n}`, `{"kind":"cancelled"}`, `{"kind":"spawn_failed","message":"…"}`, `{"kind":"runner_lost"}`. Attention timeout never appears as a process result.

## Event table

| `event` | `next_action` | What happened | Do this |
| --- | --- | --- | --- |
| `TASK_REPORTED` | `read_report` | Worker sent an interim report with `--notify`. The process is still running; `process` is `null`. | Read the single report. If it is `blocked`, prepare the answer, but do not resubmit yet: the exit event still follows and the worker may have continued. |
| `TASK_CHECK_DUE` | `inspect_task` | The attention timer expired. The child is still running; `process` is `null`. Status is unchanged. | Inspect status and recent logs. Report useful progress to the user. Cancel or intervene only when evidence requires it. A terminal event still follows later. |
| `TASK_SUCCEEDED` | `review_output` | Exit 0 and the last report was `succeeded`, or there were no reports. | Read the last summary. With `reports: []` (normal for task workloads) read `output.log` before trusting the result. Verify the work in `cwd` before telling the user it is done. |
| `TASK_BLOCKED` | `answer_and_resubmit` | The last report was `blocked`. The process has exited. | Answer the question. If it needs the user, ask them. Then submit a new task whose prompt contains the answer (see submit.md). |
| `TASK_FAILED` | `inspect_log` | Last report `failed`, non-zero exit, signal, or spawn failure. | Run `homebased task log <id> --tail 200`. Decide: fix the prompt and resubmit, raise the check timeout, fix the environment for `spawn_failed`, or report to the user. |
| `TASK_CANCELLED` | `none` | `task cancel` or `daemon stop --yes` ended it. | Nothing unless the user wants it rerun. |
| `TASK_LOST` | `inspect_log` | The worker process disappeared without writing `exit.json`, for example after a machine reboot or a `kill -9`. | Check `homebased --json daemon status` and the log. Resubmit if the work is incomplete. |

## Duplicates

Delivery is at-least-once. A second message with the same `task` and `event` is a redelivery. Acknowledge it and take no new action. A `TASK_CHECK_DUE` or `TASK_REPORTED` followed by an exit event for the same task is the normal multi-message case, not a duplicate; the exit event carries the full `reports` list.

## Reading the evidence

```bash
homebased --json task show <id>          # status, workload, check timeout, exit_reason, reports, output_log, last_event
homebased task log <id> --tail 200       # reads output.log from disk; works with the daemon down
```

`task show --json` returns `last_event` with the same object as the message, so you can recover an event you missed.
