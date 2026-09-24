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
| `config_invalid` | 2 | An explicitly selected config file is missing, unreadable, or invalid TOML. | Fix the reported path or setting. Run `homebased --json config validate`. |
| `invalid_spec` | 2 | Bad JSON, unknown field, wrong `api_version`, bad UUID, both or neither of `prompt`/`prompt_file`, timeout below 30m, empty command, cross-variant fields, unreadable `prompt_file`. | Fix the field at `input.pointer`. `homebased task schema` prints the schema. |
| `agent_configuration` | 1 | OpenCode inherited inline configuration is malformed, has the wrong shape, or already defines the generated task agent. | Fix `OPENCODE_CONFIG_CONTENT` without putting credentials in the task spec or logs, then submit again. |
| `cwd_not_found` | 3 | `cwd` is not an existing directory. | Fix `cwd`. |
| `executable_missing` | 3 | Requested program missing, not a file, or not executable. Agents also check `HOMEBASED_<AGENT>` overrides. | Submit from a shell where the program is on `PATH`, use an absolute path, or export `HOMEBASED_CODEX`, `HOMEBASED_CLAUDE`, or `HOMEBASED_GROK`. |
| `task_not_found` | 3 | No task with that full UUID exists in the checked scope. | Check the UUID and known Fleet inventory. `task list` shows local tasks only. |
| `cluster_lookup_incomplete` | 1, retryable | A known Fleet machine could not give a definitive task lookup result. | Retry `task show`, `task log`, or `task cancel` after that peer is reachable. `input.unchecked` lists UUIDs that were not checked. This is not proof that the task is absent. |
| `task_unavailable` | 1, retryable | Homebased knows the execution machine, but cannot return its task detail or log. | Restore the executor connection and retry. `task show` can return cached origin state when it has a route. |
| `task_not_started` | 3 | A retained rejection says the task did not execute. | Read the submission or cancellation result. A prevented task has no process log. |
| `summary_too_long` | 2 | Report summary over 4 KiB. | Shorten the summary; put detail in the log. |
| `too_many_reports` | 5 | A task holds at most 20 reports. | Stop reporting; the last outcome stands. |
| `task_terminal` | 5 | Report or cancel target has already exited. | Nothing to do. Resubmit if more work is needed. |
| `daemon_already_running` | 5 | `daemon serve` while another instance holds `daemon.lock`. | Use the existing daemon. |
| `tasks_in_flight` | 5 | `daemon stop` or `uninstall` with queued or running tasks. | Wait for the events, or pass `--yes` to cancel every in-flight task first. Confirm with the user before `--yes`. |
| `host_unit_home_mismatch` | 5 | `daemon uninstall` while the installed unit's `--home` is a different state directory. `input.selected` and `input.configured` name both homes. | Run uninstall with `--home` / `HOMEBASED_HOME` set to the configured home, or install over the unit from the home you intend to manage. |
| `unit_invalid` | 1 | Generated unit failed `systemd-analyze --user verify` / `plutil`, or an existing unit's daemon invocation could not be parsed. | Run `homebased daemon install --dry-run` and report the unit text to the user. |
| `machine_not_found` | 3 | No known machine matches the requested name or UUID. | Check `homebased --json fleet machines` and config. |
| `machine_unavailable` | 1, retryable | A known machine has no reachable, verified address. | Check its TCP listener, network, and address; then run `homebased --json fleet discover`. |
| `machine_identity_mismatch` | 5 | An address answered for a different machine UUID than the request named. | Do not retry through that address. Refresh discovery and use the address that answers for the expected UUID. |
| `duplicate_machine_identity` | 5 | Two live daemons claim one machine UUID. | Give a cloned installation its own Homebased state directory and machine identity. |
| `duplicate_machine_name` | 5 | More than one live machine has the selected name. | Give each machine a unique `fleet.machine_name`; validate config and rediscover. |
| `cluster_protocol_incompatible` | 5 | The peer and local daemon do not share a supported Fleet protocol version. | Update the incompatible Homebased installation. |
| `submission_outcome_unknown` | 1, retryable | The executor may have accepted the request, but the origin has no definitive reply. | Retry the same spec with the same `--request-id`. Do not create a new request UUID for the same intended task. The error input includes `request_id` and `task_id`. |
| `submission_rejected` | 5 | The executor retained a definitive rejection for this task identity. | Fix the cause and submit again with a new request UUID. |
| `submission_conflict` | 5 | The request UUID was already used with different task content. | Retry with the original content, or use a new UUID for new work. |
| `remote_submission_unavailable` | 1, retryable | This daemon cannot start remote work now. | Check Fleet configuration and the selected executor, then retry. |
| `route_not_found` | 3 | The task has an executor record but no saved origin route. | Do not guess the destination thread. Check task ownership and origin state. |
| `cluster_task_conflict` | 5 | Fleet machines disagree about the owner of this task UUID. | Do not cancel or resubmit it. Resolve the machine identity or task ownership conflict first. |

## Socket down while tasks run

This happens after a raw `systemctl stop`, a crash, or an upgrade in progress. Workers are independent of the daemon and keep running. `daemon status` still reports `in_flight` from SQLite. Start the daemon again with `homebased daemon restart` or `systemctl --user start homebased.service`; on start it reconciles every non-terminal task and delivers any event that was pending, marking a task `lost` only when its worker is gone without `exit.json`. Overdue inactivity reminders are sent again after restart until recorded.

## Callback failed

For a legacy task row, `callback: "failed"` means its terminal `codex queue` delivery failed; the message text is appended to `<home>/callback-fallback.log`. New sequenced events keep a result for each callback. `task show` lists failed event sequences in `failed_events`; callback delivery failure does not change process status. The origin machine uses the saved callback directory, environment, and resolved Codex path. See [events.md](events.md) for retry limits and origin/executor roles.

## Direct-message errors

| `code` | Exit | Cause | Recovery |
| --- | --- | --- | --- |
| `agent_thread_not_found` | 3 | The receiver has no Codex session for the exact thread UUID or `cwd`. | Check the thread id or exact receiver-side `cwd`, then retry. |
| `message_conflict` | 5 | A message UUID already names different message content or a different destination. | Reuse the UUID only for the original request. Use a new UUID for new content. |
| `message_delivery_failed` | 1, retryable | The receiver could not complete the Codex queue attempt. | Retry explicitly with the same `--message-id` and unchanged request. |
| `message_outcome_unknown` | 1, retryable | The receiver may have queued the message, but the sender did not get a valid receipt. | Retry with the same `--message-id` and same destination. Use a machine UUID to pin the receiver. A repeat can reach Codex if the receiver stopped before it saved the receipt. |
| `message_receiver_unavailable` | 1, retryable | The receiver could not inspect local Codex session metadata. | Check the receiver's session files and retry. |
| `message_invalid` | 2 | A message, UUID, or receiver-side `cwd` failed validation. | Fix the reported value. |
