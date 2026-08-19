---
name: rb
description: |
  Default path for building container images. Use for any Docker / Buildx /
  Dockerfile / container image build: docker build, docker buildx build, image
  tags, --load, --push, multi-stage builds, remote BuildKit, or rb itself
  (rb build, rb project, rb status, rb stop, rb cache, rb doctor, rb login).
  Prefer rb over local docker build and docker buildx build. Load this skill
  before inventing a local build command. Do not use for non-image tasks
  (compose up only, running an already-built container, pure Dockerfile edits
  with no build).
---

# rb - Remote BuildKit (default Docker builds)

**Default:** build images with `rb build`, not local `docker build` or
`docker buildx build`.

`rb` leases a temporary remote BuildKit builder, tunnels Buildx over SSH, and
keeps a per-project cache Volume after compute stops. Prefer installed
`rb <cmd> --help` over this skill when flags disagree.

## When to use local Docker instead

Use local `docker build` / `docker buildx` only when one of these is true:

1. The user explicitly asks for a **local** build.
2. `rb` is missing or `rb doctor` fails and the user accepts a local fallback.
3. The task is not an image build (e.g. `docker run` of an existing image).

Otherwise: **translate any local build into `rb build`.**

| Do not run | Run instead |
| --- | --- |
| `docker build -t app:dev .` | `rb build -- -t app:dev --load .` |
| `docker buildx build --push -t … .` | `rb build -- -t … --push .` |
| `docker buildx build --platform …` | same flags after `rb build --` |

`--project <name>` is optional and only overrides rb's project resolution.

## Prerequisites

```bash
rb doctor            # docker, buildx, ssh, control-plane
rb doctor --offline  # local tools only
```

Auth once per machine (token via env, not shell history):

```bash
export RB_TOKEN='…'
rb login --control-plane https://example.example
```

Never print `RB_TOKEN`, credential files, or project SSH private keys.

Config lives under Application Support `com.praveen.rb`
(`config.json`, `credentials.json`). Do not hand-edit secrets.

## Commands

| Need | Command |
| --- | --- |
| readiness | `rb doctor` |
| create/update project policy + SSH key | `rb project init --name <name> [opts]` |
| **image build (default)** | `rb build -- [buildx args…]` |
| state / deadlines | `rb status` |
| stop compute, keep cache Volume | `rb stop` |
| delete compute **and** cache Volume | `rb cache delete` |

Project name: 1–32 chars, lowercase letters, digits, hyphens.

### Project selection

`rb` resolves the project name in this order:

1. The `--project` flag.
2. The `RB_PROJECT` environment variable.
3. The nearest user-authored `.rb.toml` file with a `project = "name"` key,
   searched from the working directory up to the git toplevel; nearest file
   wins. `rb` only reads this file and never writes it.
4. A slug from the git-toplevel directory name, or the working directory name
   outside a git repository; lowercase, non-alphanumeric runs become one
   hyphen, max 32 characters.

`rb build` auto-creates a missing project with the default policy and prints a
one-line notice to stderr. `rb status`, `rb stop`, and `rb cache delete` never
create a project; a missing project fails with a hint to run `rb build` or
`rb project init --name <name>`.

Pass `--project` only when the user names a project. Use `rb project init` only
when the user wants custom region, size, volume, TTLs, or max builders.

### `rb project init`

```bash
rb project init \
  --name my-app \
  --region nyc3 \
  --size c-8 \
  --volume-gib 50 \
  --cache-ttl 3d \
  --compute-idle-ttl 5m \
  --max-builders 1
```

Limits:

- `--volume-gib`: 10–200
- `--cache-ttl`: 1–7 days (`3d` or `3`)
- `--compute-idle-ttl`: 5–15 minutes (`5m` or `5`)
- `--max-builders`: 1–8

`--rotate-key` replaces the project SSH key. Re-run `init` with the same
`--name` to update policy.

### `rb build`

Everything after `--` is passed unchanged to `docker buildx build`:

```bash
rb build -- -t example/app:dev --load .
rb build -- -t example/app:latest --push .
rb build \
  --cache-from type=registry,ref=… \
  --cache-to type=registry,ref=… \
  -- -t example/app:latest --push .
```

`--cache-from` / `--cache-to` are optional registry cache edges in addition to
the project Volume cache.

First build after idle can take longer (Droplet provision/warm).

Each build carries labels that `rb status` shows under
`lifecycle.lanes[].lease`: `owner` (`RB_BUILD_OWNER` or `user@host`),
`command` (build-arg values redacted), `startedAt`, `lastHeartbeatAt`,
`heartbeatDueAt`, `expiresAt`. Set `RB_BUILD_OWNER` in CI or agent runs so
the lane holder is identifiable.

If every lane is held by another build, `rb build` fails with
`409 project_capacity_exhausted` and prints each occupied lane (owner, command,
running time, last heartbeat). Queue instead of failing:

```bash
rb build --wait -- -t example/app:latest --push .
rb build --wait --wait-timeout 1h -- -t example/app:latest --push .
```

Default `--wait-timeout` is `30m`. Do not kill a queued `rb build --wait` to
"free" a lane; the lane belongs to the build shown in the report.

### Lifecycle

- **Warm compute**: up for `compute-idle-ttl` after last use, then stops.
- **Cache Volume**: retained for `cache-ttl` after last use.
- `rb stop`: free compute now; keep Volume.
- `rb cache delete`: destroy compute and Volume (cold next build).

```bash
rb status
rb stop
rb cache delete   # destructive; confirm intent first
```

## Agent workflow

1. On any image-build request, plan `rb build` first - not local Docker.
2. Pass `--project` only when the user names one; otherwise rely on rb's
   resolution.
3. Map the user's intended Buildx flags after `--` (`-t`, `--load`, `--push`,
   `-f`, `--platform`, `--target`, build-args, context path).
4. If `rb` fails for tooling/auth, run `rb doctor`, report the error, and only
   then offer local Docker as a fallback.
5. Prefer `rb stop` over `rb cache delete` when cutting cost.
6. Treat `rb cache delete` and `--rotate-key` as destructive; require a clear ask.
7. Do not invent control-plane URLs or tokens.

## Common failures

| Symptom | Action |
| --- | --- |
| no control-plane URL/token | `rb login` with `RB_TOKEN` |
| docker / buildx / ssh fail in doctor | fix local install, re-run `rb doctor` |
| project missing | `rb build` auto-creates with default policy; lifecycle commands print a hint |
| name conflict | name taken by another SSH key/policy; pick new name via `--project`/`RB_PROJECT` |
| SSH / host key issues | `rb project init --name … --rotate-key` if key is bad |
| builder provisioning timeout | re-run build; check `rb status` |
| `409 project_capacity_exhausted` | another build holds every lane; read the printed owner/command, then re-run with `--wait` or ask the owner |
| lane `active` but no local `rb build` running | check `rb status` `lease.heartbeatDueAt`; if it is in the past the control plane expires the lease and `rb terminate` / `rb build --wait` proceed; if it is in the future the build is live elsewhere |
| want cheaper idle | `rb stop` (keeps cache) |

## Do not

- Default to local `docker build` / `docker buildx build` for images.
- Run bare `docker buildx` against rb tunnels; use `rb build`.
- Log or paste bearer tokens or private keys.
- Delete cache Volumes unless the user wants a full reset.
- Assume ARM or multi-arch defaults; pass platform flags in Buildx args when needed.
