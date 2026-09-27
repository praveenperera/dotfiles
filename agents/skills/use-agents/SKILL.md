---
name: use-agents
description: Model roster and routing notes for delegating work to Opus 5.5, Luna Max, Astra, and Grok subagents. Use when the user says "use agents", "use subagents", names which providers or models to delegate to (for example "only OpenAI agents", "mostly Luna Max and Opus"), or invokes $use-agents.
---

# Use Agents

This skill is informational. It describes the models Praveen uses, what each is good at, and how to launch them. The user's stated mix decides which models to use; this skill decides what to send to each one inside that mix.

## Apply the user's mix

- With no stated mix, use the default routing below
- "Only OpenAI", "only Claude", or "only Grok" restricts every delegate to that provider; route inside it with the provider sections below
- "Mostly X and Y" or "prefer X" makes those the default workers; use another model only when the task needs a strength the preferred models lack, and say why
- A named model and effort level always wins over these defaults
- Keep the selection for the rest of the session until the user changes it
- If a selected model is unavailable, report it; do not silently substitute another model or a lower effort

The root thread keeps scope, design decisions, integration, and acceptance. Give each delegate a self-contained prompt with the objective, owned scope, excluded scope, decided approach, verification commands, stop conditions, and a final report of changed files, check results, and risks. Do not give two writing delegates overlapping scope. Inspect every delegate diff; a delegate report is evidence, not proof.

## Roster

| Model                                          | Use for                                                                                                                                                                                                                    | Launch                                                                                                                                     |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Claude Opus 5.5                                | Best all-around agent. Makes design and architecture decisions for most tasks. Diagnosis, brownfield bug fixes, implementation that needs judgment, front-end design, review, and verification                             | Claude Agent tool with `model: opus`, or [claude-cli.md](references/claude-cli.md) with `claude-opus-5-5` when a specific effort is needed |
| GPT-6 Luna (`gpt-6-luna`) at `max`             | Implementation after the design is decided: bounded features, tests, fixtures, migrations, renames, repeated transforms, and boilerplate that follows an existing pattern                                                  | [codex-native.md](references/codex-native.md) in Codex; [codex-cli.md](references/codex-cli.md) elsewhere                                  |
| GPT-6 Astra (`gpt-6-astra`) at `low` or `high` | Advisor and second opinion only, read-only. Consult it on a big architecture or design decision, or when the root is stuck. The root keeps the decision. `low` for a focused question, `high` for a broad or high-risk one | Same as Luna                                                                                                                               |
| Grok 4.6 (`grok-4.6`)                          | Implementation when quality matters more than usage; X and live web research                                                                                                                                               | [grok-cli.md](references/grok-cli.md); `ask-grok` skill for questions and X lookups                                                        |
| Grok 4.5 (`grok-4.5`)                          | Implementation, and the default Grok for long or high-volume work because it has better usage limits                                                                                                                       | [grok-cli.md](references/grok-cli.md) with `--model grok-4.5`                                                                              |

Do not use these unless the user names them: Claude Fable (any version), GPT-6 Sol, GPT-5.6 Sol, and Grok 4.7.

## Opus 5.5 versus Luna Max

Send work to **Luna Max** when all of these are true:

- the approach is decided and written into the prompt
- the scope is tight and names exact files or responsibilities
- a cheap check (tests, type check, lint, build) proves correctness
- following the instructions literally gives the right result

Send work to **Opus 5.5** when any of these are true:

- the cause of a bug is not known yet, or the code is brownfield with hidden coupling
- the task needs judgment about names, API shape, UI, or what to leave out
- the spec has gaps that the delegate must fill sensibly
- the work is review, verification, or edge-case testing of code another agent wrote
- a Luna pass failed twice on the same defect

A common split: the root or Opus 5.5 decides the design (with an optional Astra second opinion on a big decision), Luna Max implements it in bounded passes, and a fresh Opus 5.5 run at `high` reviews and tests the result. Luna cannot replace diagnosis, architecture, or final acceptance.

## Route inside one provider

- **OpenAI only:** the root decides the design, Luna Max implements and tests, and Astra `low` or `high` gives a read-only second opinion on big or hard decisions. For review, use a fresh Luna Max run with a read-only sandbox and a concrete checklist
- **Claude only:** Opus 5.5 for everything; vary effort by task (see below). Use lower effort for mechanical passes and `high` for review and verification
- **Grok only:** Grok 4.5 for bulk and long implementation, Grok 4.6 for work where quality matters or a 4.5 pass fell short. Keep design decisions in the root

## Opus 5.5 effort levels

Use mostly `medium` and `high` for Opus 5.5. Use `low` and `xhigh` only where the table below says. Never use `max`. The default is `medium`. A level spends more thinking on Opus 5.5 than the same level did on Opus 5, so do not carry over Opus 5 habits.

Effort controls how much the model verifies, tests edge cases, and uses its own judgment. It is not a general quality dial. Higher effort reduces failures from missed edge cases, untested bugs, and incomplete fixes. It does not fix a wrong approach or a misread requirement; fix the prompt, spec, or model choice instead.

| Effort   | Use for                                                                                                                                                                  |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `low`    | Fast work where the user stays in the loop: brainstorming, sketches, easy or mechanical changes, and a first implementation of a detailed spec that the root will review |
| `medium` | Regular feature implementation with a clear spec                                                                                                                         |
| `high`   | Work where verification or edge cases matter: brownfield bug fixes, review, and the verify-and-test pass after a lower-effort implementation                             |
| `xhigh`  | Rare. Hard problems with many hidden edge cases, such as storage engines, parsers, sanitizers, concurrency, and performance work, when `high` clearly falls short        |

Guidance for delegation:

- A detailed spec makes effort levels converge. With a precise prompt, `medium` implementation is usually enough; save higher effort for verification
- A delegate has no user in the loop. When it must make judgment calls on its own, such as choosing between two valid readings of the data, `high` does that better than `low`
- Give the delegate a way to check its work (tests, a reference implementation, a repro) before raising effort
- Pattern that works well: implement at `medium`, review in the root, then run a fresh Opus 5.5 pass at `high` to verify and test edge cases
- If Opus 5.5 at `xhigh` hits the same problem twice, the approach is probably wrong; return the decision to the root, optionally with an Astra second opinion, instead of raising effort

## Run delegates through homebased

Use the `homebased` skill to launch delegates. The daemon runs the child, streams its output, and sends a `HOMEBASED_EVENT` back to this session when the task ends, so the root can end its turn instead of running a wait or poll loop.

- **External agents** (Grok CLI, Claude CLI from Codex, Codex CLI from Claude): always use `homebased`
- **Internal agents** (Codex native spawn, Claude Agent tool): `homebased` is also useful for self-contained work that can run unattended, such as implementation with verification or a broad review, because it removes wait loops. Keep a native subagent for small tasks that the root checks and integrates in the same turn

The transport references below still give the model flags, permission rules, prompt contract, and evidence capture to put in the homebased spec.

## Transport references

Read only the reference for the transport in use:

- [codex-native.md](references/codex-native.md): Codex `collaboration.spawn_agent` for Luna and Astra
- [codex-cli.md](references/codex-cli.md): `codex exec` for Luna and Astra outside Codex; shared prompt template and evidence capture
- [claude-cli.md](references/claude-cli.md): Claude CLI for Opus 5.5 with an explicit effort
- [grok-cli.md](references/grok-cli.md): Grok headless runs, permission preflight, and sandboxing
