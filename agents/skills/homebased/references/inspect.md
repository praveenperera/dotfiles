# Inspect and cancel tasks

All commands need the daemon socket except `task log`, `task report`, and `daemon status`.

## List

```bash
homebased --json task list --thread <thread-uuid>          # this thread's tasks
homebased --json task list --status running,queued         # in flight
homebased --quiet task list --status running               # bare ids, one per line
```

Status values: `queued`, `running`, `succeeded`, `failed`, `cancelled`, `lost`. `--status` accepts repeats or a comma list.

Each entry: `id`, `name`, `display_name`, `status`, `workload`, `thread`, `cwd`, `pid`, `callback`, `timeout_secs`, `check_timeout`, `exit_reason`, `cancel_requested_at`, `created_at`, `updated_at`. Entries come back in id order, which is creation order. Human list output shows `display_name`. `name` is omitted only for tasks stored before it was required.

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
| `callback` | `pending`, `sending`, `sent`, or `failed`. `failed` means three `codex queue` attempts failed; the line is in `<home>/callback-fallback.log`. |
| `cancel_requested_at` | Set once `task cancel` ran. |
| `reports` | Worker reports with `seq`, `outcome`, `summary`, `reported_at`, and `notified_at` when `--notify` succeeded. |
| `evidence` | Task directory. |
| `output_log` | Path of the combined stdout and stderr of the child. |
| `last_event` | The event object already sent, or the one that will be sent. `null` while running with no interim event. |
| `pid` | Worker pid, for display only. Liveness is the lock, not the pid. |
| `timeout_secs` | Attention (check) timeout in seconds. Not remaining execution budget. |
| `check_timeout` | `pending` or `sent` for the attention reminder. |
| `created_at` | Insert time. |
| `updated_at` | Last row change. For a terminal task this is the finish time. |

## Log

```bash
homebased task log <id> --tail 100
homebased --json task log <id>       # {"id", "log", "truncated"}
```

Reads `output.log` directly from disk. The log can be empty while the child has not written anything yet. `--tail` keeps at most 5000 lines; `truncated` is true when earlier lines were dropped.

## Dashboard

The daemon also serves a read-only HTTP listener, by default `http://127.0.0.1:7677`. Open it in a browser to see every task, its status, and its log tail without an agent turn.

```bash
homebased --json daemon status       # "web" holds the URL, or null when the socket is down
curl -s http://127.0.0.1:7677/v1/tasks
curl -s "http://127.0.0.1:7677/v1/tasks/<id>/log?tail=200"
```

The listener answers `GET /v1/status`, `GET /v1/tasks`, `GET /v1/tasks/<id>`, and `GET /v1/tasks/<id>/log?tail=<lines>`, which returns `{"id", "log", "truncated"}`. Submit and cancel are refused there with 405; they belong to the Unix socket. See [setup.md](setup.md) for `--web-listen`.

## Task directory

`evidence` points at `<home>/tasks/<id>/`:

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
