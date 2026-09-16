---
name: fleet
description: Choose and use Praveen's machines for remote coding agents, Rust builds, AI work, and GPU training. Use for the Mac mini, ai5090.local, code.local, training.local, or homelab machine access and workload placement. Do not use for ordinary local coding that needs no machine choice.
---

# Fleet

Use the Mac mini to coordinate work. Prefer `code.local` for remote coding agents, Rust builds, and AI development. Run training sessions on `training.local`. Follow an explicit machine choice from the user.

## Machines

| Machine | Address / SSH user | Role |
| --- | --- | --- |
| Mac mini (`Praveens-Mac-mini.local`) | Local macOS, `praveen` | Main workstation; coordinate remote work, run local agents, and do macOS-specific work |
| `ai5090.local` | `192.168.1.20`, `praveen` | Ubuntu Incus host; RTX 5090 with 32 GB VRAM; host administration only |
| `code.local` | `192.168.1.18`, `praveen` | Incus container on ai5090; coding agents, compilers, source, AI development, and image builds |
| `training.local` | `192.168.1.19`, `praveen` | Incus container on ai5090; training jobs, GPU Docker workloads, datasets, and checkpoints |
| `server.local` | `192.168.1.50`, `root` | Separate Proxmox host for the service containers below |
| `media.local` | `192.168.1.11`, `root` | Plex, Jellyfin, downloads, media automation, and the home dashboard |
| `bitcoin.local` | `192.168.1.10`, `root` | Bitcoin Knots, electrs, and mempool services |
| `unifi.local` | `192.168.1.12`, `root` | UniFi Network application and its database |
| `misc.local` | `192.168.1.14`, `root` | General service container; currently a Docker registry |

These are configured roles and addresses, not proof that a machine is online. Do not assume that the current shell is on the Mac mini when this skill is used on another machine.

## Choose the instructions

- To start, continue, or inspect an agent or remote command, read [sessions.md](references/sessions.md). Agents started from the Mac mini must run in named tmux sessions on the machine that runs the work, so Praveen can attach. All agent work on ai5090 and its containers must also run inside tmux.
- For training, GPU use, shared data, or image transfer, also read [compute.md](references/compute.md).
- For service administration, read only the relevant files in `~/code/homelab`: `main.tf`, `server/etc/avahi/hosts`, `stacks/<name>/`, and `justfile`. Service stacks use `/opt/stack` on their respective containers. Do not use these machines as general coding workers.

## Machine boundaries

- Keep `ai5090.local` small: use it only for Incus, storage, NVIDIA driver work, and recovery. Do not install agents, Docker, Nix, or language toolchains on the host.
- `code` and `training` are Incus system containers, not Docker containers. Each has its own nested Docker daemon.
- The two containers share one physical GPU and have no CPU, memory, or GPU limits. Check active work before starting a large job; do not interrupt another job to make room.
- A request to code or train does not authorize host reconfiguration, stack deployment, or recreation of containers.
- If the selected machine is unavailable, report the connection failure. Do not silently move a large job to the Mac mini or another service machine.

## Source of machine details

Use `~/code/homelab/ai5090/README.md` and its `scripts/` directory for compute setup details. Use `~/code/homelab/main.tf`, the Avahi hosts file, and stack definitions for the other machines. Locate the homelab checkout if it is not at the expected path. Read only the relevant configuration; do not load tokens, `.env` files, or Terraform state to discover machine roles.

Check the current files and live state before a task that depends on a driver version, installed tool, available resource, or network address. Setup scripts describe provisioning; do not rerun them for routine coding or training.
