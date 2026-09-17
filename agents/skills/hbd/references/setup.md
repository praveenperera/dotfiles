# Setup and daemon lifecycle

## Build and install the binary

The crate lives in this repository. `cargo install` puts the binary at `~/.cargo/bin/homebased`, which must be on `PATH` for the orchestrator, the worker, and the host unit:

```bash
just web-build                        # build the dashboard assets
cargo install --path /home/praveen/code/homebasd
homebased --json version
```

The dashboard is embedded in the binary at compile time, so `just web-build` must run before `cargo install`. Without it the binary still works and serves a "dashboard is not built" page.

## Install the daemon

```bash
homebased daemon install --dry-run    # print the unit or plist
homebased daemon install              # write, verify, enable, start
homebased --json daemon status        # {"socket": "up", "in_flight": 0, "home": "...", "web": "http://127.0.0.1:7677"}
```

The unit's `ExecStart` points at the binary that ran `install`, so run it as the installed `homebased`, not `target/debug/homebased`. Install is an idempotent apply. Run it from a shell where `codex`, `claude`, `grok`, and the project toolchains are on `PATH`: the installer bakes that `PATH` and the absolute agent paths (`HOMEBASED_CODEX`, `HOMEBASED_CLAUDE`, `HOMEBASED_GROK`) into the unit. Re-run it after `PATH` changes. An agent that is not on `PATH` at install time is silently left out of the unit; only the unit's `PATH` is left to find it later.

- Linux: user unit `~/.config/systemd/user/homebased.service`, `KillMode=process`, `Restart=on-failure`. If install warns that lingering is off, run `loginctl enable-linger $USER` so the daemon survives logout.
- macOS: `~/Library/LaunchAgents/dev.praveen.homebased.plist` with `KeepAlive` and `AbandonProcessGroup`.

State directory: `--home`, else `HOMEBASED_HOME`, else `$XDG_STATE_HOME/homebased`, else `~/.local/state/homebased`. It holds `homebased.sqlite`, `homebased.sock`, `daemon.lock`, `tasks/<id>/`, and `callback-fallback.log`.

## Dashboard listener

`daemon serve` binds a read-only HTTP listener for the dashboard, `127.0.0.1:7677` by default. Change it with `--web-listen <addr|off>` or `HOMEBASED_WEB_LISTEN`:

```bash
homebased daemon serve --web-listen off              # socket only
HOMEBASED_WEB_LISTEN=127.0.0.1:9000 homebased daemon serve
HOMEBASED_WEB_LISTEN=0.0.0.0:7677 homebased daemon install   # bake a LAN bind into the host unit
```

`daemon install` copies `HOMEBASED_WEB_LISTEN` from the installing shell into the unit, next to `PATH` and the agent paths, and rejects an invalid value. Re-run `install` to change it.

The listener has no authentication and exposes cwd paths, prompts, and logs, so bind a non-loopback address only on a trusted network. A bind failure (busy port) is a warning: the daemon keeps serving the socket and `daemon status` reports `"web": null`.

## Upgrade

After rebuilding the binary:

```bash
just web-build
cargo install --path /home/praveen/code/homebasd
homebased --json daemon restart
```

`restart` restarts only the daemon. Running workers are separate processes that hold their own lock; they keep running, and the restarted daemon reconciles them and delivers their events. Do not use raw `systemctl restart`; the CLI path works whether or not a host unit exists.

## Stop and uninstall

```bash
homebased --json daemon stop            # exit 5 tasks_in_flight if anything is queued or running
homebased --json daemon stop --yes      # cancel in-flight tasks, wait for their events, then stop
homebased --json daemon uninstall       # same refusal rule; --yes cancels first
```

`uninstall` removes the host unit only. The database and task directories stay. Confirm with the user before `--yes`; it cancels their tasks.

## Foreground run for debugging

```bash
homebased daemon serve --home /tmp/hb-test
```

Only one `serve` per home; a second exits 5 `daemon_already_running`. Use a separate `--home` for experiments so the real state is untouched.
