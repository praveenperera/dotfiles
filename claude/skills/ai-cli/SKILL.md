---
name: ai-cli
description: LLM-friendly CLI design guide - 15 rules for building CLIs that AI agents can use effectively. Use when designing or reviewing CLI interfaces for agent consumption.
---

# LLM-Friendly CLI Design Guide

15 rules for building CLIs that AI agents can use effectively, derived from research across clig.dev, the "Let Me Speak Freely?" paper (arXiv:2408.02442), the "How Do LLMs Fail In Agentic Scenarios?" paper (arXiv:2512.07497), and analysis of `gh`, `kubectl`, `aws`, `docker` CLI patterns.

## Why CLIs Beat curl for LLMs

- "Let Me Speak Freely?" showed **10-15% reasoning degradation** when forcing structured output (JSON) vs free-form generation — curl requires simultaneous URL/header/JSON/auth management
- 900-trace failure analysis found curl top failures: JSON escaping disasters, URL encoding errors, hallucinated endpoints, body/query param confusion
- CLIs are **20-50 tokens** per command vs **100-300 for equivalent curl** (up to 160x token reduction)
- "Human users adapt to CLI changes; AI agents fail catastrophically" — every output field, exit code, and flag is an API contract

## The 15 Rules

### 1. Noun-verb subcommand structure
`myctl resource action` — turns discovery into deterministic tree search via `--help`. Follow `gh pr list`, `kubectl get pods`, `aws s3 ls`. Avoid flat verb-only (`create-user`, `list-users`) or mixed patterns.

### 2. `--json` flag on every data-producing command
JSON to stdout, all noise (progress, warnings, logs) to stderr. Keep JSON flat, consistent types. JSONL for streaming. `--json field1,field2` with `--jq` for inline filtering (like `gh`).

### 3. Semantic exit codes
Not just 0/1. Use: 0=success, 1=general failure, 2=usage error, 3=not found, 4=permission denied, 5=conflict. Document them, keep stable across versions and subcommands.

### 4. `--quiet` flag
Bare values, one per line. Perfect for piping: `myctl user list -q | xargs -I{} myctl user delete {}`.

### 5. `--dry-run` flag
Preview changes as structured diff without applying.

### 6. `--yes` / `--no-prompt` flag
Skip all interactive prompts. Agents can't type "y".

### 7. Auto-TTY detection
Human format for terminal, machine format for pipes. `gh` auto-switches to tab-delimited with no color/truncation when piped.

### 8. Structured errors
Include: error code (machine-parseable string), failing input (echo back what was sent), `retryable: true/false`, and suggestions. In `--json` mode, errors are also JSON. Never swallow errors silently. Never mix errors into stdout.

### 9. Consistent flag naming
Same flag does same thing in every subcommand. `--output` is always `--output`, not `--format` in one place and `--output` in another.

### 10. Idempotent operations
Prefer `ensure`/`apply`/`sync` over `create`/`delete`. When not possible, use `--if-not-exists` or distinct exit code (5) for "already exists".

### 11. Self-documenting help
Mark required vs optional. List valid enum values inline (`--env: dev, staging, prod` not just `--env string`). Show defaults. Show realistic examples. Most-used flags first.

### 12. `--stdin` / `-f -` support
Read input from pipe. Document explicitly.

### 13. Flat, stable JSON schema
Field names and types are a contract. Breaking changes cause silent agent failures. Version output schema with `api_version` field.

### 14. Introspection commands
`config show`, `auth status`, `version --json`. Optional: `commands` (flat list), `schema` (JSON Schema export).

### 15. Respect `NO_COLOR`
Strip formatting when not a TTY. Support `NO_COLOR=1` env var.

## Gold Standard: What `gh` Does Right

- Auto format switching when piped vs terminal
- `gh api` gives raw API access with auth handled
- `--json field1,field2 --jq '...'` for inline filtering
- Extensions system for adding capabilities
- Every destructive operation has `--yes`

## Anti-Patterns That Break LLM Usage

- Exit 0 on error
- Prompting for confirmation when stdin is a pipe
- Printing headers/decoration to stdout
- Color escape codes in piped output
- Paginating output (like `less`) in pipe context
- Inconsistent field naming across commands
- Requiring temp files instead of supporting stdin

## Sources

- [Let Me Speak Freely?](https://arxiv.org/abs/2408.02442) — structured output degrades reasoning
- [How Do LLMs Fail In Agentic Scenarios?](https://arxiv.org/abs/2512.07497) — 900-trace failure analysis
- [Command Line Interface Guidelines](https://clig.dev/)
- [Writing CLI Tools That AI Agents Actually Want to Use](https://dev.to/uenyioha/writing-cli-tools-that-ai-agents-actually-want-to-use-39no)
- [Keep the Terminal Relevant: Patterns for AI Agent Driven CLIs](https://www.infoq.com/articles/ai-agent-cli/)
- [Scripting with GitHub CLI](https://github.blog/engineering/engineering-principles/scripting-with-github-cli/)
- [Why CLIs Outperform MCP for AI Agents](https://medium.com/codetodeploy/why-clis-outperform-mcp-for-ai-agents-and-how-to-build-your-own-cli-army-1b423a7be782)
