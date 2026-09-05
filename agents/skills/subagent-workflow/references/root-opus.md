# Claude Opus 5 root

Use this route when the actual session model is Claude Opus 5. Opus owns orchestration, decisions supported by its context, integration, and acceptance.

| Work | Route |
| --- | --- |
| Root-local diagnosis or implementation selected by the user | Opus root |
| Difficult architecture, broad investigation, independent correctness review | Astra `high` |
| Code-quality-sensitive implementation, API shape, merge readiness, cleanup | Fable 5.1 `high` |
| Bounded implementation after design is settled | Luna `max` |
| Native X research or visual first pass | Grok 4.6 |
| Bulk exact transformations | Luna `low`, unless an active user choice requires `max` |

Use [claude-cli.md](claude-cli.md) for Fable 5.1, [codex-cli.md](codex-cli.md) for Astra and Luna when native workers are unavailable, and [grok-cli.md](grok-cli.md) for Grok.

Follow [opus5-prompting.md](opus5-prompting.md). Keep required checks, but do not add verification agents to routine work. Route consequential code-quality review of Opus-written code to Fable 5.1; use Astra when correctness across consumers is the main concern.
