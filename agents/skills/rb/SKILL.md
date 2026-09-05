---
name: rb
description: Build container images or manage remote builders with rb. Use for Docker/Buildx image builds and rb operations, not running images, compose-only work, or Dockerfile-only edits.
---

# rb - Remote BuildKit

Use `rb build` as the default path for container image builds. It leases a
temporary remote BuildKit builder, tunnels Buildx over SSH, and keeps a
per-project cache Volume after compute stops. Prefer installed `rb <cmd>
--help` when its flags differ from this skill.

## Choose the build path

Use local `docker build` or `docker buildx build` only when:

1. the user explicitly asks for a local build
2. `rb` is missing or `rb doctor` fails and the user accepts a local fallback
3. the task is not an image build, such as running an existing image

Otherwise, translate the image build to `rb build`:

| Do not run | Run instead |
| --- | --- |
| `docker build -t app:dev .` | `rb build -- -t app:dev --load .` |
| `docker buildx build --push -t … .` | `rb build -- -t … --push .` |
| `docker buildx build --platform …` | same flags after `rb build --` |

Do not run bare `docker buildx` against an rb tunnel. Everything after `--` is
passed unchanged to `docker buildx build`; forward the user's intended flags,
including `-t`, `--load`, `--push`, `-f`, `--platform`, `--target`, build args,
and context path.

## Basic commands

```bash
rb doctor
rb doctor --offline
rb build -- -t example/app:dev --load .
rb build -- -t example/app:latest --push .
rb status
rb stop
rb cache delete
```

`rb stop` releases compute and keeps the cache Volume. `rb cache delete`
destroys compute and the cache Volume, so treat it as destructive and run it
only after a clear request. Read [project-management.md](references/project-management.md)
for authentication, project policy, and lifecycle administration.

## Project selection

`rb` resolves the project name in this order:

1. the `--project` flag
2. the `RB_PROJECT` environment variable
3. the nearest user-authored `.rb.toml` with `project = "name"`, searched from
   the working directory up to the git toplevel; the nearest file wins
4. a slug from the git-toplevel directory, or the working directory outside a
   git repository, with lowercase non-alphanumeric runs collapsed to one
   hyphen and a maximum length of 32 characters

`rb` only reads `.rb.toml`; it never writes it. `rb build` auto-creates a
missing project with the default policy and prints a one-line stderr notice.
`rb status`, `rb stop`, and `rb cache delete` never create a project and show
a hint to run `rb build` or `rb project init --name <name>` when it is missing.
Pass `--project` only when the user names a project. Use `rb project init` for
custom region, size, volume, TTL, or builder limits; read the project reference
before changing policy or keys.

## Constraints

- Keep `RB_TOKEN` in the environment, not shell history. Never print it,
  credential files, or project SSH private keys
- Do not hand-edit secrets in Application Support `com.praveen.rb`
- Do not invent control-plane URLs or tokens
- Treat `--rotate-key` as destructive and require a clear request
- Prefer `rb stop` when reducing cost and keeping the cache

Read [registry-cache.md](references/registry-cache.md) for registry choice and
managed cache behavior. Read [troubleshooting.md](references/troubleshooting.md)
for readiness failures, occupied lanes, queueing, and recovery actions.

## Platform assumptions

Do not assume ARM or multi-arch defaults. Pass platform flags in the Buildx
arguments when the target requires them. The first build after idle can take
longer while the Droplet provisions or warms.
