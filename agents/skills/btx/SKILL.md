---
name: btx
description: Explore external repository source or fetch GitHub repository metadata with the btx CLI.
---

# btx

Clone or update repos for exploration using the `btx` CLI. Also fetches GitHub repository metadata so agents do not need a separate `curl` to the GitHub API.

## Usage

```bash
btx <repo> [options]    # clone/update
btx info <repo>         # GitHub metadata only (no clone)
```

**Arguments:**
- `<repo>`: `owner/repo`, HTTPS URL, SSH URL, or local path
- `btx info` only supports GitHub (`owner/repo` or `github.com` URL)

**Options (clone):**
- `--fresh` / `-f`: Force fresh clone (removes cached version)
- `--ref <ref>` / `-r <ref>`: Checkout specific branch, tag, or SHA
- `--full`: Clone complete history (default: single-branch)
- `--quiet` / `-q`: Suppress progress logs
- `--info` / `-i`: Also fetch GitHub repository metadata into a `github` object

## Output

### Clone

JSON with repo location. With `--info` on a GitHub source, includes `github`:

```json
{
  "path": "/Users/you/.cache/cmd/repos/github.com/owner/repo",
  "url": "https://github.com/owner/repo",
  "branch": "main",
  "updated_at": "2025-01-16T...",
  "stale": false,
  "github": {
    "full_name": "owner/repo",
    "owner": "owner",
    "owner_type": "Organization",
    "private": false,
    "archived": false,
    "fork": false,
    "default_branch": "main",
    "pushed_at": "2025-01-15T...",
    "description": "..."
  }
}
```

Without `--info`, the `github` field is omitted. If `--info` fails (network/auth), clone still succeeds and `github` is omitted (warning logged).

### `btx info`

Metadata only (hard-fails if not GitHub or the API call fails):

```json
{
  "url": "https://github.com/owner/repo",
  "github": {
    "full_name": "owner/repo",
    "owner": "owner",
    "owner_type": "Organization",
    "private": false,
    "archived": false,
    "fork": false,
    "default_branch": "main",
    "pushed_at": "2025-01-15T...",
    "description": "..."
  }
}
```

Auth for metadata uses `GITHUB_TOKEN`, `GH_TOKEN`, or `gh auth token` when available.

## Workflow

1. **Source only** - `btx <repo>`, parse `path`, explore with Glob/Grep/Read
2. **Metadata only** - `btx info <repo>` (ownership, archived, last push, …)
3. **Both** - `btx <repo> --info` (one call; no separate curl)
4. Prefer `btx` over `curl`/raw GitHub API for source inspection and for this repository metadata

## Examples

```bash
# GitHub shorthand
btx anthropics/claude-code

# Specific branch
btx tokio-rs/tokio --ref tokio-1.0.0

# Fresh clone
btx facebook/react --fresh

# Local path (validates and returns info)
btx ./my-project

# GitHub metadata only
btx info rust-lang/rust

# Clone + metadata
btx anthropics/claude-code --info
```
