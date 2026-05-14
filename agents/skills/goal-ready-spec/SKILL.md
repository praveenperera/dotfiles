---
name: goal-ready-spec
description: Use only when the user explicitly invokes $goal-ready-spec or specifically asks to make a spec goal-ready. Creates guarded, auditable implementation specs for Codex or another agent.
---

# Goal-Ready Spec

## Overview

Create specs that an execution agent can follow without replacing explicit requirements with merely equivalent behavior. The goal is to collaborate with the user until the spec is clear, executable, and faithful to their intent, not to finish a spec in one pass. The output must make architectural ownership, state location, scope boundaries, verification commands, and completion evidence auditable before any goal is marked complete.

Do not use this skill for ordinary planning, brainstorming, outlining, refining, reviewing, or implementation-plan requests unless the user explicitly invokes `$goal-ready-spec` or specifically asks to make the spec goal-ready.

## Workflow

1. Read the user request, prior plan text, and relevant project instructions before writing the spec.
2. If working inside a repository, inspect existing docs/config/code that affect the requested work. Do not write from general assumptions when project files can answer the question.
3. Classify requirements before writing the spec. Treat requirements as binding only when they are current, unrejected, and material to the requested goal. Do not promote examples, discarded alternatives, speculative ideas, or stale plan text into Hard Requirements.
4. Treat architecture, ownership, lifecycle, state placement, API shape, generated-file expectations, and test requirements as first-class requirements when they are binding.
5. Identify any binding requirement where the user asked for a specific component, module, actor, manager, owner, or data model. Mark these as "no equivalent substitution" requirements.
6. Ask focused questions and incorporate the answers before finalizing whenever missing information would prevent a clear, executable, faithful spec. Only leave non-blocking uncertainty in Risks and Open Questions.
7. Write the spec to `_plans/<short-slug>/spec.md` only when the user requests a saved spec or the repo-local `_plans` workflow explicitly requires one. Otherwise, provide the spec in the response and state that no file was written.
8. Before finalizing, audit the spec against the source request and confirm every binding material requirement appears in an acceptance criterion, completion criterion, or completion-audit item.

## Required Spec Sections

Use this structure unless the user asks for a different format:

- Objective: the concrete end state in one short paragraph
- Source Context: files, docs, plans, and user statements the spec is based on
- Scope: what must change
- Non-Goals: what must not change
- Requirement Classification: binding requirements, inferred requirements, non-goals or rejected alternatives, and open questions
- Hard Requirements: binding requirements copied or faithfully paraphrased from the plan
- Architecture and Ownership Invariants: where state, lifecycle, orchestration, APIs, and boundaries must live
- No Equivalent Substitutions: requirements where matching behavior is insufficient unless the named owner/design is implemented
- Execution Contract: the implementation goal must reference this spec by path or title and treat it as the controlling implementation contract
- Implementation Plan: ordered phases with files or modules likely involved
- Deviation Protocol: what requires stopping to ask before implementation continues
- Verification Plan: exact commands, tests, generated outputs, manual checks, and expected evidence
- Completion Audit: required `## Completion Audit` section the execution agent must fill out before committing or marking the goal complete
- Completion Criteria: process requirements before the goal is complete, including verification results and every Completion Audit item filled with concrete evidence or marked N/A with a reason
- Acceptance Criteria: behavior plus required architecture, not behavior alone
- Risks and Open Questions: unresolved assumptions that must be answered before or during execution

## Guardrails

- If the user explicitly requires component X to own state, lifecycle, cadence, cancellation, retries, persistence, reconciliation, or API shape, the spec must require X to own it.
- Passing tests or matching user-visible behavior does not satisfy an explicit architecture or ownership requirement by itself.
- The spec is the controlling implementation contract for the goal unless it conflicts with newer user instructions, repository instructions, or facts discovered during implementation.
- The implementation goal must cite the spec path or title and must not treat the spec as optional background context.
- The implementation agent must ask before moving ownership, changing boundaries, replacing an actor/manager/module, weakening acceptance criteria, or skipping verification.
- If following the spec exactly becomes incorrect or impossible, the implementation agent must stop and ask before deviating.
- If the spec is not ready yet, ask the user focused questions and work with them until it is ready instead of finalizing a weak draft.
- The goal is not complete until the Completion Audit checklist is filled out with concrete evidence for every item or an N/A reason for irrelevant items.
- The completion audit must include a "Deviations from plan" section. If there are no deviations, it must explicitly say so.
- Do not phrase critical checks as vague intentions like "verify it works". Name the command, file, assertion, generated artifact, or manual observation that proves the requirement.
- If a requirement cannot yet be verified and is not blocking, mark it as a risk or open question instead of burying it in the implementation plan.

## Completion Audit Template

Every generated spec must include this `## Completion Audit` section. Executing the goal includes completing the section with evidence; the goal is incomplete until the audit is filled out or each irrelevant item is marked N/A with a reason.

```markdown
## Completion Audit

Before marking this goal complete or creating a commit, fill out this checklist. Every item must include concrete evidence or an N/A reason:

- Spec Reference: cite the spec path or title this implementation followed
- Requirement Trace: map each Hard Requirement to the file, test, command output, or manual evidence that proves it
- Ownership Trace: show where each Architecture and Ownership Invariant is implemented
- No Equivalent Substitutions: confirm no named owner/design was replaced by behavior-only equivalence
- Faithful Execution: compare the final implementation against the spec and confirm every required section was followed, or list approved deviations
- Verification Results: list every command/manual check run and the result
- Generated Artifacts: list generated files updated or explain why none were needed
- Completion Criteria: confirm every Completion Criteria item is satisfied with evidence or marked N/A with a reason
- Deviations from Plan: list every deviation and the user approval for it, or state "none"
- Residual Risk: list any remaining uncertainty, skipped test, or environment limitation
```

## Goal Execution Handoff

When the user asks Codex to execute one of these specs as a goal, the implementation goal should cite the spec path or title. The execution agent should first read the full spec, project instructions, and referenced files, then treat the spec as the controlling implementation contract unless it conflicts with newer user instructions, repository instructions, or implementation facts. It should maintain the Completion Audit as a live checklist and refuse to mark the goal complete until every audit item and completion criterion is filled with concrete evidence or marked N/A with a reason.
