---
name: use-agents
description: Model roster and routing notes for delegating work to Sonnet 5.5, Haiku 5.5, Opus 5.5, Astra, and Grok subagents. Use when the user says "use agents", "use subagents", names which providers or models to delegate to (for example "only Claude agents", "mostly Sonnet and Opus"), or invokes $use-agents.
---

# Use Agents

This skill is informational. It describes the models Praveen uses, what each is good at, and how to launch them. The user's stated mix decides which models to use; this skill decides what to send to each one inside that mix.

## Apply the user's mix

- With no stated mix, use the default routing below
- "Only Claude" or "only Grok" restricts every delegate to that provider; route inside it with the provider sections below
- "Mostly X and Y" or "prefer X" makes those the default workers; use another model only when the task needs a strength the preferred models lack, and say why
- A named model and effort level always wins over these defaults
- Keep the selection for the rest of the session until the user changes it
- If a selected model is unavailable, report it; do not silently substitute another model or a lower effort

The root thread keeps scope, design decisions, integration, and acceptance. Give each delegate a self-contained prompt with the objective, owned scope, excluded scope, decided approach, verification commands, stop conditions, and a final report of changed files, check results, and risks. Do not give two writing delegates overlapping scope. Inspect every delegate diff; a delegate report is evidence, not proof.

## Roster

| Model                                          | Use for                                                                                                                                                                                                                                                                             | Launch                                                                                                                                         |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Sonnet 5.5 (`claude-sonnet-5-5`)        | Default delegate for most work. Scoped implementation after the design is decided, tests, and migrations. Bug investigation and root-cause analysis, codebase analysis and triage, code review and audits, dead-code and cleanup audits, verification, and computer use             | Claude Agent tool with `model: sonnet`, or [claude-cli.md](references/claude-cli.md) with `claude-sonnet-5-5` when a specific effort is needed |
| Claude Haiku 5.5 (`claude-haiku-5-5`)          | Mechanical and high-volume work with a cheap check: renames, signature changes, import moves, fixtures and table test cases, boilerplate that follows an existing pattern, inventories, classifications, repeated transforms, and focused lookups                                   | Claude Agent tool with `model: haiku`, or [claude-cli.md](references/claude-cli.md) with `claude-haiku-5-5`                                    |
| Claude Opus 5.5 (`claude-opus-5-5`)            | Usual root. As a delegate: long unattended builds and heavy rewrites, implementation that needs judgment and must be merge-ready, front-end and UI design, game feel, and work where the delegate must ask for help or change course instead of following a process into a dead end | Claude Agent tool with `model: opus`, or [claude-cli.md](references/claude-cli.md) with `claude-opus-5-5` when a specific effort is needed     |
| GPT-6 Astra (`gpt-6-astra`) at `low` or `high` | Advisor and second opinion only, read-only. Consult it on a big architecture or design decision, or when the root is stuck. The root keeps the decision. `low` for a focused question, `high` for a broad or high-risk one                                                          | [codex-native.md](references/codex-native.md) in Codex; [codex-cli.md](references/codex-cli.md) elsewhere                                      |
| Grok 4.6 (`grok-4.6`)                          | Implementation when quality matters more than usage; X and live web research                                                                                                                                                                                                        | [grok-cli.md](references/grok-cli.md); `ask-grok` skill for questions and X lookups                                                            |
| Grok 4.5 (`grok-4.5`)                          | Implementation, and the default Grok for long or high-volume work because it has better usage limits                                                                                                                                                                                | [grok-cli.md](references/grok-cli.md) with `--model grok-4.5`                                                                                  |

Do not use these unless the user names them: Claude Fable (any version) and Grok 4.7.

