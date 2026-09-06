---
name: rb
description: Build container images and manage rb builders. Use for Docker/Buildx builds or rb operations, not image runs, compose-only tasks, or Dockerfile-only edits.
---

# rb - Remote BuildKit

Default to `rb build` for container images. Use local Docker only when the user
requests a local build, or when `rb` is missing or fails readiness checks and the
user accepts the fallback. Running an existing image is outside this workflow.

## Build

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
