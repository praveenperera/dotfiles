# Claude Fable 5.1 root

Use this route when the actual session model is Claude Fable 5.1. Fable owns code quality, focused cleanup, integration, and acceptance. Do not delegate away a small change it can finish directly.

| Work | Route |
| --- | --- |
| Code ready for merge, public API shape, focused cleanup, product intent, writing | Fable root |
| Domain decisions already supported by the root's context | Fable root |
| Front-end design | Fable root, Astra, or Opus 5 |
| Reducing complexity and deciding which code to remove | Fable root or Astra |
| Difficult architecture, subtle diagnosis, high-risk independent review | Astra `medium`; `low` for focused work; `high` or above only on explicit user request |
| Normal investigation and correctness review | Sol `high`, read-only |
| Bounded implementation after design is settled | Luna `max` |
| Native X research or visual first pass | Grok 4.6 |
| Deliberate second Claude opinion | Opus 5, or a fresh Fable 5.1 run for independent code-quality judgment |
| Bulk exact transformations | Luna `low`, unless an active user choice requires `max` |

Use [codex-cli.md](codex-cli.md) for Astra and Luna when native workers are unavailable. Use [claude-cli.md](claude-cli.md) for Claude Agent-tool or CLI selection and [grok-cli.md](grok-cli.md) for Grok.

Apply the Fable section of [frontier-prompting.md](frontier-prompting.md). Fable can simplify its own implementation, but that does not replace independent review when the change requires it. Use a fresh Sol `high` run for normal correctness review and Astra for difficult or high-risk review; Fable-written code does not automatically require Astra.
