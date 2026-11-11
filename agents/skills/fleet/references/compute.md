# GPU work, shared files, and images

Run long commands as homebased tasks and all GPU work through the homebased resource queue, as the `homebased` skill says. Use [sessions.md](sessions.md) only for interactive sessions and hand-offs. Run short resource checks directly.

## GPU runs

Submit every GPU run through the resource queue, smoke tests included, such as an `nvidia-smi` check in the image before a long run. Build the image first as a CPU homebased task, and chain the GPU job on it with `homebased resource job submit --after <build-task-id>`, so the job does not hold the GPU during the build. Check active jobs, RAM, and disk space before a large run. GPU Docker workloads use `--gpus all`.

`ai5090` has about 61 GiB of usable RAM. Incus limits `code` to that minus 6 GiB (about 55 GiB) and 28 GiB of swap. Submit non-Docker jobs through Homebased; `homebased.service` caps them together at the container's memory minus 8 GiB with 8 GiB of swap, and a job over the cap is OOM-killed. Check the live values with `free -h` and `systemctl --user show homebased.service -p MemoryMax -p MemoryCurrent`.

Docker containers run under the `code` Docker daemon, outside `homebased.service`, so its cap does not reach them even when a Homebased task starts them. Cap each nested Docker job, so that one job cannot use the full container limit. Leave several GiB for the kernel, SSH, Incus, and other work in `code`. For a single training job, pass both flags and keep them equal so the container cannot fill host swap:

```bash
docker run --rm --gpus all \
    --memory=32g \
    --memory-swap=32g \
    my-project:<version>
```

If `--memory-swap` is omitted, Docker allows extra swap equal to `--memory`. That can stall SSH. Do not pass `--oom-kill-disable`. Lower `--memory` when `free -h` shows less headroom. A `nvidia-smi` smoke test does not need these flags. Consult `~/code/homelab/ai5090/README.md` for the same rule.

Use a distinct run directory. Keep the configuration, source revision, image tag or ID, logs, and checkpoint path together. Report the run directory with the homebased task or job id and its log path. `homebased --json task show <task-id>` gives the path as `output_log`; for a queue job, take the run's task id from `homebased --json resource job show <job>`. Do not overwrite another run's checkpoints.

The host provides the NVIDIA kernel driver. The `code` container provides the user-space libraries and NVIDIA Container Toolkit. For driver or nested Docker faults, consult the homelab README before changing setup. Do not change the host driver to fix an application dependency without that check.

## Shared storage

`code` keeps active data in `/shared` on the Lexar and finished, read-mostly data in `/data` on the Kingston. Read [storage.md](storage.md) for the layout, where to write new data, and how to move finished data.

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
