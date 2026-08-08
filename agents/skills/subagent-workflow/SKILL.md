---
name: subagent-workflow
description: Provide multi-model routing and subagent workflow guidance for a Fable 5 or Opus 5 root agent working with Fable 5, Opus 5, GPT-5.6 Sol, and GPT-5.6 Luna. Use only when the user explicitly asks for subagent-workflow by name. Do not infer its use from a general request for review, delegation, model selection, or subagent work.
disable-model-invocation: false
---

# Subagent Workflow

## Invocation gate

Use this skill only when the user explicitly requests `subagent-workflow` by name, including through `/subagent-workflow`. Do not invoke it merely because a task could benefit from delegation, model routing, review, or subagents.

Detect the root model from the current session. Supported roots are **Fable 5** and **Opus 5**. If the user names a root ("use opus as root", "opus root", "fable root"), honor that. If ambiguous, treat the model currently running this skill as root. Treat this skill as a small set of routing heuristics and operational guardrails, not a prescribed workflow. Use your own judgment for decomposition, topology, sequencing, delegation, and review. Remain accountable for the user's intent and the integrated result, and never treat a delegate's final message as proof that its work is correct.

## Root modes

Routing depends on who is root. Shared defaults: Sol at `high` reasoning is the default delegated implementer for substantial work; Luna at `max` reasoning takes easy tightly scoped delegations; Luna at `low` keeps bulk mechanical work; honor explicit user model choices.

### Fable 5 root

Fable keeps work that needs its intent inference and restraint. Do it in the root thread when practical; do not spend Fable as a subagent on work the root can finish cheaply.

| Work | Route |
| --- | --- |
| Ambiguous architecture, planning, intent-sensitive product decisions | Fable root |
| Public APIs, SDK shape, UI/UX, copy, final simplification | Fable root |
| Substantial bounded implementation, hard debugging, migrations | Sol via Codex (`high` by default) |
| Independent adversarial or inventory-style review | Sol via Codex, or Opus when a deliberate second opinion helps |
| High-taste second opinion when the root already shaped the design | Opus 5 via Agent tool (`model: opus`, `high` effort) |
| Full implementation substitute when the user says "use opus" | Opus 5 via Agent tool (session-long until "use sol") |
| Easy, tightly scoped change with cheap verification | Luna via Codex (`max` reasoning) |
| Repeated high-volume mechanical work | Luna via Codex (`low`–`medium` reasoning) |

### Opus 5 root

Opus orchestrates and can implement. **Do not use the Opus root as the final authority on taste, surface design, or simplification of its own work.** Route those to Fable.

| Work | Route |
| --- | --- |
| Orchestration, long-horizon agentic work, complex debugging, root-cause analysis | Opus root |
| Substantial bounded implementation, hard debugging, migrations | Sol via Codex (`high` by default) |
| Implementation when the user says "use opus", or when Opus already owns the thread and Sol is a poor fit | Opus root directly, or Opus via Agent tool for an isolated owned scope |
| High-taste review, UI/UX/API/copy judgment, intent-sensitive surface design | Fable 5 via Agent tool (`model: fable`, `high` effort) |
| Final simplification of a Sol (or Opus) implementation | Fable 5 via Agent tool |
| Independent cross-vendor or inventory-style review | Sol via Codex |
| Easy, tightly scoped change with cheap verification | Luna via Codex (`max` reasoning) |
| Repeated high-volume mechanical work | Luna via Codex (`low`–`medium` reasoning) |

When the Opus root would ordinarily do a taste pass itself under the Fable-root table, spawn Fable instead. Keep the Fable prompt short and self-contained; point at the diff, owned paths, and a success condition rather than dumping this skill file.

## Route by intelligence, taste, and cost

Use these working definitions:

- **intelligence**: difficulty a model can handle unsupervised, including inferring intent and recovering from ambiguity
- **taste**: quality of UI/UX, code shape, API design, copy, and restraint
- **cost efficiency**: relative practical affordability, where a higher score is better

Treat the scores as routing heuristics, not benchmarks:

| Model        | Intelligence | Taste | Cost efficiency | Default role                                                                                                       |
| ------------ | -----------: | ----: | --------------: | ------------------------------------------------------------------------------------------------------------------ |
| Fable 5      |            9 |     9 |               2 | intent-sensitive work, high-taste review, and simplification (root when Fable is root; Agent `fable` when Opus is) |
| GPT-5.6 Sol  |            8 |     7 |               8 | persistent implementation, hard debugging, migrations, broad investigation, independent review                     |
| Opus 5       |            8 |     8 |               6 | root orchestration and long-horizon work when Opus is root; second opinions or "use opus" implementation when Fable is root |
| GPT-5.6 Luna | 5 (7 at `max`) |     4 |              10 | easy tightly scoped delegated changes at `max` reasoning; repeated or high-volume mechanical transforms, classification, inventory, bulk processing, simple generated text at `low` |

Luna's intelligence is effort-dependent: after the July 30, 2026 80% price cut, Luna at `max` reasoning reaches roughly Sol-`medium` capability at a small fraction of Sol's cost, so work that fits below Sol `high` routes to Luna `max` instead of Sol `low` or `medium`.

Read [references/model-routing.md](references/model-routing.md) before making a consequential or disputed model choice.

Apply these behavioral corrections:

