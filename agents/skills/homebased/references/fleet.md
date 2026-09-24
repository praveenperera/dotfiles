# Configure Fleet and find machines

## Config file

The default file is `~/.config/homebased/config.toml`. It is optional. If it
does not exist, Fleet is disabled. Select another file with the global
`--config <path>` flag or `HOMEBASED_CONFIG`. An explicitly selected file must
exist and pass validation. The daemon reads the file at startup.

Fleet is off unless `fleet.enabled = true`. This example uses the current
config keys:

```toml
[fleet]
enabled = true
machine_name = "code"

[fleet.discovery]
mdns = true
tailscale = true
tailscale_port = 7677

[[fleet.machines]]
address = "http://main:7677"

[[fleet.machines]]
address = "http://training:7677"
```

`machine_name` is optional; Homebased uses the host name when it is absent.
Choose a unique name for each installation. DNS-SD/mDNS is on by default.
Tailscale discovery is off by default. Its port defaults to `7677`. Each
configured peer address is an HTTP base URL. HTTPS, paths, query strings, and
user information are not valid address parts. If no port is present, Homebased
uses `7677`. The listener accepts single-label LAN names, `.local` names,
`.ts.net` names, LAN or tailnet IP addresses, and its bind IP in the HTTP
`Host` header. It rejects other public DNS names.

Every Fleet machine needs a reachable `HOMEBASED_WEB_LISTEN` address. This
HTTP listener serves Fleet requests and the optional dashboard. Fleet has no
authentication. A reachable peer can use Fleet routes and the dashboard file
browser, so use a trusted LAN or tailnet. The daemon still works for local
tasks when Fleet is disabled or its TCP listener is off.

Validate config and load changes with a daemon restart:

```bash
homebased --json config validate
homebased daemon restart
```

`config validate --json` prints the chosen path, whether the file exists, the
machine-name source, and the effective Fleet settings. It exits with
`config_invalid` when an explicit file is missing or the TOML is not valid.

The config file is the source for configured addresses. Fleet UUIDs, cached
discovery data, and addresses added by `fleet add` are stored in Homebased
state. `fleet add` does not edit the config file.

## Machine commands

All `fleet` commands use the local daemon socket. `fleet machines` works while
Fleet is disabled and returns the local machine with an empty peer list. The
other commands require Fleet to be enabled.

```bash
homebased --json fleet machines
homebased --json fleet discover
homebased --json fleet probe code
homebased --json fleet probe http://training:7677
homebased --json fleet add http://training:7677
homebased --json fleet remove http://training:7677
homebased --json fleet remove <machine-uuid>
```

- `machines` shows the local identity, known peers, and addresses that have
  not yet been verified.
- `discover` runs a probe round over known addresses and returns the resulting
  inventory.
- `probe` accepts a known machine name, machine UUID, or HTTP address. A direct
  address probe reports the identity and protocol compatibility that answered.
- `add` saves an explicit address in the peer directory and starts a probe.
- `remove <http-address>` removes that explicit address. `remove <name>` or
  `remove <machine-uuid>` forgets the known machine and its explicit addresses.

A configured address is not removed by `fleet remove`; it remains in the
config file and can bind to the machine again. Remove it from the file and
restart the daemon to stop managing it.

Machine UUIDs identify installations, not addresses. Do not copy a Homebased
state directory when cloning a machine. A stale address cannot redirect a
request to a different machine UUID. Duplicate live UUIDs and duplicate live
names block routing until they are resolved.

## Remote task routes

Submit through the local daemon that owns the Codex thread. Add a machine name
to the submit JSON to run the child on that Fleet machine. The origin keeps the
thread, request mapping, callback context, and event delivery. The executor
keeps the child process, logs, reports, and evidence. See [submit.md](submit.md)
for the spec, dry run, request UUID, and callback environment rules.

Use `task show`, `task log`, and `task cancel` through the local daemon. `show`
and `log` find a task by its full UUID in the known Fleet. `cancel` uses the
saved executor identity and stores a local cancellation request before it
returns. See [inspect.md](inspect.md) for offline state and cancellation
results.

The Fleet protocol uses plain HTTP and has no authentication. Do not route it
over a network that you do not trust.
