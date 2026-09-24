# Setup and daemon lifecycle

## Build and install the binary

The crate lives in this repository. `cargo install` puts the binary at `~/.cargo/bin/homebased`, which must be on `PATH` for the orchestrator, the worker, and the host unit:

```bash
just web-build                        # build the dashboard assets
cargo install --path /home/praveen/code/homebased
homebased --json version
```

The dashboard is embedded in the binary at compile time, so `just web-build` must run before `cargo install`. Without it the binary still works and serves a "dashboard is not built" page.

## Install the daemon

```bash
HOMEBASED_WEB_LISTEN=0.0.0.0:7677 homebased daemon install --dry-run
HOMEBASED_WEB_LISTEN=0.0.0.0:7677 homebased daemon install
homebased --json daemon status        # {"socket": "up", "in_flight": 0, "home": "...", "web": "http://0.0.0.0:7677"}
# without HOMEBASED_WEB_LISTEN, "web" is null and other machines cannot reach Fleet routes
```

On Praveen's machines, always preserve `HOMEBASED_WEB_LISTEN=0.0.0.0:7677` during install or reinstall. The `main:7677` dashboard depends on this LAN bind. Verify both `http://main:7677/` and `/v1/status` after installation.

The unit's `ExecStart` points at the binary that ran `install`, so run it as the installed `homebased`, not `target/debug/homebased`. Install is an idempotent apply. Run it from a shell where `codex`, `claude`, `grok`, `opencode`, and the project toolchains are on `PATH`: the installer bakes that `PATH` and the absolute agent paths (`HOMEBASED_CODEX`, `HOMEBASED_CLAUDE`, `HOMEBASED_GROK`, and `HOMEBASED_OPENCODE`) into the unit. Re-run it after `PATH` changes. An agent that is not on `PATH` at install time is silently left out of the unit; only the unit's `PATH` is left to find it later.

## Config file and host unit

The default config is `~/.config/homebased/config.toml`. It is optional; if it
is absent, Fleet is disabled. A path selected with `--config` or
`HOMEBASED_CONFIG` must exist and pass validation before install.

```bash
homebased --config /etc/homebased/config.toml --json config validate
homebased --config /etc/homebased/config.toml daemon install --dry-run
homebased --config /etc/homebased/config.toml daemon install
```

When you select an explicit config path, `daemon install` writes its absolute
path as `HOMEBASED_CONFIG` in the systemd unit or LaunchAgent. The daemon uses
that file after logout, host restart, or `homebased daemon restart`. If you
reinstall the unit, pass the same `--config` path or set `HOMEBASED_CONFIG`
again. Without an explicit path, the unit uses the default config path.

After a config edit, restart the daemon. Keep each machine name unique and
enable Fleet in the config on every machine that must join. See [fleet.md](fleet.md)
for the config shape and machine commands.

OpenCode v2 uses `opencode run --standalone`. Keep its credentials in the OpenCode user environment and install from the same user shell. Homebased passes the selected `provider/model#variant` value to OpenCode and gives the child its managed work permissions. Run the ignored CLI smoke check after an OpenCode upgrade:

```bash
HOMEBASED_OPENCODE="$HOME/.opencode/bin/opencode" just smoke-cli
```

- Linux: user unit `~/.config/systemd/user/homebased.service`, `KillMode=process`, `Restart=on-failure`. If install warns that lingering is off, run `loginctl enable-linger $USER` so the daemon survives logout.
- macOS: `~/Library/LaunchAgents/dev.praveen.homebased.plist` with `KeepAlive` and `AbandonProcessGroup`.

State directory: `--home`, else `HOMEBASED_HOME`, else `$XDG_STATE_HOME/homebased`, else `~/.local/state/homebased`. It holds `homebased.sqlite`, `homebased.sock`, `daemon.lock`, `tasks/<id>/`, and `callback-fallback.log`.

## Dashboard listener

`daemon serve` does not bind the HTTP listener unless `--web-listen` / `HOMEBASED_WEB_LISTEN` is a host:port. Fleet peers use this same listener:

```bash
homebased daemon serve                               # socket only; dashboard off
homebased daemon serve --web-listen code:7677        # host-based listener
HOMEBASED_WEB_LISTEN=0.0.0.0:9000 homebased daemon serve
HOMEBASED_WEB_LISTEN=100.x.y.z:7677 homebased daemon install   # Tailscale bind
HOMEBASED_WEB_LISTEN=0.0.0.0:7677 homebased daemon install   # bake a LAN bind into the host unit
```

`daemon install` copies `HOMEBASED_WEB_LISTEN` from the installing shell into the unit, next to `PATH`, the agent paths, and an explicit `HOMEBASED_CONFIG` path. It rejects an invalid listener or config. If the listener is unset, the unit does not start an HTTP listener, so other machines cannot reach Fleet routes. Re-run `install` to change the listener.

There is no application login or access token. Network reachability is the access boundary: any peer that can reach the dashboard can read task data and every regular file available to the daemon user through the device-wide file browser. A second content-origin port serves raw files (text, raster images, and fully active HTML inline; other types download). The dashboard and content origins do not grant CORS access to each other. Accepted `Host` values are `localhost`, single-label LAN names, mDNS names (`*.local`), numeric local and Tailscale addresses, the configured bind address, and Tailscale MagicDNS names (`*.ts.net`). Unexpected hosts are rejected.

The listener has no application login or access token. It exposes Fleet routes and the optional dashboard. Bind a non-loopback address only on a trusted network. A bind failure (busy port) is a warning: the daemon keeps serving the socket and `daemon status` reports `"web": null`.

## Upgrade

From a GitHub release (replaces the installed binary, then restarts the daemon and dashboard):

```bash
homebased --json update
homebased --json update --tag v0.2.0   # pin a tag
homebased --json update --dry-run      # tag, target, and path only
```

After rebuilding from this repository:

```bash
just web-build
cargo install --path /home/praveen/code/homebased
homebased --json daemon restart
```

`update` and `daemon restart` restart only `serve`. Running workers are separate processes that hold their own lock; they keep running, and the restarted daemon reconciles them and delivers their events. Do not use raw `systemctl restart`; the CLI path uses the host supervisor only when the installed unit's `--home` matches the selected state directory, and otherwise respawns the standalone daemon for that home.

## Stop and uninstall

```bash
homebased --json daemon stop            # exit 5 tasks_in_flight if anything is queued or running
homebased --json daemon stop --yes      # cancel in-flight tasks, wait for their events, then stop
homebased --json daemon uninstall       # same refusal rule; --yes cancels first
```

`stop`, `restart`, and `uninstall` match the selected state directory to the installed unit's `--home` before they control the host supervisor. For `stop` and `restart`, a matching unit uses `systemctl`/`launchctl`; any other unit state leaves the host service alone and uses the selected socket or standalone daemon path. `uninstall` refuses with `host_unit_home_mismatch` when the unit belongs to another home, and with `unit_invalid` when the unit's daemon invocation cannot be parsed.

`uninstall` removes the host unit only. The database and task directories stay. Confirm with the user before `--yes`; it cancels their tasks.

## Foreground run for debugging

```bash
homebased daemon serve --home /tmp/hb-test
```

Only one `serve` per home; a second exits 5 `daemon_already_running`. Use a separate `--home` for experiments so the real state is untouched. Lifecycle commands for that home do not stop or remove a host unit that points at a different home.
