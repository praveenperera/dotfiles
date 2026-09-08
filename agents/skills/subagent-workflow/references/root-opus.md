# Claude Opus 5 root

Use this route when the actual session model is Claude Opus 5. Opus owns orchestration, decisions supported by its context, integration, and acceptance.

| Work | Route |
| --- | --- |
| Root-local diagnosis or implementation selected by the user | Opus root |
| Front-end design | Opus root or Astra; the root may suggest Fable |
| Reducing complexity and deciding which code to remove | Astra checks the run's complexity-check list; the root may suggest Fable |
| Difficult architecture and high-risk independent review | Astra `medium`; `low` for focused work; `high` or above only on explicit user request |
| Normal investigation and correctness review | Sol `high`, read-only |
| Code-quality-sensitive implementation, API shape, merge readiness, cleanup | Luna `max` or Sol; the root may suggest Fable |
| Bounded implementation after design is settled | Luna `max` |
| Native X research or visual first pass | Grok 4.6 |
| Bulk exact transformations | Luna `low`, unless an active user choice requires `max` |

Use [claude-cli.md](claude-cli.md) for Fable 5.1, [codex-cli.md](codex-cli.md) for Astra and Luna when native workers are unavailable, and [grok-cli.md](grok-cli.md) for Grok.

Follow [opus5-prompting.md](opus5-prompting.md). Keep required checks, but do not add verification agents to routine work. Use Sol for ordinary code-quality review of Opus-written code; use Astra when correctness across consumers is the main concern. The root may suggest Fable.
