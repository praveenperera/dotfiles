---
name: subagent-workflow
description: Provide multi-model routing and subagent workflow guidance for a Fable 5, Opus 5, or GPT-5.6 Sol root agent working with Fable 5, Opus 5, GPT-5.6 Sol, and GPT-5.6 Luna. Use only when the user explicitly asks for subagent-workflow by name. Do not infer its use from a general request for review, delegation, model selection, or subagent work.
disable-model-invocation: false
---

# Subagent Workflow

## Invocation gate

Use this skill only when the user explicitly requests `subagent-workflow` by name, including through `/subagent-workflow`. Do not invoke it merely because a task could benefit from delegation, model routing, review, or subagents.

Detect the root model from the current session. Supported roots are **Fable 5**, **Opus 5**, and **GPT-5.6 Sol**. A session started from Claude Code has a Claude root; a session started from the Codex CLI has a Sol root. If the user names a root ("use opus as root", "opus root", "fable root"), honor that. If ambiguous, treat the model currently running this skill as root. Treat this skill as a small set of routing heuristics and operational guardrails, not a prescribed workflow. Use your own judgment for decomposition, topology, sequencing, delegation, and review. Remain accountable for the user's intent and the integrated result, and never treat a delegate's final message as proof that its work is correct.

## Root modes

Routing depends on who is root. Read the file for the active root, and do not read the other two:

| Root | Session | File |
| --- | --- | --- |
| Fable 5 | Claude Code running Fable 5 | [references/root-fable.md](references/root-fable.md) |
| Opus 5 | Claude Code running Opus 5 | [references/root-opus.md](references/root-opus.md) |
| GPT-5.6 Sol | Codex CLI | [references/root-sol.md](references/root-sol.md) |

Each file holds that root's routing table, ownership defaults, transport, and cautions. Everything else in this skill and in [references/model-routing.md](references/model-routing.md) applies to every root.

Shared defaults, whoever is root: Sol at `high` reasoning is the default implementer for substantial work; Luna at `max` reasoning takes easy tightly scoped delegations and tests; Luna at `low` keeps bulk mechanical work; no root is the final authority on the taste and simplification of its own output; honor explicit user model choices.

## Route by intelligence, taste, and cost

Use these working definitions:

- **intelligence**: difficulty a model can handle unsupervised, including inferring intent and recovering from ambiguity
- **taste**: quality of UI/UX, code shape, API design, copy, and restraint
- **cost efficiency**: relative practical affordability, where a higher score is better

Treat the scores as routing heuristics, not benchmarks:

| Model        | Intelligence | Taste | Cost efficiency | Default role                                                                                                       |
| ------------ | -----------: | ----: | --------------: | ------------------------------------------------------------------------------------------------------------------ |
| Fable 5      |            9 |     9 |               2 | intent-sensitive work, high-taste review, and simplification; the default taste authority under every root |
| GPT-5.6 Sol  |            8 |     7 |               8 | persistent implementation, hard debugging, migrations, broad investigation, independent review                     |
| Opus 5       |            8 |     8 |               6 | orchestration and long-horizon agentic work; deliberate second opinions; delegated implementation when the user directs "use opus" |
| GPT-5.6 Luna `max` |      7 |     4 |               9 | easy tightly scoped delegated changes, tests, and one-off mechanical work. Accurate and literal: the result tracks the quality of the diagnosis, which stays with the root |
| GPT-5.6 Luna `low` |      5 |     4 |              10 | repeated or high-volume mechanical transforms, classification, inventory, bulk processing, simple generated text |

Luna's intelligence is effort-dependent: after the July 30, 2026 80% price cut, Luna at `max` reasoning reaches roughly Sol-`medium` capability at a small fraction of Sol's cost, so work that fits below Sol `high` routes to Luna `max` instead of Sol `low` or `medium`.

Read [references/model-routing.md](references/model-routing.md) before making a consequential or disputed model choice.

Apply these behavioral corrections:

