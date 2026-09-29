---
name: use-sol
description: Route implementation, tests, investigation, and mechanical repository work to GPT-6.1 Sol subagents at low or high reasoning. Use only when the user explicitly says "use Sol", "use Sol sub-agents", "use Sol 6.1", or invokes $use-sol. Do not infer this skill from a task that merely looks suitable for delegation.
---

# Use Sol

## Enforce the invocation gate

Activate this workflow only when the user asks to use Sol for implementation or explicitly invokes this skill. A statement that Sol is the current root, a model comparison, or a request to edit this skill does not activate it. Keep it active for the rest of the session until the user cancels it.

While active, send implementation work to Sol 6.1. Keep design and acceptance in the root thread. Do not silently substitute another model if Sol 6.1 is unavailable.

## Route suitable work

Send Sol work that has a decided approach, a tight scope, and a cheap correctness check.

At `low` reasoning:

- mechanical edits such as renames, signature changes, import moves, and API migrations along a decided pattern
- fixtures, table test cases, and boilerplate that follows an established repository pattern
- inventories, classifications, and repeated transforms across many files

At `high` reasoning:

- bounded implementation that follows an existing design
- new tests and test extensions that need edge-case judgment
- bug investigation and root-cause analysis that report findings without editing

Keep these responsibilities in the root thread:

- architecture, domain modeling, and API or UI surface design
- product intent, tradeoff analysis, and high-taste judgment, including front-end and UI design
- security judgment and the decision on what an investigation found
- long unattended builds and heavy rewrites, which Sol can grind on without progress
- review, integration, verification, and final acceptance

Decompose a large change into bounded passes after the root agent decides the design. Do not ask Sol to discover the architecture or correct an ambiguous instruction.

## Use internal subagents

If you are Codex, spawn with [codex-native.md](../use-agents/references/codex-native.md). If you are not Codex, use the [Codex CLI delegation reference](../use-agents/references/codex-cli.md).

Select GPT-6.1 Sol (`gpt-6.1-sol`) with the reasoning level from the list above when the tool needs an explicit model choice. Use `xhigh` only for a deep audit, and do not use `max`. If a `low` pass fails its check or needs a judgment call, rerun it at `high`. If the configured worker already uses Sol 6.1, do not add redundant overrides. Start a fresh worker with only the task-local context unless continuity is necessary.

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

Treat Sol's report as evidence, not proof.

After each pass:

1. Inspect every changed file and the complete diff.
2. Reject changes outside the owned scope.
3. Complete required verification on the integrated code. Reuse reliable results for unchanged code; rerun checks in the root thread for new changes, failures, missing evidence, or unresolved concerns.
4. Confirm that tests protect behavior or a non-obvious invariant instead of edited literals or implementation details.
5. Check for repetition, unnecessary abstractions, dead logic, missed cases, and repository convention violations.

The root agent owns correctness and the integrated result.

## Send repair passes

Send each concrete defect back to Sol while this workflow is active. Give the defect location, why it is wrong, the required end state, the owned scope, and the verification command. Do not prescribe a diff.

Use a follow-up on the same worker when its context remains useful. Start a fresh worker when independent reasoning or a clean context is more valuable. Inspect and verify every repair.

Take the work back into the root thread when two consecutive repair passes do not fix the same defect, or when the fix requires a design decision. State the reason when this happens.

Act directly only to remove a confirmed out-of-scope change made by the worker or to repair an urgent break that blocks other work. Report that action to the user.
