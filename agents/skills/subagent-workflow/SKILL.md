---
name: subagent-workflow
description: Provide multi-model routing and subagent workflow guidance for a Fable 5 root agent working with Fable 5, Opus 5, GPT-5.6 Sol, and GPT-5.6 Luna. Use only when the user explicitly asks for subagent-workflow by name. Do not infer its use from a general request for review, delegation, model selection, or subagent work.
disable-model-invocation: false
---

# Subagent Workflow

## Invocation gate

Use this skill only when the user explicitly requests `subagent-workflow` by name, including through `/subagent-workflow`. Do not invoke it merely because a task could benefit from delegation, model routing, review, or subagents.

Assume the caller is a Fable 5 root agent. Treat this skill as a small set of routing heuristics and operational guardrails, not a prescribed workflow. Use your own judgment for decomposition, topology, sequencing, delegation, and review. Remain accountable for the user's intent and the integrated result, and never treat a delegate's final message as proof that its work is correct.

## Route by intelligence, taste, and cost

Use these working definitions:

- **intelligence**: difficulty a model can handle unsupervised, including inferring intent and recovering from ambiguity
- **taste**: quality of UI/UX, code shape, API design, copy, and restraint
- **cost efficiency**: relative practical affordability, where a higher score is better

Treat the scores as routing heuristics, not benchmarks:

| Model        | Intelligence | Taste | Cost efficiency | Default role                                                                                                       |
| ------------ | -----------: | ----: | --------------: | ------------------------------------------------------------------------------------------------------------------ |
| Fable 5      |            9 |     9 |               2 | ambiguous architecture, planning, intent-sensitive work, high-taste review, and simplification                     |
| GPT-5.6 Sol  |            8 |     7 |               8 | persistent implementation, hard debugging, migrations, broad investigation, independent review                     |
| Opus 5       |            8 |     8 |               6 | high-taste review, deliberate second opinions, and full implementation substitute when the user directs "use opus" |
| GPT-5.6 Luna |            5 |     4 |              10 | repeated or high-volume mechanical transforms, classification, inventory, bulk processing, simple generated text   |

Read [references/model-routing.md](references/model-routing.md) before making a consequential or disputed model choice.

Apply these behavioral corrections:

- For Fable work, preserve the high-level goal, constraints, and authority boundaries. Whether you handle it directly or delegate it, check for early stopping, omitted requirements, and inferred intent overriding an explicit requirement.
- Watch for Sol overengineering: it can turn a small change into a rewrite with extra abstractions, speculative fallbacks, or excessive tests. Give it a narrow objective, explicit owned scope, and a minimality constraint. Require the smallest coherent change, preserve established abstractions, and stop to re-plan instead of piling on code when the approach is wrong.
- Use Opus 5 for long-horizon agentic implementation, second opinions, and high-taste review when Fable's intent inference and restraint are not required. Its taste relative to Fable is unproven at launch, so keep Fable on intent-sensitive surface design. Launch-era reports describe three failure modes: it stops early and reports unfinished work as done, it argues with explicit instructions, and it follows large instruction-dense skill files less reliably than short ones. Give it a focused self-contained prompt instead of a large instruction bundle, and verify completion against the success condition rather than trusting its report. Read [references/opus5-prompting.md](references/opus5-prompting.md) before writing Opus 5 prompts: unhobble style constraints, prefer judgment and rich references over rules and examples, use progressive disclosure, and keep authority, scope, and verification hard.
- Give Luna only tasks with an exact procedure and cheap verification. Do not ask it to choose architecture, infer product intent, or judge subtle code quality.

Honor an explicit user model choice. If that model is unavailable, report the failure instead of silently substituting another model.

Default every Codex delegation to GPT-5.6 Sol with `high` reasoning. Use Sol with `low` reasoning only for an easy, tightly scoped change whose correct result is cheap to verify. A single easy delegated change still uses Sol. Reserve Luna for repeated, high-volume, or cheap fan-out mechanical workloads, not merely because a Sol task is easy.

Sol is the default delegated implementer. When the user says "use opus", usually because Sol subscription limits are low, route delegated implementation to Opus 5 for the rest of the session as a full substitute: the same prompt contract, owned scope, verification, and no-commit rules, run through the Agent tool with model `opus` and `high` effort instead of the Codex CLI. "use sol" reaffirms the default. Absent a directive, Sol implements and Opus 5 serves review and second-opinion roles.

When Opus 5 implements, treat a report of "done" as unverified. Check the success condition and the diff before accepting a short run, re-prompt it to continue instead of integrating partial work, and state in the prompt that explicit requirements take precedence over its own view of the better approach. Write the prompt using [references/opus5-prompting.md](references/opus5-prompting.md), not a Sol-style instruction dump.

## Combine models

Use the models in whatever shape best fits the task. Fable's intent inference and restraint complement Sol's persistence; Opus 5 benchmarks near Fable at roughly half Fable's cost per task and can take either implementation or a taste-sensitive pass, though launch-era reports put its instruction following below that benchmark standing; Luna reduces the cost of repeated mechanical work. Decide whether to delegate, which model acts first, and how many passes are worthwhile from the actual evidence and risk.

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

Read [references/codex-cli.md](references/codex-cli.md) for preflight checks, prompt template, fresh `codex exec` commands, artifact capture, and postflight checks. When the delegate is Opus 5, also read [references/opus5-prompting.md](references/opus5-prompting.md) and write the prompt body to that guide.

## Run and integrate

For every Codex pass:

1. Start a fresh ephemeral `codex exec` invocation with an explicit model, reasoning effort, working directory, sandbox, and output file.
2. Capture baseline and postflight repository state, exit status, stdout, stderr, and the final message under `_scratch/subagent-workflow/<run-id>/`.
3. Reject edits outside owned scope and any read-only mutation.
4. Inspect the diff, verify important claims against source files, and run the repository's required checks independently.
5. Account for the selected model's known failure modes before integrating the result.

Choose follow-ups, additional reviewers, escalation, and repair paths using your best judgment. The orchestrator remains accountable for the integrated result. Never let a delegate commit, push, open or modify a pull request, deploy, or change external state unless the user separately authorizes that exact action.

## Review beyond the spec

A spec-conformance audit verifies the spec's checklist and therefore inherits the spec's blind spots. After conformance passes on a spec-driven build, run a separate adversarial review pass whose prompt explicitly ignores the spec and hunts failure scenarios:

- Re-derive inventories (consumers, call sites, config readers) by searching the code; never trust the spec's enumerations.
- Hunt lifecycle staleness: for every value captured at creation and used later, ask what happens when it changes in between. When one instance is found, fix the class, not the instance.
- Audit every error path for fail-open behavior. In privacy or security code, treat unlikely-but-fail-open as fix-worthy, not a documented residual risk.
- Require a timeout or cancellation path on every await a UI or state machine depends on.
- Report which behaviors only runtime or device testing can verify, so untested liveness paths are flagged instead of silently assumed covered.
