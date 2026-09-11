# Grok 4.6 root

Use this route when the actual session model is `grok-4.6`. Grok owns live evidence collection, selected implementation, integration, and acceptance.

| Work | Route |
| --- | --- |
| Native X research, live evidence, visual or interactive first passes | Grok root |
| Front-end design beyond a first pass | Astra or Opus 5; the root may suggest Fable |
| Reducing complexity and deciding which code to remove | Astra checks the run's complexity-check list; the root may suggest Fable |
| Difficult architecture and high-risk correctness review | Astra `medium`; `low` for focused work; `high` or above only on explicit user request |
| Normal investigation and correctness review | Sol `high`, read-only |
| Code-quality-sensitive implementation, public API shape, merge readiness, cleanup | Luna `max` or Sol; the root may suggest Fable |
| Bounded implementation after design is settled | Luna `max` |
| Deliberate second Claude opinion | Opus 5 when it adds a distinct perspective |
| Bulk exact transformations | Luna `low`, unless an active user choice requires `max` |

Use [claude-cli.md](claude-cli.md) for Claude delegates and [codex-native.md](codex-native.md) for Astra and Luna. Use [grok-cli.md](grok-cli.md) only when starting a separate Grok pass.

A strong visual first pass does not establish production code quality. Use Sol for ordinary merge-readiness review and Astra for difficult correctness questions. The root may suggest Fable. Preserve ownership boundaries while the root continues independent research.
