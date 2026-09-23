# GPU work, shared files, and images

Use [sessions.md](sessions.md) for persistent or interactive execution. Run short resource checks directly.

## GPU runs

Check `nvidia-smi`, active jobs, RAM, and disk space before a large run. Test GPU access in `code` and the Docker image before a long run. GPU Docker workloads use `--gpus all`.

Incus limits `code` to 28 GiB of RAM and 28 GiB of swap. Also cap the nested Docker job, so that one job cannot use the full container limit. `ai5090` has 30.2 GiB of usable RAM. Leave several GiB for the kernel, SSH, Incus, and other work in `code`. For a single training job, pass both flags and keep them equal so the container cannot fill host swap:

```bash
docker run --rm --gpus all \
    --memory=22g \
    --memory-swap=22g \
    my-project:<version>
```

If `--memory-swap` is omitted, Docker allows extra swap equal to `--memory`. That can stall SSH. Do not pass `--oom-kill-disable`. Lower `--memory` when `free -h` shows less headroom. A `nvidia-smi` smoke test does not need these flags. Consult `~/code/homelab/ai5090/README.md` for the same rule.

Use a distinct run directory. Keep the configuration, source revision, image tag or ID, logs, and checkpoint path together. Report the run directory with the tmux attach command. Do not overwrite another run's checkpoints.

The host provides the NVIDIA kernel driver. The `code` container provides the user-space libraries and NVIDIA Container Toolkit. For driver or nested Docker faults, consult the homelab README before changing setup. Do not change the host driver to fix an application dependency without that check.

## Shared storage

`code` mounts an Incus custom volume at `/shared`:

| Path | Use |
| --- | --- |
| `/shared/datasets` | Reusable training data |
| `/shared/models` | Model files |
| `/shared/checkpoints` | Checkpoints, separated by project and run |
| `/shared/artifacts` | Shared outputs |

The host and Mac mini do not mount `/shared`. Create or download large data into `/shared` from `code` when possible. The volume is separate from the container root, so it survives container recreation. Keep ordinary source checkouts in the container home directory.

Transfer host files through Incus and set container ownership:

```bash
incus file push --uid 1000 --gid 1000 /host/path/file code/shared/artifacts/file
```

For many small files, use one tar stream to avoid one Incus API call per file:

```bash
tar -C /host/path -cf - directory \
    | incus exec code --user 1000 --group 1000 -- \
        tar -C /shared/artifacts -xf -
```

Use an explicit transfer for Mac files. Never write directly under `/srv/agents/incus` or mount the custom volume on the host to transfer files; use the container's storage access.

## Docker images

Use the `rb` skill before choosing an image build command. Build images, including training images, in `code` from the required source revision. Load internal training images into the local daemon. Publish public images to Docker Hub.

Use an immutable version tag or image ID for each training run, not only `latest`. `/shared` does not hold Docker images; they stay in the nested Docker store in `code`.

Use `docker system df` to check usage. Do not remove tagged training images that the user still needs. Consult the homelab README for cache policy and maintenance.
