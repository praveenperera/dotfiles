# Inspect and cancel tasks

Inspection commands need the local daemon socket. `task report` writes to the local state database, and `daemon status` can inspect the local state while the socket is down.

## List

```bash
homebased --json task list --thread <thread-uuid>          # this thread's tasks
homebased --json task list --status running,queued         # in flight
homebased --quiet task list --status running               # bare ids, one per line
```

Status values: `queued`, `running`, `succeeded`, `failed`, `cancelled`, `lost`. `--status` accepts repeats or a comma list.

Each entry: `id`, `name`, `display_name`, `status`, `workload`, `thread`, `cwd`, `project_root`, `origin_machine`, `execution_machine`, `pid`, `callback`, `timeout_secs`, `check_timeout`, `exit_reason`, `cancel_requested_at`, `created_at`, `updated_at`. `project_root`, `origin_machine`, and `execution_machine` can be absent. Entries come back in id order, which is creation order. Human list output shows `display_name`. `name` is omitted only for tasks stored before it was required.

`task list` reads tasks stored on this machine. It does not query every Fleet peer.

## Show

```bash
homebased --json task show <id>
```

| Field | Meaning |
| --- | --- |
| `name` | Submitted goal label. Omitted only for tasks stored before name was required. |
| `display_name` | Non-empty label: the submitted name, or a workload fallback for unnamed stored rows. |
| `status` | Process status, see above. |
| `workload` | `{"type":"agent","agent":"…","model":null\|string}` or `{"type":"task","command":[…]}`. |
| `exit_reason` | `null` while running, else the tagged payload (`exit`, `signal`, `cancelled`, `spawn_failed`). |
| `callback` | Retained task-row callback state. For tasks with sequenced events, use `failed_events` for per-event callback failures. |
| `cancel_requested_at` | Set once `task cancel` ran. |
| `reports` | Worker reports with `seq`, `outcome`, `summary`, `reported_at`, and `notified_at` when `--notify` succeeded. |
| `evidence` | Task directory. |
| `output_log` | Path of the combined stdout and stderr of the child. |
| `last_event` | The event object already sent, or the one that will be sent. `null` while running with no interim event. |
| `pid` | Worker pid, for display only. Liveness is the lock, not the pid. |
| `timeout_secs` | Output-inactivity timeout in seconds. Not remaining execution budget. |
| `check_timeout` | `pending` or `sent` for the inactivity reminder. |
| `created_at` | Insert time. |
| `updated_at` | Last row change. For a terminal task this is the finish time. |

Fleet-aware `show` results can also include `origin_machine`, `execution_machine`, `found_on`, `availability`, `submission`, `last_accepted_seq`, `last_settled_seq`, `last_update`, and `failed_events`. `submission` describes durable acceptance; it is not process status. A callback failure does not change the process status.

When the task is not stored locally, `task show` checks known Fleet machines in parallel. It follows any saved origin route to its execution machine. A remote result also has `origin_machine`, `execution_machine`, `found_on`, and `availability`. The executor record is the source for process state, reports, log path, and evidence. If an origin route is known but its executor is offline, `show` returns the cached submission and last known state with an unavailable marker. The cached response does not invent a process status for an unknown submission.

The lookup scope is the known Fleet at the time of the request. If any machine cannot give a definitive answer, the CLI returns the retryable `cluster_lookup_incomplete` error with the unchecked machine UUIDs. It returns `task_not_found` only when every machine in that scope gives a definitive negative answer. A known route whose executor cannot be reached is not `task_not_found`.

## Log

```bash
homebased task log <id> --tail 100
homebased --json task log <id>       # {"id", "log", "truncated"}
```

The local daemon reads `output.log` on the execution machine. For a remote task, it gets the log from the executor over Fleet. The log can be empty while the child has not written anything yet. `--tail` keeps at most 5000 lines; `truncated` is true when earlier lines were dropped. If the executor is offline or the log has been removed, the CLI returns `task_unavailable`. A task prevented before it started has no log and returns `task_not_started`.

## Dashboard

The daemon serves a read-only HTTP dashboard only when `--web-listen` / `HOMEBASED_WEB_LISTEN` is a host:port. Open that URL in a browser to see every task, its status, and its log tail without an agent turn.

```bash
homebased --json daemon status       # "web" holds the URL, or null when the dashboard is off or the socket is down
curl -s http://main:7677/v1/tasks
curl -s "http://main:7677/v1/tasks/<id>/log?tail=200"
```

The listener answers `GET /v1/status`, `GET /v1/tasks`, `GET /v1/tasks/<id>`, and `GET /v1/tasks/<id>/log?tail=<lines>`, which returns `{"id", "log", "truncated"}`. Submit and cancel are refused there with 405; they belong to the Unix socket. See [setup.md](setup.md) for `--web-listen`.

## Task directory

`evidence` points at `<home>/tasks/<id>/` on the execution machine. For a remote task, `<home>` is the executor's state directory. The origin keeps its callback log and delivery lock in its own task directory. For a local task, origin and executor share one machine.

| File | Content |
| --- | --- |
| `prompt.txt` | Agent only: the submitted prompt, byte for byte. |
| `prompt.trailer.txt` | Agent only: the reporting trailer, when enabled. |
| `prompt.feed.txt` | Agent only: what the agent actually received. |
| `output.log` | Child stdout and stderr. |
| `exit.json` | Written by the worker parent at exit. Absent for `lost` tasks. |
| `callback.log` | Output of the last `codex queue` attempt. |
| `delivery.lock` | Serializes callback delivery across daemon restarts. |
| `runner.lock` | Liveness lock. Held while the worker parent is alive. |

`<home>` is `--home`, else `HOMEBASED_HOME`, else `$XDG_STATE_HOME/homebased`, else `~/.local/state/homebased`. The SQLite database is `<home>/homebased.sqlite` and is the source of truth; do not edit it.

## Cancel

```bash
homebased --json task cancel <id>
```

Sends SIGTERM to the worker, which forwards it to the child's process group, waits for the group to disappear, and sends SIGKILL after 10 seconds if descendants remain. The exit event arrives as `TASK_CANCELLED`. Cancelling a terminal task exits 0 and changes nothing. A queued task with no worker yet is cancelled directly.

When the task is not local, `task cancel` looks up its origin and executor, then saves a cancellation request on the machine where you ran the command. The local daemon retries delivery after a network failure or restart. The JSON response has `delivery.state`: `pending` means the executor has not acknowledged the request; `delivered` means it has saved the request. The executor result can still be `pending_application`, so check task status to learn when the child has stopped. A terminal task keeps its actual result. A task with incomplete Fleet lookup returns `cluster_lookup_incomplete`; it is not reported as cancelled.