- For Fable work, preserve the high-level goal, constraints, and authority boundaries. Whether the root is Fable or a Fable subagent, check for early stopping, omitted requirements, and inferred intent overriding an explicit requirement.
- Watch for Sol overengineering: it can turn a small change into a rewrite with extra abstractions, speculative fallbacks, or excessive tests. Give it a narrow objective, explicit owned scope, and a minimality constraint. Require the smallest coherent change, preserve established abstractions, and stop to re-plan instead of piling on code when the approach is wrong.
- Use Opus 5 for long-horizon agentic implementation, complex debugging, and deliberate second opinions. Never assign Opus the high-taste review or simplification of its own output; send that to Fable. Launch-era reports describe three Opus failure modes: it stops early and reports unfinished work as done, it argues with explicit instructions, and it follows large instruction-dense skill files less reliably than short ones. Give Opus a focused self-contained prompt instead of a large instruction bundle, and verify completion against the success condition rather than trusting its report. Read [references/opus5-prompting.md](references/opus5-prompting.md) before writing Opus 5 prompts: unhobble style constraints, prefer judgment and rich references over rules and examples, use progressive disclosure, and keep authority, scope, and verification hard.
- Give Luna tasks with cheap verification: exact-procedure mechanical work at `low` reasoning, or an easy tightly scoped change at `max` reasoning. Do not ask it to choose architecture, infer product intent, or judge subtle code quality. Luna is accurate and literal, so the quality of the result tracks the quality of the diagnosis, which stays with the root agent. It does what the prompt says and no more: it will write an assertion that another assertion already implies, because you asked for it. State the required end state precisely, and expect no correction of a flawed instruction.

Honor an explicit user model choice. If that model is unavailable, report the failure instead of silently substituting another model.

Default every substantial Codex delegation to GPT-5.6 Sol with `high` reasoning; do not run Sol below `high`. Route an easy, tightly scoped change whose correct result is cheap to verify to Luna with `max` reasoning: since the July 2026 price cut, Luna `max` reasons near Sol `medium` at a small fraction of the cost. Use Luna with `low` reasoning only for repeated, high-volume, or cheap fan-out mechanical workloads, where `max` wastes tokens and latency. Luna has no other operating point in this workflow: `max` for one-off delegations, `low` at volume.

Sol is the default delegated implementer under a Claude root, and the root itself under a Sol root. Session directives:

- **"use sol"** reaffirms Sol as the delegated implementer.
- **"use opus"** routes delegated implementation to Opus 5 for the rest of the session: same prompt contract, owned scope, verification, and no-commit rules. From a Fable root, run that through the Agent tool with model `opus` and `high` effort. From an Opus root, prefer implementing in the root thread when continuity helps; use an Opus Agent subagent only for an isolated owned scope that should not share the root context.
- **"use fable"** (Opus or Sol root) does not make Fable the implementer by default; it reaffirms Fable for review, taste, surface design, and simplification. If the user clearly wants Fable to implement, honor that as an explicit model choice for that task.
- **"use luna"** (any root) routes implementation to Luna at `max` reasoning for the rest of the session. The root states the design, and reviews and verifies every pass. See the `use-luna-max` skill for that contract.

Absent a directive, Sol implements. The review, taste, and second-opinion assignments that go with each directive are in the active root's file.

When Opus 5 implements (root or subagent), treat a report of "done" as unverified. Check the success condition and the diff before accepting a short run, re-prompt it to continue instead of integrating partial work, and state in the prompt that explicit requirements take precedence over its own view of the better approach. Write Opus prompts using [references/opus5-prompting.md](references/opus5-prompting.md), not a Sol-style instruction dump.

When Fable reviews or simplifies as a subagent, give it the same shared prompt contract as other delegates: owned scope, excluded scope, evidence (diff and paths), success condition, and no-commit rules. Prefer a short judgment-oriented prompt over a checklist dump. Counter Fable laziness with an explicit completion bar (what must be covered, what must not be omitted).

## Combine models

Use the models in whatever shape best fits the task and the active root. Fable's intent inference and restraint complement Sol's persistence. Opus 5 is a strong orchestrator and implementer at lower cost than Fable, but launch-era instruction following is weaker than its benchmark standing. No root reviews its own taste: Opus needs Fable because its restraint is unproven, and Sol needs Fable because it overbuilds and the model that wrote the extra abstraction is the least likely to see it. Luna reduces the cost of repeated mechanical work and, at `max` reasoning, of easy bounded delegations. Decide whether to delegate, which model acts first, and how many passes are worthwhile from the actual evidence and risk.

Do not delegate when handoff and reintegration cost more than the task. Do not give two writers overlapping ownership or let parallel implementations edit the same files. Read-only reviewers may inspect shared repository state.

## Prepare the delegation

Choose exactly one mode:

- **read-only analysis**: inspect and report; use a read-only sandbox
- **implementation**: edit only the owned scope; use a workspace-write sandbox

