---
name: use-luna-max
description: Route implementation, tests, and mechanical repository work to internal GPT-5.6 Luna subagents at max reasoning. Use only when the user explicitly says "use Luna", "use Luna sub-agents", "use Luna Max", or invokes $use-luna-max. Do not infer this skill from a task that merely looks suitable for delegation.
---

# Use Luna Max

## Enforce the invocation gate

Activate this workflow only when the user explicitly names Luna or invokes this skill. Keep it active for the rest of the session until the user cancels it.

While active, send implementation work to Luna. Keep design and acceptance in the root thread. Do not silently substitute another model if Luna is unavailable.

## Route suitable work

Send Luna Max work that has a decided approach, a tight scope, and a cheap correctness check:

- easy, bounded implementation that follows an existing design
- tests and test extensions, including fixtures and table cases
- mechanical edits such as renames, signature changes, import moves, and API migrations
- inventories, classifications, and repeated transforms across many files
- boilerplate that follows an established repository pattern

Keep these responsibilities in the root thread:

- architecture, domain modeling, and API or UI surface design
- product intent, tradeoff analysis, and high-taste judgment
- subtle debugging, security judgment, and broad investigation
- review, integration, verification, and final acceptance

Decompose a large change into bounded passes after the root agent decides the design. Do not ask Luna to discover the architecture or correct an ambiguous instruction.

## Use internal subagents

Use Codex's internal subagent tools. Do not launch the Codex CLI.

If you are not Codex, read the [Codex CLI delegation reference](../subagent-workflow/references/codex-cli.md) and use that transport instead.

Select GPT-5.6 Luna with `max` reasoning when the tool needs an explicit model choice. If the configured worker already uses Luna Max, do not add redundant overrides. Start a fresh worker with only the task-local context unless continuity is necessary.

Give each worker a self-contained contract with:

1. one concrete objective and an observable success condition
2. the exact owned files, directories, or responsibility
3. excluded scope and unrelated changes to preserve
4. the decided approach and repository patterns to follow
5. evidence to inspect
6. exact verification commands and expected results
7. stop conditions for ambiguity, scope mismatch, or a required design choice
8. a final report with changed files, check results, blockers, and residual risks

Require the worker to read applicable `AGENTS.md` files, preserve concurrent work, stay inside its owned scope, and avoid commits, staging, publication, deployment, external writes, and nested subagents.

Do not give two writing workers overlapping scope. Use read-only workers for parallel evidence collection only when their results are independent.

## Review and verify

Treat Luna's report as evidence, not proof.

After each pass:

1. Inspect every changed file and the complete diff.
2. Reject changes outside the owned scope.
3. Run the relevant verification in the root thread.
4. Confirm that tests protect behavior or a non-obvious invariant instead of edited literals or implementation details.
5. Check for repetition, unnecessary abstractions, dead logic, missed cases, and repository convention violations.

The root agent owns correctness and the integrated result.

## Send repair passes

Send each concrete defect back to Luna while this workflow is active. Give the defect location, why it is wrong, the required end state, the owned scope, and the verification command. Do not prescribe a diff.

Use a follow-up on the same worker when its context remains useful. Start a fresh worker when independent reasoning or a clean context is more valuable. Inspect and verify every repair.

Take the work back into the root thread when two consecutive repair passes do not fix the same defect, or when the fix requires a design decision. State the reason when this happens.

Act directly only to remove a confirmed out-of-scope change made by the worker or to repair an urgent break that blocks other work. Report that action to the user.
