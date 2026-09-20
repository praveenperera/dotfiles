---
name: fleet
description: Choose and use Praveen's machines for remote coding agents, builds, and GPU training. Use for Mac mini, ai5090.local, code.local, training.local, or homelab access and workload placement. Do not use for ordinary local coding that needs no machine choice.
---

# Fleet

Use the Mac mini to coordinate work. Use `code.local` for remote development and `training.local` for training and GPU workloads. Follow an explicit machine choice from the user, within the machine boundaries below.

## Machines

| Machine | Address / SSH user | Role |
| --- | --- | --- |
| Mac mini (`Praveens-Mac-mini.local`) | Local macOS, `praveen` | Main workstation, local agents, and macOS work |
| `ai5090.local` | `192.168.1.20`, `praveen` | Ubuntu Incus host; host administration only |
| `code.local` | `192.168.1.18`, `praveen` | Incus container on ai5090; coding agents, compilation, AI development, and non-training image builds |
| `training.local` | `192.168.1.19`, `praveen` | Incus container on ai5090; training image builds, training jobs, GPU workloads, datasets, and checkpoints |
| `server.local` | `192.168.1.50`, `root` | Separate Proxmox host for the service containers below |
| `media.local` | `192.168.1.11`, `root` | Plex, Jellyfin, downloads, media automation, and home dashboard |
| `bitcoin.local` | `192.168.1.10`, `root` | Bitcoin Knots, electrs, and mempool |
| `unifi.local` | `192.168.1.12`, `root` | UniFi Network and its database |
| `misc.local` | `192.168.1.14`, `root` | General services; currently a Docker registry |

These are configured roles and addresses, not proof that a machine is online. Confirm the current machine; do not assume the shell is on the Mac mini.

## Boundaries

- Keep `ai5090` small. Use it only for Incus, storage, NVIDIA driver work, and recovery. Do not install coding agents, Docker, or language toolchains on the host. Do not use Nix.
- `code` and `training` are Incus system containers. Each has its own nested Docker daemon and Docker/containerd stores. Never share `/var/lib/docker` or containerd stores.
- Both containers share one RTX 5090 with 32 GB VRAM. Incus has no CPU, RAM, or GPU limits. Cap nested Docker jobs with `--memory` and `--memory-swap` as in [compute.md](references/compute.md). Check active work before a large job; do not interrupt another job to make room.
- Do not use service machines as coding workers. Coding or training does not authorize host reconfiguration, stack deployment, or container recreation.
- If the selected machine is unavailable, report the connection failure. Do not silently move the job elsewhere.

## Task instructions

- For agents and work that must survive a disconnect, remain interactive, or support a later handoff, read [sessions.md](references/sessions.md) and use a named tmux session on the execution machine. Run short, non-interactive checks and routine commands directly without creating a session.
- For GPU work, shared files, and image builds, also read [compute.md](references/compute.md).
- For compute administration, read `~/code/homelab/ai5090/README.md` and the relevant setup script in `ai5090/scripts/`. Do not rerun provisioning for routine work.
- For service administration, read the relevant homelab configuration: `main.tf`, `server/etc/avahi/hosts`, `stacks/<name>/`, or `justfile`. Service stacks use `/opt/stack` in their containers.

## Storage reuse

- Reuse the project's established checkout pattern and keep one writer per checkout. For long-lived agent slots such as `cove-wk1` through `cove-wk4`, use independent clones with private build and scratch directories. Use worktrees only where the project already prefers them.
- On `code` or `training`, use `cp -a --reflink=always -- SOURCE NEW_DESTINATION` for large independent writable copies of fixtures, datasets, models, checkpoints, or SDKs. Run the copy inside the owning container; use `/shared` for data both containers need.
- Keep pnpm `packageImportMethod: auto`; it tries a CoW clone, then a hard link, and then a full copy. Use `sccache` for Rust compilation when the project configures it.
- For managed storage, copy conditions, and optional Rust build seeds, read **Reflink workflows** in `ai5090/README.md`.

Locate the homelab checkout if it is elsewhere. Check current configuration and live state when a task depends on an address, driver, installed tool, or available resource. Do not read tokens, `.env` files, or Terraform state to discover machine roles.
