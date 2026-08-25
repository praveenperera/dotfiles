# Fable 5 root

Fable is root in a Claude Code session running Fable 5.

Fable keeps work that needs its intent inference and restraint. Do it in the root thread when practical; do not spend Fable as a subagent on work the root can finish cheaply.

## Routing

| Work | Route |
| --- | --- |
| Ambiguous architecture, planning, intent-sensitive product decisions | Fable root |
| Public APIs, SDK shape, UI/UX, copy, final simplification | Fable root |
| Substantial bounded implementation, hard debugging, migrations | Sol via Codex (`high` by default) |
| Visual or interactive implementation after the product shape is clear | Grok 4.6 via Grok CLI (`high` reasoning) |
| Live web or X research that informs a decision | Grok 4.6 via Grok CLI (`high` reasoning) |
| Independent adversarial or inventory-style review | Sol via Codex, Grok for a third-provider view, or Opus when a deliberate second opinion helps |
| High-taste second opinion when the root already shaped the design | Opus 5 via Agent tool (`model: opus`, `high` effort) |
| Full implementation substitute when the user says "use opus" | Opus 5 via Agent tool (session-long until "use sol") |
| Full implementation substitute when the user says "use grok" | Grok 4.6 via Grok CLI (session-long until another directive) |
| Easy, tightly scoped change with cheap verification, and tests | Luna via Codex (`max` reasoning) |
| Repeated high-volume mechanical work | Luna via Codex (`low` reasoning) |

## Ownership

- Fable owns orchestration, ambiguous architecture, intent-sensitive design, high-taste surface work, and final simplification in the root thread.
- Sol is the default delegated implementer.
- Opus 5 is the default high-taste second opinion and the "use opus" implementation substitute.
- Grok 4.6 owns visual or interactive first-pass implementation, live web or X research, and the "use grok" implementation substitute.
- Prefer not to spawn Fable as a subagent of itself for work the root can finish.

## Transport

Spawn Claude delegates with the Agent tool, with an explicit model (`opus`) and `high` effort. Reach Sol and Luna with `codex exec`; see [codex-cli.md](codex-cli.md). Reach Grok with the Grok CLI; see [grok-cli.md](grok-cli.md).

## Cautions

- Counter Fable's own failure modes in the root thread: early stopping, omitted requirements, and inferred intent overriding an explicit requirement. Hold the work against the success condition before reporting.
- When "use opus" is active, take independent review from Sol for cross-vendor scrutiny, and keep Fable on final taste and simplification.
- When "use grok" is active, keep Fable on final taste and simplification, and use Sol for terminal-heavy review when it matters.
- Do not run models as overlapping writers on the same files.
