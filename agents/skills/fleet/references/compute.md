# GPU work, shared files, and images

Use [sessions.md](sessions.md) for all execution, including resource checks.

## GPU runs

Check `nvidia-smi`, active jobs, RAM, and disk space before a large run. Test GPU access in the selected Incus container and Docker image before a long run. GPU Docker workloads use `--gpus all`.

Use a distinct run directory. Keep the configuration, source revision, image tag or ID, logs, and checkpoint path together. Report the run directory with the tmux attach command. Do not overwrite another run's checkpoints.

The host provides the NVIDIA kernel driver. The containers provide the user-space libraries and NVIDIA Container Toolkit. For driver or nested Docker faults, consult the homelab README before changing setup. Do not change the host driver to fix an application dependency without that check.

## Shared storage

Both containers mount the same Incus volume at `/shared`:

| Path | Use |
| --- | --- |
| `/shared/datasets` | Reusable training data |
| `/shared/models` | Model files |
| `/shared/checkpoints` | Checkpoints, separated by project and run |
| `/shared/artifacts` | Shared outputs |

The host and Mac mini do not mount `/shared`. Create or download large data into `/shared` from either container when possible. Files in `/shared` are visible in both containers. Keep ordinary source checkouts in the execution container unless shared source is needed.

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

Use the `rb` skill before choosing an image build command. Build non-training images in `code`; build training images directly in `training`, from the required source revision. Load internal training images into the local daemon. Publish public images to Docker Hub.

Use an immutable version tag or image ID for each training run, not only `latest`. The containers have separate image stores: `--load` in one does not load the other, and `/shared` does not share Docker images.

Use `docker system df` to check usage. Do not remove tagged training images that the user still needs. Consult the homelab README for cache policy and maintenance.
