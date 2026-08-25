# Grok 4.6 root

Grok is root when the session starts from Grok Build with model `grok-4.6`.

Grok implements in the root thread when its long-running agent behavior, live research, or visual and interactive strengths fit the task. **Do not use the Grok root as the final authority on taste, surface design, or simplification of its own work.** Route those passes to Fable. Use Sol when difficult terminal work or deep repository surgery is the main risk.

## Routing

| Work | Route |
| --- | --- |
| Long-running implementation, broad product prototypes, visual or interactive first passes | Grok root |
| Live web or X research, unfamiliar-domain investigation | Grok root |
| Terminal-heavy debugging, migrations, difficult repository surgery | Sol via Codex (`high` reasoning) |
| Ambiguous architecture, intent-sensitive product decisions, public API and UI/UX shape | Fable 5 via the Claude CLI, before implementing |
| High-taste review, copy judgment, final simplification of the root's own work | Fable 5 via the Claude CLI |
| Deliberate second opinion or long-horizon plan critique | Opus 5 via the Claude CLI |
| Independent adversarial or inventory-style review | Sol via Codex, or Fable when taste is at stake |
| Easy, tightly scoped change with cheap verification, and tests | Luna via Codex (`max` reasoning) |
| Repeated high-volume mechanical work | Luna via Codex (`low` reasoning) |

## Ownership

- Grok owns long-running implementation, visual or interactive first passes, and live web or X research in the root thread.
- Sol owns terminal-heavy delegated implementation, difficult repository surgery, and persistent inventory work.
- Fable owns taste, surface design, intent-sensitive decisions, and final simplification.
- Opus supplies a deliberate second opinion when an independent Claude pass helps and another Fable spend is not justified.

Prefer the root thread when Grok is the selected implementer. Start a fresh Grok pass only for an isolated owned scope, a clean repair context, or an independent perspective on work the root already did.

## Transport

Reach Claude models with the installed `claude` CLI in print mode, with an explicit model and effort, and a self-contained prompt on standard input:

```sh
claude -p --model fable --effort high \
  --permission-mode plan \
  --no-session-persistence \
  --add-dir "$PWD" \
  <<'CLAUDE_PROMPT'
<prompt>
CLAUDE_PROMPT
```

- Use `--model opus` for an Opus pass.
- Keep `--permission-mode plan` for review, taste, and second-opinion passes.
- Add each required context directory with `--add-dir`; never grant broad filesystem access.
- Do not steer the reviewer. Pass facts, constraints, the diff, and the paths, and leave the judgment to it.

Reach Sol and Luna with `codex exec`; see [codex-cli.md](codex-cli.md).

## Cautions

- Grok 4.6 launch results are strong for agentic and visual work, but its DeepSWE and Terminal-Bench results trail Sol. Route from the actual workload, not the aggregate score.
- Treat the root's visual output as a first pass until an independent taste pass checks it against project examples and user intent.
- A self-review by the root inherits the root's blind spots. Send taste and simplification to Fable and terminal-heavy review to Sol.
- Do not run the root and another writer on overlapping files.
