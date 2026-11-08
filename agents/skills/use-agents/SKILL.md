---
name: use-agents
description: Model roster and routing notes for delegating work to Opus 5.5, GPT-6.1 Sol, Astra, Sonnet 5.5, Haiku 5.5, and Grok subagents. Use when the user says "use agents", "use subagents", names which providers or models to delegate to (for example "only Claude agents", "mostly Opus"), or invokes $use-agents.
disable-model-invocation: true
---

# Use Agents

This skill is informational. It describes the models Praveen uses, what each is good at, and how to launch them. The user's stated mix decides which models to use; this skill decides what to send to each one inside that mix.

## Budget

Praveen has two Claude Ultra plans and one $100 Codex plan. Claude capacity is plentiful, so Opus 5.5 does most delegated work. Sol and Astra share the single Codex plan, so spend it on reviews and rare second opinions, not on implementation or exploration.

## Apply the user's mix

- With no stated mix, use the default routing below
- "Only Claude" or "only Grok" restricts every delegate to that provider; route inside it with the provider sections below
- "Mostly X and Y" or "prefer X" makes those the default workers; use another model only when the task needs a strength the preferred models lack, and say why
- A named model and effort level always wins over these defaults
- Keep the selection for the rest of the session until the user changes it
- If a selected model is unavailable, report it; do not silently substitute another model or a lower effort

The root thread keeps scope, design decisions, integration, and acceptance. Give each delegate a self-contained prompt with the objective, owned scope, excluded scope, decided approach, verification commands, stop conditions, and a final report of changed files, check results, and risks. Do not give two writing delegates overlapping scope. Inspect every delegate diff; a delegate report is evidence, not proof.

## Roster

| Model                                          | Use for                                                                                                                                                                                                                                        | Launch                                                                                                                                     |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Claude Opus 5.5 (`claude-opus-5-5`)            | Default delegate for most work: implementation, tests, migrations, fix passes, bug investigation and root-cause analysis, codebase triage, exploration, verification, computer use, long unattended builds, heavy rewrites, and front-end work | Claude Agent tool with `model: opus`, or [claude-cli.md](references/claude-cli.md) with `claude-opus-5-5` when a specific effort is needed |
| GPT-6.1 Sol (`gpt-6.1-sol`) at `xhigh`         | Code review, read-only. One fresh review of a finished change set before acceptance                                                                                                                                                            | [codex-native.md](references/codex-native.md) in Codex; [codex-cli.md](references/codex-cli.md) elsewhere                                  |
| GPT-6 Astra (`gpt-6-astra`) at `low` or `high` | Second opinion only, read-only, and sparingly: a big architecture or design decision, or when the root is stuck after its own attempts. The root keeps the decision. `low` for a focused question, `high` for a broad or high-risk one         | [codex-native.md](references/codex-native.md) in Codex; [codex-cli.md](references/codex-cli.md) elsewhere                                  |
| Claude Haiku 5.5 (`claude-haiku-5-5`)          | Optional. High-volume mechanical fan-out where following the prompt literally is enough and a cheap check proves it, such as renames, fixtures, inventories, and repeated transforms across many slices                                        | Claude Agent tool with `model: haiku`, or [claude-cli.md](references/claude-cli.md) with `claude-haiku-5-5`                                |
| Claude Sonnet 5.5 (`claude-sonnet-5-5`)        | Optional, at the root's discretion. Well-scoped work where Opus would be more than the task needs, such as bounded implementation with a cheap check, investigation, or verification                                                           | Claude Agent tool with `model: sonnet`, or [claude-cli.md](references/claude-cli.md) with `claude-sonnet-5-5`                              |
| Grok 4.5 (`grok-4.5`)                          | Implementation, and the default Grok for long or high-volume work because it has better usage limits. At `high` effort, reading X posts                                                                                                        | [grok-cli.md](references/grok-cli.md) with `--model grok-4.5`; `ask-grok` skill for X reads                                                |

Do not use these unless the user names them: Claude Fable (any version), Grok 4.7, and Sol for implementation or exploration (the `use-sol` skill covers that when the user asks for it).

