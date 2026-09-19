# Errors and recovery

Errors go to stderr. With `--json` they are one object:

```json
{"api_version":1,"error":{"code":"invalid_spec","message":"…","retryable":false,"input":{"pointer":"/thread","value":"nope"}}}
```

`input` echoes what failed: a JSON pointer and value for specs, the task id for lookups, the id and status for terminal-task conflicts, the connect message for socket failures.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | success |
| 1 | error |
| 2 | usage, including a bad spec or `--json` with `--quiet` |
| 3 | not found |
| 4 | permission |
| 5 | conflict |

## Error codes

| `code` | Exit | Cause | Recovery |
| --- | --- | --- | --- |
| `daemon_unavailable` | 1, retryable | Socket missing or refused. | `homebased --json daemon status`. If `socket` is `down`, read setup.md and start or restart the daemon. Tasks already running keep running and still report. |
| `invalid_spec` | 2 | Bad JSON, unknown field, wrong `api_version`, bad UUID, both or neither of `prompt`/`prompt_file`, timeout below 2h, empty command, cross-variant fields, unreadable `prompt_file`. | Fix the field at `input.pointer`. `homebased task schema` prints the schema. |
| `cwd_not_found` | 3 | `cwd` is not an existing directory. | Fix `cwd`. |
| `executable_missing` | 3 | Requested program missing, not a file, or not executable. Agents also check `HOMEBASED_<AGENT>` overrides. | Submit from a shell where the program is on `PATH`, use an absolute path, or export `HOMEBASED_CODEX`, `HOMEBASED_CLAUDE`, or `HOMEBASED_GROK`. |
| `task_not_found` | 3 | Unknown task id. Ids are full UUIDs. | Use `task list --json` to find the id. |
| `summary_too_long` | 2 | Report summary over 4 KiB. | Shorten the summary; put detail in the log. |
| `too_many_reports` | 5 | A task holds at most 20 reports. | Stop reporting; the last outcome stands. |
| `task_terminal` | 5 | Report or cancel target has already exited. | Nothing to do. Resubmit if more work is needed. |
| `daemon_already_running` | 5 | `daemon serve` while another instance holds `daemon.lock`. | Use the existing daemon. |
| `tasks_in_flight` | 5 | `daemon stop` or `uninstall` with queued or running tasks. | Wait for the events, or pass `--yes` to cancel every in-flight task first. Confirm with the user before `--yes`. |
| `unit_invalid` | 1 | Generated systemd unit failed `systemd-analyze --user verify`. | Run `homebased daemon install --dry-run` and report the unit text to the user. |

## Socket down while tasks run

This happens after a raw `systemctl stop`, a crash, or an upgrade in progress. Workers are independent of the daemon and keep running. `daemon status` still reports `in_flight` from SQLite. Start the daemon again with `homebased daemon restart` or `systemctl --user start homebased.service`; on start it reconciles every non-terminal task and delivers any event that was pending, marking a task `lost` only when its worker is gone without `exit.json`. Overdue attention reminders are sent again after restart until recorded.

## Callback failed

`task show` with `callback: "failed"` means `codex queue` failed three times. The message text is appended to `<home>/callback-fallback.log`. Read it there, check that `codex` is on the daemon's `PATH` and that submit ran on this machine, and re-run `homebased daemon install` if the install-time `PATH` is stale.