Write a self-contained prompt that includes:

1. objective and observable success condition
2. authority and exact owned scope
3. excluded scope and unrelated changes to preserve
4. evidence to inspect
5. required verification
6. stop conditions
7. final report requirements

Preserve the user's constraints verbatim when possible. Require the delegate to read applicable `AGENTS.md` files, inspect relevant context before acting, do its own work without spawning subagents or nested agents, avoid commits and external writes, and report changed files, verification, blockers, and residual risks.

Read [references/codex-cli.md](references/codex-cli.md) for preflight checks, prompt template, fresh `codex exec` commands, artifact capture, and postflight checks. When the delegate is Opus 5, also read [references/opus5-prompting.md](references/opus5-prompting.md) and write the prompt body to that guide. When the delegate is Fable 5 via the Agent tool, use the same short contract shape as Opus: hard authority and verification, soft style, progressive references. A Claude delegate reached from a Sol root through the `claude` CLI takes the same contract; only the transport differs.

## Run and integrate

For every Codex pass:

1. Start a fresh ephemeral `codex exec` invocation with an explicit model, reasoning effort, working directory, sandbox, and output file.
2. Capture baseline and postflight repository state, exit status, stdout, stderr, and the final message under `_scratch/subagent-workflow/<run-id>/`.
3. Reject edits outside owned scope and any read-only mutation.
4. Inspect the diff, verify important claims against source files, and run the repository's required checks independently.
5. Account for the selected model's known failure modes before integrating the result.

For every Claude Agent-tool pass (Opus or Fable subagent):

1. Use an explicit model (`opus` or `fable`) and `high` effort unless the user directed otherwise.
2. Keep the prompt self-contained; do not rely on the subagent inheriting this skill or the root's full context.
3. Capture the final message and, for implementation, the resulting repository state under `_scratch/subagent-workflow/<run-id>/`.
4. Apply the same scope rejection, independent verification, and failure-mode accounting as for Codex.

Choose follow-ups, additional reviewers, escalation, and repair paths using your best judgment. The orchestrator remains accountable for the integrated result. Never let a delegate commit, push, open or modify a pull request, deploy, or change external state unless the user separately authorizes that exact action.

## Repair defects through the delegate

When review finds a defect in delegated work, send it back to the delegate that wrote the code. The root finds the defect; the delegate fixes it. This keeps the root's context on judgment rather than on typing, and it keeps the cost of the repair at the delegate's price.

Start a new ephemeral pass with the same prompt contract. In the prompt:

- name each defect and its exact location, such as file and line or function
- state why it is wrong, in one sentence
- state the required end state, not the keystrokes
- keep the owned scope to the files that hold the defects
- repeat the original verification, and require that the earlier work still passes

Do not send a diff to apply. The delegate must make the change and verify it. Verify the repair yourself, the same way as the first pass.

Repeat while each pass removes real defects. Take the work into the root thread when two passes in a row fail to fix the same defect, when the fix needs a design decision the delegate cannot make, or when the handoff now costs more than the repair. Say so when you do.

The root acts directly in these cases:

- a change outside the owned scope: revert it at once, then state the boundary in the next pass
- an urgent break, such as a broken build blocking other work
- final taste, surface design, and simplification, which stay with the model that owns them under the active root

A delegate that repeats the same defect after a clear repair prompt is a routing signal. Move the work to a stronger model instead of sending a third pass.

## Review beyond the spec

A spec-conformance audit verifies the spec's checklist and therefore inherits the spec's blind spots. After conformance passes on a spec-driven build, run a separate adversarial review pass whose prompt explicitly ignores the spec and hunts failure scenarios:

- Re-derive inventories (consumers, call sites, config readers) by searching the code; never trust the spec's enumerations.
- Hunt lifecycle staleness: for every value captured at creation and used later, ask what happens when it changes in between. When one instance is found, fix the class, not the instance.
- Audit every error path for fail-open behavior. In privacy or security code, treat unlikely-but-fail-open as fix-worthy, not a documented residual risk.
- Require a timeout or cancellation path on every await a UI or state machine depends on.
- Report which behaviors only runtime or device testing can verify, so untested liveness paths are flagged instead of silently assumed covered.

Prefer Fable for the taste-sensitive leg of that review, on design and surface quality, and Sol for the inventory leg, on re-derivation and missed-case hunting. Never let the root take the adversarial pass on its own implementation.