Run Claude delegates through the Claude CLI with `homebased` by default; see [Run delegates through homebased](#run-delegates-through-homebased). The CLI also sets an explicit effort. The Claude Agent tool selects the model but inherits the session effort, so keep it for short tasks where the session effort is acceptable.

## Default routing

Send work to **Opus 5.5** unless another row of the roster fits better. Pick the effort from [Opus 5.5 effort levels](#opus-55-effort-levels).

Use **Sonnet 5.5** when you judge that Opus is more than the task needs, such as a bounded pass with a decided approach and a cheap check, or a long agent loop where the cheaper model is good enough. If a Sonnet pass fails its check twice on the same defect, rerun it on Opus.

Use **Haiku 5.5** only for wide mechanical fan-out, where many cheap slices finish faster than a few Opus runs and a check catches every mistake. Haiku is priced at $0.10/$0.50 per MTok only while the prompt stays under 100K tokens; above that it costs five times as much, and its compaction is set at 100K. So split the work into slices that each finish well under 100K tokens, such as one directory, file group, or batch of items per agent. If a Haiku pass fails its check or needs a judgment call, rerun it on Opus at `medium` instead of sending repair passes to Haiku.

Send **reviews to Sol at `xhigh`**. For a larger task, run one fresh review of the related change set once the implementation is done and verified; add a second only when another risk area would bloat that prompt. Do not start one reviewer per package or file, do not repeat completed checks, and skip the Sol review for small, reversible changes the root has already checked. When Sol is unavailable or its usage is exhausted, report that and run the review on a fresh Opus at `high`.

Use **Astra sparingly**. Consult it at most once or twice in a task, and only for a decision that is expensive to reverse or when the root is stuck after its own attempts. For an ordinary close call, ask a fresh read-only Opus at `high` instead. Do not consult Astra for routine choices with a conventional default.

A common split: the root decides the design (with an Astra second opinion only on a big decision), Opus implements at `medium` or `high`, Haiku handles any wide mechanical pass, the root verifies, and one fresh Sol `xhigh` review checks the result. No delegate replaces the root's diagnosis, architecture, or final acceptance.

## Route inside one provider

- **Claude only:** the default routing with no Sol, Astra, or Grok. Reviews go to a fresh Opus at `high`; keep big design decisions in the root
- **Grok only:** Grok 4.5 for bulk and long implementation, Grok 4.6 for work where quality matters or a 4.5 pass fell short. Keep design decisions in the root
- **OpenAI only:** Sol for implementation through the `use-sol` skill and for reviews at `xhigh`, Astra for second opinions. Say that this draws on the single Codex plan

## Opus 5.5 effort levels

Use mostly `medium` and `high` for Opus 5.5. Use `low` and `xhigh` only where the table below says. Never use `max`. The default is `medium`. A level spends more thinking on Opus 5.5 than the same level did on Opus 5, so do not carry over Opus 5 habits.

Effort controls how much the model verifies, tests edge cases, and uses its own judgment. It is not a general quality dial. Higher effort reduces failures from missed edge cases, untested bugs, and incomplete fixes. It does not fix a wrong approach or a misread requirement; fix the prompt, spec, or model choice instead.

| Effort   | Use for                                                                                                                                                                  |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `low`    | Fast work where the user stays in the loop: brainstorming, sketches, easy or mechanical changes, and a first implementation of a detailed spec that the root will review |
| `medium` | Feature implementation with a clear spec, tests, migrations, fix passes, cleanups, and docs                                                                              |
| `high`   | Work where verification or edge cases matter: brownfield bug fixes, investigation and root-cause analysis, triage, verification, computer use, and Claude-only reviews   |
| `xhigh`  | Rare. Hard problems with many hidden edge cases, such as storage engines, parsers, sanitizers, concurrency, and performance work, when `high` clearly falls short        |

Guidance for delegation:

- A detailed spec makes effort levels converge. With a precise prompt, `medium` implementation is usually enough; save higher effort for verification
- A delegate has no user in the loop. When it must make judgment calls on its own, such as choosing between two valid readings of the data, `high` does that better than `low`
- Give the delegate a way to check its work (tests, a reference implementation, a repro) before raising effort
- Pattern that works well: implement at `medium`, verify in the root or with a fresh Opus at `high`, then one Sol `xhigh` review of the change set
- If Opus 5.5 at `xhigh` hits the same problem twice, the approach is probably wrong; return the decision to the root, optionally with an Astra second opinion, instead of raising effort

## Sonnet 5.5 and Haiku 5.5 effort levels

Use Sonnet at `medium` for implementation and `high` for investigation and review; never `xhigh` or `max`. Use Haiku at `low` for lookups, inventories, and classifications and `medium` for mechanical edits; never `max`.

## Run delegates through homebased

Use the `homebased` skill to launch every delegate that is not a short task. The daemon runs the child, streams its output, and sends a `HOMEBASED_EVENT` back to this session when the task ends, so the root can end its turn instead of running a wait or poll loop. In a T3 Code thread (the `t3-code` MCP tools are present), use T3's `delegate_task` instead for that agent work and for every model from another provider, so the work shows under the thread; keep `homebased` there for command tasks, Fleet machines, and GPU work.

- **Default:** `homebased` for implementation with verification, investigation, review, audits, research, and any fan-out
- **Short tasks only:** use a native subagent (Claude Agent tool, Codex native spawn) when the scope is narrow and the root checks and integrates the result in the same turn, such as a focused lookup or a small edit, or when non-overlapping owned scopes can run at the same time. Do not spawn when the child would reload the same large context, the scopes overlap, the current thread already has the needed files, or a nested agent would review or edit the same work
- **External agents** (Grok CLI, Codex CLI for Sol or Astra, Claude CLI from Codex): always `homebased` or T3 `delegate_task`, whatever the task size

When it is unclear whether a task is short, use `homebased`.

The transport references below still give the model flags, permission rules, prompt contract, and evidence capture to put in the homebased spec.

## Fan out and report once

When work splits into many parallel workers, such as coverage slices, a race between identical briefs, or a batch of measurements, give the user one report instead of a stream of status updates:

1. Before launching, state the done condition and the report the fan-out must return. For a race, declare the selection rule up front: first pass, rank all, or best of.
2. Launch every worker at once. Each brief stands alone and names its slice, how to verify, and the exact commits and method when it measures. Each worker reports `PASS`, `ISSUES`, or `BLOCKED` with evidence, and lists every issue it can prove, not only the first.
3. When the user asks one coordinator to run the fan-out, such as an Opus `high` agent that starts Haiku workers, that coordinator drains the workers and sends the single report. Grant it nested delegation in its brief.
4. Drop a result that lacks the commits or method its brief named and respawn that worker once. A second miss is a gap, and a gap is never a pass. If a worker drops out, continue without it and note it.
5. Report one compact table of results, one-line evidenced issues, gaps and dropouts, and the race rule when used. Do not paste raw worker output.

## Long commands inside a delegate

A delegate must not wait in the foreground for a long build, test suite, CI watch, benchmark, or training run. Waiting that way uses up its turn and context budget, and if the delegate dies or is cancelled, everything it learned dies with it. The delegate submits the command through `homebased` and owns the wait itself. The root never waits on a delegate's jobs and never passes their events on. How the delegate waits depends on its runner:

- **T3 `delegate_task` child on a Claude model.** The child persists between turns and can run background commands.
  1. Submit the command from the child's own session with `homebased task run` or `resource job submit`, so its events come back to the child and not the root.
  2. Start `homebased wait --next --final-only <ids>` as a background Bash command with `timeout: 7200000`, register it with `t3_background_wait`, and end the turn. `--final-only` still prints interim events, such as a preempted job, but exits only when a target ends. A wait accepts only ids submitted from the same session on the same machine.
  3. When the wait exits, handle the events, and before ending that turn start the wait again for every id whose final event has not come yet.
  4. Report only a final state: done, failed, or blocked with a reason the root must act on. Never end a turn with only "still waiting".

  While a background wait is live, T3 counts the child as waiting and does not wake the parent. A turn that ends with no live wait and no final result wakes the root for nothing.
- **T3 `delegate_task` child on a Codex model.** Codex cannot run a background command that wakes it, so every event would start a turn and wake the root. Give Codex children only work that finishes inside a turn, such as reviews, and send long commands to a Claude child.
- **homebased agent worker** (`HOMEBASED_TASK_ID` is set). Follow the homebased worker rules: submit the command, report `waiting` with notes on what each outcome means and what to do next, and exit. Homebased starts the continuation on its own, and the root does nothing. Do not write a `RESUME.md`.

Briefs point to this section instead of restating it. [orchestrate](../orchestrate/references/briefs.md) has a brief template that does this.

## Transport references

Read [delegate-run.md](references/delegate-run.md) for the shared run directory, prompt contract, and postflight capture, then only the reference for the transport in use:

- [claude-cli.md](references/claude-cli.md): Claude CLI for Opus 5.5, Haiku 5.5, or Sonnet 5.5 with an explicit effort
- [codex-native.md](references/codex-native.md): Codex `collaboration.spawn_agent` for Sol reviews and Astra
- [codex-cli.md](references/codex-cli.md): `codex exec` for Sol reviews and Astra outside Codex
- [grok-cli.md](references/grok-cli.md): Grok headless runs and permission preflight