- For Fable work, preserve the high-level goal, constraints, and authority boundaries. Whether the root is Fable or a Fable subagent, check for early stopping, omitted requirements, and inferred intent overriding an explicit requirement.
- Watch for Sol overengineering: it can turn a small change into a rewrite with extra abstractions, speculative fallbacks, or excessive tests. Give it a narrow objective, explicit owned scope, and a minimality constraint. Require the smallest coherent change, preserve established abstractions, and stop to re-plan instead of piling on code when the approach is wrong.
- Use Opus 5 for long-horizon agentic implementation and complex debugging. When Fable is root, also use Opus for deliberate second opinions and as the full implementation substitute when the user directs "use opus". When Opus is root, do not assign Opus the high-taste review or simplification of its own output; send that to Fable. Launch-era reports describe three Opus failure modes: it stops early and reports unfinished work as done, it argues with explicit instructions, and it follows large instruction-dense skill files less reliably than short ones. Give Opus a focused self-contained prompt instead of a large instruction bundle, and verify completion against the success condition rather than trusting its report. Read [references/opus5-prompting.md](references/opus5-prompting.md) before writing Opus 5 prompts: unhobble style constraints, prefer judgment and rich references over rules and examples, use progressive disclosure, and keep authority, scope, and verification hard.
- Give Luna tasks with cheap verification: exact-procedure mechanical work at `low` reasoning, or an easy tightly scoped change at `max` reasoning. Do not ask it to choose architecture, infer product intent, or judge subtle code quality.

Honor an explicit user model choice. If that model is unavailable, report the failure instead of silently substituting another model.

Default every substantial Codex delegation to GPT-5.6 Sol with `high` reasoning; do not run Sol below `high`. Route an easy, tightly scoped change whose correct result is cheap to verify to Luna with `max` reasoning: since the July 2026 price cut, Luna `max` reasons near Sol `medium` at a small fraction of the cost. Use Luna with `low` reasoning for repeated, high-volume, or cheap fan-out mechanical workloads, where `max` wastes tokens and latency.

Sol is the default delegated implementer under both roots. Session directives:

- **"use sol"** reaffirms Sol as the delegated implementer.
- **"use opus"** routes delegated implementation to Opus 5 for the rest of the session: same prompt contract, owned scope, verification, and no-commit rules. From a Fable root, run that through the Agent tool with model `opus` and `high` effort. From an Opus root, prefer implementing in the root thread when continuity helps; use an Opus Agent subagent only for an isolated owned scope that should not share the root context.
- **"use fable"** (Opus root) does not make Fable the implementer by default; it reaffirms Fable for review, taste, surface design, and simplification. If the user clearly wants Fable to implement, honor that as an explicit model choice for that task.

Absent a directive, Sol implements. Under a Fable root, Opus serves review and second-opinion roles unless "use opus" is active. Under an Opus root, Fable serves review, taste, and simplification roles.

When Opus 5 implements (root or subagent), treat a report of "done" as unverified. Check the success condition and the diff before accepting a short run, re-prompt it to continue instead of integrating partial work, and state in the prompt that explicit requirements take precedence over its own view of the better approach. Write Opus prompts using [references/opus5-prompting.md](references/opus5-prompting.md), not a Sol-style instruction dump.

When Fable reviews or simplifies as a subagent, give it the same shared prompt contract as other delegates: owned scope, excluded scope, evidence (diff and paths), success condition, and no-commit rules. Prefer a short judgment-oriented prompt over a checklist dump. Counter Fable laziness with an explicit completion bar (what must be covered, what must not be omitted).

## Combine models

Use the models in whatever shape best fits the task and the active root. Fable's intent inference and restraint complement Sol's persistence. Opus 5 is a strong orchestrator and implementer at lower cost than Fable, but launch-era instruction following is weaker than its benchmark standing; under an Opus root, pair it with Fable for taste rather than trusting Opus to be its own taste authority. Luna reduces the cost of repeated mechanical work and, at `max` reasoning, of easy bounded delegations. Decide whether to delegate, which model acts first, and how many passes are worthwhile from the actual evidence and risk.

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

Read [references/codex-cli.md](references/codex-cli.md) for preflight checks, prompt template, fresh `codex exec` commands, artifact capture, and postflight checks. When the delegate is Opus 5, also read [references/opus5-prompting.md](references/opus5-prompting.md) and write the prompt body to that guide. When the delegate is Fable 5 via the Agent tool, use the same short contract shape as Opus: hard authority and verification, soft style, progressive references.

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

## Review beyond the spec

A spec-conformance audit verifies the spec's checklist and therefore inherits the spec's blind spots. After conformance passes on a spec-driven build, run a separate adversarial review pass whose prompt explicitly ignores the spec and hunts failure scenarios:

- Re-derive inventories (consumers, call sites, config readers) by searching the code; never trust the spec's enumerations.
- Hunt lifecycle staleness: for every value captured at creation and used later, ask what happens when it changes in between. When one instance is found, fix the class, not the instance.
- Audit every error path for fail-open behavior. In privacy or security code, treat unlikely-but-fail-open as fix-worthy, not a documented residual risk.
- Require a timeout or cancellation path on every await a UI or state machine depends on.
- Report which behaviors only runtime or device testing can verify, so untested liveness paths are flagged instead of silently assumed covered.

Under an Opus root, prefer Fable for taste-sensitive adversarial review of design and surface quality, and Sol when the pass is mainly inventory re-derivation and missed-case hunting. Under a Fable root, Opus or Sol can take the independent pass; keep Fable on the final simplification when the implementation was not already simplified in-root.
