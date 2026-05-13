---
name: goal-ready-spec
description: Use when turning a chat, rough plan, design discussion, or requirements dump into a goal-ready implementation spec for Codex or another agent. Trigger on requests for a full spec, execution-ready plan, goal-ready plan, guarded implementation plan, or a plan that must preserve architecture, ownership, state placement, acceptance criteria, and completion-audit guardrails.
---

# Goal-Ready Spec

## Overview

Create specs that an execution agent can follow without replacing explicit requirements with merely equivalent behavior. The output must make architectural ownership, state location, scope boundaries, verification commands, and completion evidence auditable before any goal is marked complete.

## Workflow

1. Read the user request, prior plan text, and relevant project instructions before writing the spec.
2. If working inside a repository, inspect existing docs/config/code that affect the requested work. Do not write from general assumptions when project files can answer the question.
3. Extract every explicit requirement into a checklist item. Treat architecture, ownership, lifecycle, state placement, API shape, generated-file expectations, and test requirements as first-class requirements.
4. Identify any requirement where the user asked for a specific component, module, actor, manager, owner, or data model. Mark these as "no equivalent substitution" requirements.
5. Ask before finalizing if scope, ownership, state placement, API shape, or verification requirements are too ambiguous to make the spec executable. Only leave non-blocking uncertainty in Risks and Open Questions.
6. Write the spec to `_plans/<short-slug>/spec.md` only when the user requests a saved spec or the repo-local `_plans` workflow explicitly requires one. Otherwise, provide the spec in the response and state that no file was written.
7. Before finalizing, audit the spec against the source request and confirm every material requirement appears in an acceptance criterion, completion criterion, or completion-audit item.

## Required Spec Sections

Use this structure unless the user asks for a different format:

- Objective: the concrete end state in one short paragraph
- Source Context: files, docs, plans, and user statements the spec is based on
- Scope: what must change
- Non-Goals: what must not change
- Hard Requirements: exact requirements copied or faithfully paraphrased from the plan
- Architecture and Ownership Invariants: where state, lifecycle, orchestration, APIs, and boundaries must live
- No Equivalent Substitutions: requirements where matching behavior is insufficient unless the named owner/design is implemented
- Execution Contract: the implementation goal must reference this spec by path or title and treat it as the controlling source of truth
- Implementation Plan: ordered phases with files or modules likely involved
- Deviation Protocol: what requires stopping to ask before implementation continues
- Verification Plan: exact commands, tests, generated outputs, manual checks, and expected evidence
- Completion Audit: checklist the execution agent must fill out before committing or marking the goal complete
- Completion Criteria: what must be true before the goal is complete, including every Completion Audit item filled with concrete evidence
- Acceptance Criteria: behavior plus architecture plus completed checklist evidence, not behavior alone
- Risks and Open Questions: unresolved assumptions that must be answered before or during execution

## Guardrails

- If the plan says component X owns state, lifecycle, cadence, cancellation, retries, persistence, reconciliation, or API shape, the spec must require X to own it.
- Passing tests or matching user-visible behavior does not satisfy an explicit architecture or ownership requirement by itself.
- The implementation goal must cite the spec path or title and must not treat the spec as optional background context.
- The implementation agent must ask before moving ownership, changing boundaries, replacing an actor/manager/module, weakening acceptance criteria, or skipping verification.
- The goal is not complete until the Completion Audit checklist is filled out with concrete evidence for every item.
- The completion audit must include a "Deviations from plan" section. If there are no deviations, it must explicitly say so.
- Do not phrase critical checks as vague intentions like "verify it works". Name the command, file, assertion, generated artifact, or manual observation that proves the requirement.
- If a requirement cannot yet be verified and is not blocking, mark it as a risk or open question instead of burying it in the implementation plan.

## Completion Audit Template

Include this template in generated specs:

```markdown
## Completion Audit

Before marking this goal complete or creating a commit, fill out this checklist:

- Spec Reference: cite the spec path or title this implementation followed
- Requirement Trace: map each Hard Requirement to the file, test, command output, or manual evidence that proves it
- Ownership Trace: show where each Architecture and Ownership Invariant is implemented
- No Equivalent Substitutions: confirm no named owner/design was replaced by behavior-only equivalence
- Faithful Execution: compare the final implementation against the spec and confirm every required section was followed, or list approved deviations
- Verification Results: list every command/manual check run and the result
- Generated Artifacts: list generated files updated or explain why none were needed
- Completion Criteria: confirm every Completion Criteria item is satisfied with evidence
- Deviations from Plan: list every deviation and the user approval for it, or state "none"
- Residual Risk: list any remaining uncertainty, skipped test, or environment limitation
```

## Goal Execution Handoff

When the user asks Codex to execute one of these specs as a goal, the implementation goal should cite the spec path or title. The execution agent should first read the full spec, project instructions, and referenced files, then treat the spec as the controlling source of truth. It should maintain the Completion Audit as a live checklist and refuse to mark the goal complete until every audit item and completion criterion is filled with concrete evidence.
