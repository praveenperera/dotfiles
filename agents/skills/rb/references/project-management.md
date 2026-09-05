# rb project management

Read this reference when readiness, authentication, project policy, SSH keys,
project selection, or cache lifecycle administration is in scope. The root skill
covers the normal build path.

## Readiness and authentication

Use these checks to diagnose readiness:

```bash
rb doctor            # docker, buildx, ssh, and control plane
rb doctor --offline  # local tools only
```

Authenticate once per machine with the token in the environment, not shell
history:

```bash
export RB_TOKEN='…'
rb login --control-plane https://example.example
```

Never print `RB_TOKEN`, credential files, or project SSH private keys. Config
lives under Application Support `com.praveen.rb` (`config.json` and
`credentials.json`); do not hand-edit secrets.

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
Pass `--project` only when the user names a project.

## Project policy

Use `rb project init` only when the user wants a custom region, size, volume,
TTL, or builder limit:

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

Project names have 1–32 lowercase letters, digits, or hyphens. Limits are:

- `--volume-gib`: 10–200
- `--cache-ttl`: 1–7 days (`3d` or `3`)
- `--compute-idle-ttl`: 5–15 minutes (`5m` or `5`)
- `--max-builders`: 1–8

Re-run `init` with the same `--name` to update policy. `--rotate-key` replaces
the project SSH key; treat it as destructive and run it only after a clear
request.

## Compute and cache lifecycle

- Warm compute stays up for `compute-idle-ttl` after last use, then stops
- The cache Volume is retained for `cache-ttl` after last use
- `rb stop` frees compute now and keeps the Volume
- `rb cache delete` destroys compute and the Volume, causing a cold next build

Prefer `rb stop` when cutting cost. Do not delete a cache Volume unless the user
wants a full reset.
