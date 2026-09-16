# GPU work and training

Read [sessions.md](sessions.md) before execution. Run the following checks and workloads inside a named tmux session on the selected machine.

## Coding versus training

- Use `code.local` for Rust compilation, coding agents, AI development, and container image builds.
- Use `training.local` for long training sessions and GPU Docker workloads. Connect as `praveen`.
- Both containers use the same RTX 5090 with 32 GB VRAM. They do not have separate GPUs or reserved resource shares. Check `nvidia-smi`, active jobs, memory, and disk space before a large run.
- Keep a training run's configuration, source revision, image tag or digest, logs, and checkpoint path together. Use a distinct run directory and report it with the tmux attach command. Do not overwrite another run's checkpoints.

## Shared storage

Both containers mount the same Incus volume at `/shared`:

| Path | Use |
| --- | --- |
| `/shared/datasets` | Reusable training data |
| `/shared/models` | Model files |
| `/shared/checkpoints` | Training checkpoints, separated by project and run |
| `/shared/artifacts` | Outputs that must be available in both containers |

The Mac mini does not automatically have this mount. Use an explicit transfer when local files are needed. Keep ordinary source checkouts in the selected container's workspace unless the task needs shared source. The containers have separate root filesystems and Docker stores; never share `/var/lib/docker` between them.

## Docker images

The private compute registry is `10.77.0.19:5000`, on `training` through the Incus private network. `code` uses `10.77.0.18` on that network. The registry is not exposed on the home LAN; do not assume the Mac mini can reach it. It is separate from the registry on `misc.local`.

Use the `rb` skill before choosing an image build command. Build in `code`, publish the image to the private compute registry when it is for internal training, then pull and run it in `training`. Use an explicit version tag or digest to identify the training image. Public images go to Docker Hub.

GPU Docker workloads use `--gpus all`. Check GPU access in the container and in the chosen image before a long run. Do not install a new host driver to fix an application dependency without first checking the homelab setup.

## Host details when needed

The host provides the NVIDIA kernel driver. Incus passes the GPU to both containers, which have matching user-space libraries and NVIDIA Container Toolkit. The documented setup disables Incus `nvidia.runtime` and sets `nvidia-container-cli.no-cgroups = true` for nested Docker. These are setup constraints, not changes to apply during each run.

Host storage is rooted at `/srv/agents`, with the Incus `agents` pool at `/srv/agents/incus`. For driver, storage, or recovery work, read `~/code/homelab/ai5090/README.md` and only the relevant setup script. Confirm current state before making changes; normal development stays in `code` and training stays in `training`.
