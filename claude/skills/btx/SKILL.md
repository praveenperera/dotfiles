---
name: btx
aliases:
  - better-context
description: |
  Clone and explore external codebases using the btx CLI. Use this skill when:
  - User wants to explore a GitHub repository (e.g., "explore anthropics/claude-code")
  - User asks how a library/framework works and you need to read its source
  - User wants to understand implementation details of an external project
  - User provides a repo URL or owner/repo and wants you to examine it
---

# btx

Clone or update repos for exploration using the `btx` CLI.

## Usage

```bash
btx <repo> [options]
```

**Arguments:**
- `<repo>`: `owner/repo`, HTTPS URL, SSH URL, or local path

**Options:**
- `--fresh` / `-f`: Force fresh clone (removes cached version)
- `--ref <ref>` / `-r <ref>`: Checkout specific branch, tag, or SHA
- `--full`: Clone complete history (default: single-branch)
- `--quiet` / `-q`: Suppress progress logs

## Output

JSON with repo location and metadata:

```json
{
  "path": "/Users/you/.cache/cmd/repos/github.com/owner/repo",
  "url": "https://github.com/owner/repo",
  "branch": "main",
  "updated_at": "2025-01-16T...",
  "stale": false
}
```

## Workflow

1. Run `btx <repo>` to clone/update
2. Parse the JSON output to get the `path`
3. Use Glob/Grep/Read tools to explore the codebase at that path

## Examples

```bash
# GitHub shorthand
btx anthropics/claude-code

# Specific branch
btx tokio-rs/tokio --ref tokio-1.0.0

# Fresh clone
btx facebook/react --fresh

# Local path (just validates and returns info)
btx ./my-project
```
