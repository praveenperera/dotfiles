# GPT-5.6 Sol root

Sol is root when the session starts from the Codex CLI.

Sol implements in the root thread instead of delegating implementation to itself. **Do not use the Sol root as the final authority on taste, surface design, or simplification of its own work**, and account for its tendency to overbuild: the root cannot see its own scope creep. Route those passes to Fable.

## Routing

| Work | Route |
| --- | --- |
| Implementation, hard debugging, migrations, broad investigation | Sol root |
| Ambiguous architecture, intent-sensitive product decisions, public API and UI/UX shape | Fable 5 via the Claude CLI, before implementing |
| High-taste review, copy judgment, final simplification of the root's own work | Fable 5 via the Claude CLI |
| Deliberate second opinion, long-horizon plan critique | Opus 5 via the Claude CLI |
| Independent adversarial or inventory-style review | a fresh ephemeral `codex exec` Sol pass, or Fable when taste is at stake |
| Easy, tightly scoped change with cheap verification, and tests | Luna via Codex (`max` reasoning) |
| Repeated high-volume mechanical work | Luna via Codex (`low` reasoning) |

## Ownership

- Sol owns implementation, debugging, migrations, and investigation in the root thread.
- Fable owns taste, surface design, intent-sensitive decisions, and final simplification.
- Opus supplies the deliberate second opinion when an independent Claude pass helps and another Fable spend is not justified.
- This is the one root where the root and the default delegated implementer are the same model. Prefer the root thread, and spawn a fresh `codex exec` Sol pass only for an isolated owned scope, a clean repair context, or an independent perspective on work the root already did.

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
- Keep `--permission-mode plan` for review, taste, and second-opinion passes, so the reviewer cannot edit the repository.
- Add each required context directory with `--add-dir`; never grant broad filesystem access.
- Never start `claude -p` before the prompt is available; print mode exits when it starts without a prompt argument or standard input.
- Do not steer the reviewer: pass facts, constraints, the diff, and the paths, and leave the judgment to it.

Reach Luna and any Sol subagent with `codex exec`; see [codex-cli.md](codex-cli.md).

## Cautions

- Counter Sol's overbuilding in the root thread, where no delegate prompt can do it: make the smallest coherent change, preserve existing abstractions, and stop to re-plan instead of piling on code when the approach is wrong.
- A self-review by the root inherits the root's blind spots. Send the taste and simplification pass to Fable, and give Fable the diff, the owned paths, and a completion bar.
- Do not run the root and a Sol subagent as overlapping writers on the same files.
