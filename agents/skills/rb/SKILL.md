---
name: rb
description: Route container image builds between local Docker and rb, and manage rb builders. Use for Docker/Buildx builds or rb operations, not image runs, compose-only tasks, or Dockerfile-only edits.
---

# rb - Container image builds

Choose the build location before choosing the command:

- On `code` or another intended Linux build machine with Docker,
  use its local Docker daemon
- On `ai5090`, enter the selected Incus container and use that container's
  Docker daemon; do not install Docker on the host
- On the Mac, default to a named tmux session on `code`, including for GPU
  training images; read the `fleet` skill for remote session and checkout
  handling
- Use AWS through `rb build` only when the task needs parallel builds beyond the
  available local builder, or when the user explicitly requests AWS

Running an existing image is outside this workflow.

## Local build

Use Buildx and forward the required BuildKit flags directly:

```bash
docker buildx build -t example/app:dev --load .
docker buildx build -t example/app:latest --push .
```

Do not send a single build to AWS only because the current shell is on the Mac.
Move the work to `code` first. Keep public image publishing on Docker Hub.

On `code`, keep BuildKit garbage collection enabled with a 50 GB cache limit.
Inspect `docker system df` after an unusually large build batch. For immediate
cleanup, remove only old unused build cache and dangling images:

```bash
docker builder prune --force --filter until=168h --reserved-space 50GB
docker image prune --force --filter until=168h
```

Do not run `docker system prune --volumes` or `docker image prune --all` without
a clear request because unused volumes and tagged images can contain work that
must be retained.

## Parallel AWS build

Forward the intended Buildx flags after `--`; do not run bare Buildx against an
rb tunnel. Prefer installed `rb <command> --help` if flags differ from this skill.

```bash
rb build -- -t example/app:dev --load .
rb build -- -t example/app:latest --push .
```

Pass `--project` only when the user names one. Otherwise use rb's project
resolution. A build creates a missing project with default policy; use
`rb project init` only for requested custom policy. Do not assume ARM or
multi-architecture defaults; pass the required platform in the Buildx arguments.
The first build after idle can take longer while remote compute starts.

## Read when needed

- [Project management](references/project-management.md): authentication, project
  selection, custom policy, SSH keys, status, or compute/cache lifecycle
- [Registry and cache](references/registry-cache.md): choosing a push registry,
  registry credentials, or configuring managed or manual BuildKit caches
- [Troubleshooting](references/troubleshooting.md): readiness failures, occupied
  lanes, queueing, or recovery

## Constraints

- Keep `RB_TOKEN` in the environment, not shell history. Never print tokens,
  credential files, or project SSH private keys, and do not hand-edit secrets
- Do not invent control-plane URLs or tokens
- Publish public images to Docker Hub
- Prefer `rb stop` to release compute while retaining cache. `rb cache delete`
  destroys both compute and cache; it and `--rotate-key` require a clear request
