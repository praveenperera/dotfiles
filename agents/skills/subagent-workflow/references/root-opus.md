# Opus 5 root

Opus is root in a Claude Code session running Opus 5.

Opus orchestrates and can implement. **Do not use the Opus root as the final authority on taste, surface design, or simplification of its own work.** Route those to Fable.

## Routing

| Work | Route |
| --- | --- |
| Orchestration, long-horizon agentic work, complex debugging, root-cause analysis | Opus root |
| Substantial bounded implementation, hard debugging, migrations | Sol via Codex (`high` by default) |
| Implementation when the user says "use opus", or when Opus already owns the thread and Sol is a poor fit | Opus root directly, or Opus via Agent tool for an isolated owned scope |
| High-taste review, UI/UX/API/copy judgment, intent-sensitive surface design | Fable 5 via Agent tool (`model: fable`, `high` effort) |
| Final simplification of a Sol (or Opus) implementation | Fable 5 via Agent tool |
| Visual or interactive implementation after the product shape is clear | Grok 4.6 via Grok CLI (`high` reasoning) |
| Live web or X research that informs a decision | Grok 4.6 via Grok CLI (`high` reasoning) |
| Independent cross-vendor or inventory-style review | Sol via Codex, or Grok for a third-provider view |
| Full implementation substitute when the user says "use grok" | Grok 4.6 via Grok CLI |
| Easy, tightly scoped change with cheap verification, and tests | Luna via Codex (`max` reasoning) |
| Repeated high-volume mechanical work | Luna via Codex (`low` reasoning) |

## Ownership

- Opus owns orchestration, long-horizon agentic implementation, and complex debugging in the root thread.
- Sol remains the default delegated implementer.
- **Fable is the default model for high-taste review, UI/UX/API/copy judgment, intent-sensitive surface design, and final simplification.**
- Grok 4.6 owns visual or interactive first-pass implementation, live web or X research, and the "use grok" implementation substitute.
- A cheap self-check is fine; consequential taste and simplification go to Fable.
- When "use opus" is active, Opus implements, in the root thread or in an isolated Agent scope. Take independent review from Fable for taste and from Sol when cross-vendor inventory scrutiny matters.

## Transport

Spawn Claude delegates with the Agent tool, with an explicit model (`fable`) and `high` effort. Reach Sol and Luna with `codex exec`; see [codex-cli.md](codex-cli.md). Reach Grok with the Grok CLI; see [grok-cli.md](grok-cli.md).

## Cautions

- When the root would ordinarily do a taste pass itself under the Fable-root table, spawn Fable instead. Keep the Fable prompt short and self-contained; point at the diff, owned paths, and a success condition rather than dumping this skill file.
- Treat an Opus report of "done" as unverified. Check the success condition and the diff before accepting a short run, and re-prompt to continue instead of integrating partial work.
- When Grok implements, keep Fable on final taste and simplification. Use Sol to cross-check terminal-heavy or deep repository changes.
- Opus follows large instruction-dense files less reliably than short ones. Read [opus5-prompting.md](opus5-prompting.md) before writing an Opus prompt.
- Do not run models as overlapping writers on the same files.