Run Claude delegates through the Claude CLI with `homebased` by default; see [Run delegates through homebased](#run-delegates-through-homebased). The CLI also sets an explicit effort. The Claude Agent tool selects the model but inherits the session effort, so keep it for short tasks where the session effort is acceptable.

## Sonnet 5.5, Haiku 5.5, or Opus 5.5

Send work to **Sonnet 5.5** by default. Use it for:

- at `medium`: implementation with a decided approach, a tight scope, and a cheap correctness check (tests, type check, lint, build)
- at `medium`: tests, migrations, and review-fix-loop fix passes
- at `high`: bug investigation, root-cause analysis, and mapping what code a change must touch and why
- at `high`: review, audits, and verification of code that Opus or another agent wrote
- at `high`: browser and computer-use tasks

Send work to **Haiku 5.5** when following the prompt literally gives the right result and a cheap check proves it: renames, fixtures, boilerplate, inventories, classifications, repeated transforms, and focused lookups. Do not send it work that needs judgment about design, naming, or what to leave out.

Haiku is priced at $0.10/$0.50 per MTok only while the prompt stays under 100K tokens; above that it costs five times as much ($0.50/$2.50), and its compaction is set at 100K to stay under the line. So prefer many small Haiku agents over one long one: split the work into slices that each finish well under 100K tokens, such as one directory, file group, or batch of items per agent, and fan them out in parallel. Give each brief only the context its slice needs. A Haiku run that keeps hitting compaction is a sign the slice is too big; split it further instead of letting it run on.

Send work to **Opus 5.5** when any of these are true:

- the work is a long unattended build, a large port, or a heavy rewrite
- the code must be merge-ready and the task needs judgment about names, API shape, or what to leave out
- the work involves front-end, UI, visual design, or game feel
- the spec has gaps that the delegate must fill sensibly
- the work is a deep audit of a large or high-risk change; use Opus at `high`
- a Sonnet pass failed twice on the same defect

A common split: the root decides the design (with an optional Astra second opinion on a big decision), Haiku 5.5 handles mechanical passes, Sonnet 5.5 at `medium` implements bounded passes or Opus 5.5 implements judgment-heavy ones, and a fresh Sonnet 5.5 run at `high` reviews and tests the result. When Opus wrote the code, a Sonnet review is the default second pair of eyes. No delegate replaces the root's diagnosis, architecture, or final acceptance.

## Sonnet 5.5 and Haiku 5.5 effort levels

Sonnet 5.5 defaults to `high`, and its levels are recalibrated from Sonnet 5, so do not carry over Sonnet 5 habits. Haiku 5.5 defaults to `medium`. Do not use `xhigh` or `max` on Sonnet; send a deep audit of a large or high-risk change to Opus 5.5 at `high` instead. Do not use `max` on Haiku.

| Model  | Effort   | Use for                                                                                                |
| ------ | -------- | ------------------------------------------------------------------------------------------------------ |
| Sonnet | `medium` | Implementation after the design is decided, tests, migrations, and fix passes                          |
| Sonnet | `high`   | Bug investigation and root-cause analysis, codebase triage, review, audits, verification, computer use |
| Haiku  | `low`    | Focused lookups, inventories, and classifications                                                      |
| Haiku  | `medium` | Mechanical edits and repeated transforms                                                               |

If a Haiku pass fails its check or needs a judgment call, rerun it on Sonnet at `medium` instead of sending repair passes to Haiku. If a Sonnet `medium` pass fails the same way, rerun it at `high`.

## Route inside one provider

- **Claude only:** the default routing above without Astra or Grok. Keep big design decisions in the root
- **Grok only:** Grok 4.5 for bulk and long implementation, Grok 4.6 for work where quality matters or a 4.5 pass fell short. Keep design decisions in the root
- **OpenAI only:** Astra is the only OpenAI model in the roster and it is advisory, so there is no implementation worker. Report that instead of substituting another provider

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
- Pattern that works well: implement at `medium`, review in the root, then run a fresh Sonnet 5.5 pass at `high` to verify and test edge cases
- If Opus 5.5 at `xhigh` hits the same problem twice, the approach is probably wrong; return the decision to the root, optionally with an Astra second opinion, instead of raising effort

## Run delegates through homebased

Use the `homebased` skill to launch every delegate that is not a short task. This includes Sonnet, Haiku, and Opus. The daemon runs the child, streams its output, and sends a `HOMEBASED_EVENT` back to this session when the task ends, so the root can end its turn instead of running a wait or poll loop.

- **Default:** `homebased` for implementation with verification, investigation, review, audits, research, and any fan-out
- **Short tasks only:** use a native subagent (Claude Agent tool, Codex native spawn) when the scope is narrow and the root checks and integrates the result in the same turn, such as a focused lookup or a small edit
- **External agents** (Grok CLI, Codex CLI for Astra, Claude CLI from Codex): always `homebased`, whatever the task size

When it is unclear whether a task is short, use `homebased`.

The transport references below still give the model flags, permission rules, prompt contract, and evidence capture to put in the homebased spec.

## Fan out and report once

When work splits into many parallel workers, such as coverage slices, a race between identical briefs, or a batch of measurements, give the user one report instead of a stream of status updates:

1. Before launching, state the done condition and the report the fan-out must return. For a race, declare the selection rule up front: first pass, rank all, or best of.
2. Launch every worker at once. Each brief stands alone and names its slice, how to verify, and the exact commits and method when it measures. Each worker reports `PASS`, `ISSUES`, or `BLOCKED` with evidence, and lists every issue it can prove, not only the first.
3. When the user asks one coordinator to run the fan-out, such as a Sonnet `high` agent that starts Haiku workers, that coordinator drains the workers and sends the single report. Grant it nested delegation in its brief.
4. Drop a result that lacks the commits or method its brief named and respawn that worker once. A second miss is a gap, and a gap is never a pass. If a worker drops out, continue without it and note it.
5. Report one compact table of results, one-line evidenced issues, gaps and dropouts, and the race rule when used. Do not paste raw worker output.

## Long commands inside a delegate

A delegate must not wait inside a long build, test suite, CI watch, benchmark, or training run. Waiting uses its turn and context budget, and if the delegate dies or is cancelled, everything it learned dies with it. The homebased worker rules forbid submitting tasks unless the prompt allows it, so every brief for a delegate that may run such a command must grant that permission and include this handoff:

1. Write `RESUME.md` under the repository's `_scratch/` directory, never the repository root, with the task id, what each outcome means, and the next steps for each outcome
2. Submit the command as a homebased `task` workload with `thread` set to the root's id, which `homebased --json task show "$HOMEBASED_TASK_ID"` prints
3. Report `blocked` with the summary `WAITING <task-id>` and exit

When the event for the waited task arrives, the root resumes the delegate with that outcome. For Claude delegates (Opus, Sonnet, Haiku), submit a new Claude task with the original brief, the outcome, and an instruction to continue from `RESUME.md`. Homebased runs Claude with `--no-session-persistence` and rejects `--resume` and `--continue`, so the old session cannot resume, and `RESUME.md` is the only thing that carries progress forward.

Keep this rule in a shared brief file that each delegate prompt includes, so new delegates get it without the root restating it. A running Claude delegate gets it through `homebased message send --worker`; for other providers, add the rule to the next brief.

## Transport references

Read [delegate-run.md](references/delegate-run.md) for the shared run directory, prompt contract, and postflight capture, then only the reference for the transport in use:

- [claude-cli.md](references/claude-cli.md): Claude CLI for Sonnet 5.5, Haiku 5.5, or Opus 5.5 with an explicit effort
- [codex-native.md](references/codex-native.md): Codex `collaboration.spawn_agent` for Astra
- [codex-cli.md](references/codex-cli.md): `codex exec` for Astra outside Codex
- [grok-cli.md](references/grok-cli.md): Grok headless runs and permission preflight
