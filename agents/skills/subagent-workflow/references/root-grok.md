# Grok 4.6 root

Use this route when the actual session model is `grok-4.6`. Grok owns live evidence collection, selected implementation, integration, and acceptance.

| Work | Route |
| --- | --- |
| Native X research, live evidence, visual or interactive first passes | Grok root |
| Front-end design beyond a first pass | Astra, Opus 5, or Fable 5.1 |
| Reducing complexity and deciding which code to remove | Astra or Fable 5.1 |
| Difficult architecture and high-risk correctness review | Astra `medium`; `low` for focused work; `high` or above only on explicit user request |
| Normal investigation and correctness review | Sol `high`, read-only |
| Code-quality-sensitive implementation, public API shape, merge readiness, cleanup | Fable 5.1 `high` |
| Bounded implementation after design is settled | Luna `max` |
| Deliberate second Claude opinion | Opus 5 when it adds a distinct perspective |
| Bulk exact transformations | Luna `low`, unless an active user choice requires `max` |

Use [claude-cli.md](claude-cli.md) for Claude delegates and [codex-cli.md](codex-cli.md) for Astra and Luna when native workers are unavailable. Use [grok-cli.md](grok-cli.md) only when starting a separate Grok pass.

A strong visual first pass does not establish production code quality. Use Fable 5.1 for consequential merge-readiness review and Astra for difficult correctness questions. Preserve ownership boundaries while the root continues independent research.
