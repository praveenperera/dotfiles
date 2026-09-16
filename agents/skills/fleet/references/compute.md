# GPU work and training

Read [sessions.md](sessions.md) before execution. Run the following checks and workloads inside a named tmux session on the selected machine.

## Coding versus training

- Use `code.local` for Rust compilation, coding agents, AI development, and non-training container image builds.
- Use `training.local` for training image builds, long training sessions, and GPU Docker workloads. Connect as `praveen`.
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

The `ai5090` host and Mac mini do not mount `/shared`. Prefer creating or downloading large data directly into `/shared` from `code` or `training`.

For a file that is already on the host, push it through either container and set container ownership explicitly:

```bash
incus file push --uid 1000 --gid 1000 \
    /host/path/file \
    code/shared/artifacts/file
```

For a directory with many small files, use one uncompressed tar stream to avoid a separate Incus API operation for every file:

```bash
tar -C /host/path -cf - directory \
    | incus exec code --user 1000 --group 1000 -- \
        tar -C /shared/artifacts -xf -
```

Files written through `code` are immediately visible in `training`. Do not write directly under `/srv/agents/incus` or permanently mount the custom volume on the host; these paths bypass the intended Incus ownership and storage boundary.

Use an explicit transfer when files from the Mac mini are needed. Keep ordinary source checkouts in the selected container's workspace unless the task needs shared source. The containers have separate root filesystems and Docker stores; never share `/var/lib/docker` between them.

## Docker images

The `code` and `training` containers have separate Docker daemons and image stores. An image built with `--load` in one container is not available in the other. Do not share `/var/lib/docker` between them.

Use the `rb` skill before choosing an image build command. Build an internal training image directly in `training` with an explicit version tag:

```bash
docker buildx build \
    --tag <project>:<version> \
    --load .
```

Run that local image in `training`:

```bash
docker run --gpus all <project>:<version>
```

Keep the source checkout in `training`, or check out the same Git revision there before the build. `/shared` can carry source archives and other ordinary files, but it does not make Docker images visible across daemons. Use an immutable version tag or image ID for training runs; do not rely only on `latest`. Public images go to Docker Hub.

BuildKit garbage collection on `training` keeps up to 50 GB of cache. Check usage with `docker system df`; do not remove tagged training images unless the user no longer needs them.

GPU Docker workloads use `--gpus all`. Check GPU access in the container and in the chosen image before a long run. Do not install a new host driver to fix an application dependency without first checking the homelab setup.

## Host details when needed

The host provides the NVIDIA kernel driver. Incus passes the GPU to both containers, which have matching user-space libraries and NVIDIA Container Toolkit. The documented setup disables Incus `nvidia.runtime` and sets `nvidia-container-cli.no-cgroups = true` for nested Docker. These are setup constraints, not changes to apply during each run.

Host storage is rooted at `/srv/agents`, with the Incus `agents` pool at `/srv/agents/incus`. For driver, storage, or recovery work, read `~/code/homelab/ai5090/README.md` and only the relevant setup script. Confirm current state before making changes; normal development stays in `code` and training stays in `training`.
