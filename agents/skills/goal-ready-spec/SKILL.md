---
name: goal-ready-spec
description: Use only when the user explicitly invokes $goal-ready-spec or specifically asks to make a spec goal-ready. Creates guarded, auditable implementation specs for Codex or another agent.
---

# Goal-Ready Spec

## Overview

Create file-backed specs that an execution agent can follow without replacing explicit requirements with merely equivalent behavior. The goal is to collaborate with the user until the spec is clear, executable, and faithful to their intent, not to finish a spec in one pass. The output must make architectural ownership, state location, scope boundaries, verification commands, and completion evidence auditable before any goal is marked complete. Plan files should be compact-resilient and context-efficient: use progressive disclosure so the agent can load a small controlling contract first, then open detailed context files only when needed.

Do not use this skill for ordinary planning, brainstorming, outlining, refining, reviewing, or implementation-plan requests unless the user explicitly invokes `$goal-ready-spec` or specifically asks to make the spec goal-ready.

## Workflow

1. Read the user request, prior plan text, and relevant project instructions before writing the spec.
2. If working inside a repository, inspect existing docs/config/code that affect the requested work. Do not write from general assumptions when project files can answer the question.
3. Classify requirements before writing the spec. Treat requirements as binding only when they are current, unrejected, and material to the requested goal. Do not promote examples, discarded alternatives, speculative ideas, or stale plan text into Hard Requirements.
4. Treat architecture, ownership, lifecycle, state placement, API shape, generated-file expectations, and test requirements as first-class requirements when they are binding.
5. Identify any binding requirement where the user asked for a specific component, module, actor, manager, owner, or data model. Mark these as "no equivalent substitution" requirements.
6. Front-load clarification during spec creation. Ask focused questions and incorporate the answers before finalizing whenever missing information would prevent a clear, executable, faithful spec. Only leave non-blocking uncertainty in Risks and Open Questions.
7. Preserve the original spec or plan text as `_plans/<short-slug>/original-spec.md` before rewriting it into the goal-ready contract. If the source was only provided in chat, copy the relevant source text into that file so later agents can audit the transformed spec against the original.
8. Write the goal-ready spec to `_plans/<short-slug>/spec.md`, the progress checkpoint to `_plans/<short-slug>/progress.md`, and the audit checklist to `_plans/<short-slug>/audit.md`. This skill is for the repo-local `_plans` workflow; do not produce an unfixed chat-only spec unless the user explicitly asks for that fallback.
9. The `_plans/<short-slug>/` folder is the goal's durable working area. Start with `original-spec.md`, `spec.md`, `progress.md`, and `audit.md`. Add supporting phase, decision, context, or evidence files only when the work is large enough that splitting them improves context management. Prefer small, named files with clear purposes over copying large context into the main spec.
10. Before finalizing, audit the spec against the source request and confirm every binding material requirement appears in an acceptance criterion, completion criterion, or completion-audit item.

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
- Plan Folder Contract: define `_plans/<short-slug>/` as the durable working area for spec, progress, audit, and optional supporting files, with progressive-disclosure loading rules
- Goal Runtime Handoff: instructions that make the spec durable across `/goal` continuations, including rereading controlling sections, maintaining a separate Completion Audit file, and not marking the goal complete until audit evidence exists
- Implementation Plan: ordered phases with files or modules likely involved
- Deviation Protocol: contract-level changes that require stopping to ask before implementation continues
- Verification Plan: exact commands, tests, generated outputs, manual checks, and expected evidence
- Progress File: required progress checkpoint path and template the execution agent should keep short and current
- Completion Audit File: required audit file path and template the execution agent must fill out with evidence before committing or marking the goal complete
- Completion Criteria: process requirements before the goal is complete, including verification results and every Completion Audit file item filled with concrete evidence or marked N/A with a reason
- Acceptance Criteria: behavior plus required architecture, not behavior alone
- Risks and Open Questions: unresolved assumptions that must be answered before or during execution
- Recommended Goal Objective: concise `/goal` wording that references the spec as the controlling contract

## Guardrails

- If the user explicitly requires component X to own state, lifecycle, cadence, cancellation, retries, persistence, reconciliation, or API shape, the spec must require X to own it.
- Do not promote implementation guesses, examples, tentative wording, or local plan mechanics into Hard Requirements or No Equivalent Substitutions. Those categories are only for explicit, current, material user requirements or requirements forced by inspected project facts.
- Passing tests or matching user-visible behavior does not satisfy an explicit architecture or ownership requirement by itself.
- The spec is the controlling implementation contract for the goal unless it conflicts with newer user instructions, repository instructions, or facts discovered during implementation.
- The implementation goal must cite the spec path or title and must not treat the spec as optional background context.
- Resolve blocking ambiguity during spec creation whenever possible. The execution agent should treat the finalized spec as the contract and continue through routine implementation choices without asking.
- During execution, the agent must stop and ask only when the next step would change ownership, boundaries, required APIs, acceptance criteria, verification scope, or contradict the spec, repo facts, or newer user instructions.
- If the spec is not ready yet, ask the user focused questions and work with them until it is ready instead of finalizing a weak draft.
- The goal is not complete until the Completion Audit file is filled out with concrete evidence for every item or an N/A reason for irrelevant items.
- The spec must instruct the execution agent to update the separate Completion Audit file before calling `update_goal`.
- The spec must instruct the execution agent to reread controlling spec sections and inspect current repo state on continuation or resume before deciding the next work. For large specs, do not require rereading every line on every continuation; require rereading Objective, Hard Requirements, Architecture and Ownership Invariants, Deviation Protocol, Verification Plan, Completion Criteria, and any detailed sections relevant to the next task.
- The recommended `/goal` objective should stay short and point to the spec path or title rather than embedding the whole implementation contract.
- Plan-folder files must support progressive disclosure: put the smallest durable contract in `spec.md`, progress and evidence in `audit.md`, and bulky analysis or context in separate files that are linked from the relevant section.
- Do not instruct the execution agent to load every file in `_plans/<short-slug>/` on every continuation. It should load the index/controlling sections first, then only the referenced detail files needed for the next action or completion decision.
- Default to exactly four files: `original-spec.md`, `spec.md`, `progress.md`, and `audit.md`. Add `phases/`, `decisions/`, `context/`, or other supporting files only when the spec is large, naturally phase-oriented, or would otherwise overload context.
- Required files must always have these roles:
  - `original-spec.md`: original user-provided spec, plan, or source text preserved before goal-ready rewriting
  - `spec.md`: controlling contract, index, scope, requirements, architecture, verification, completion criteria, and recommended goal objective
  - `progress.md`: short resume checkpoint, active focus, completed phase summaries, next action, blockers, and read-next guidance
  - `audit.md`: completion evidence, verification results, deviations, residual risk, and final proof checklist
- `progress.md` is for compact resume state and should be rewritten as the active work changes. `audit.md` is for completion evidence and should not become a running task log.
- Keep `progress.md` short enough to read every continuation. If it grows into a diary, compress it back into current state, completed phase summaries, next action, and blockers.
- If `spec.md` grows beyond a compact controlling contract, roughly 300-500 lines, split phase/detail content into supporting files and keep `spec.md` as the index and authoritative summary.
- Do not copy large command outputs, generated files, logs, or broad code excerpts into plan files. Summarize the relevant evidence and reference file paths, commands, snapshot names, artifacts, or key lines.
- When leaving a phase, write a short phase handoff summary in `progress.md` so later continuations do not need to reread old phase detail files unless they are debugging or auditing that phase.
- When considering completion, load `audit.md`, Completion Criteria, Acceptance Criteria, Verification Plan, and any supporting files referenced by missing or uncertain audit items before calling `update_goal`.
- The completion audit must include a "Deviations from plan" section. If there are no deviations, it must explicitly say so.
- Do not phrase critical checks as vague intentions like "verify it works". Name the command, file, assertion, generated artifact, or manual observation that proves the requirement.
- If a requirement cannot yet be verified and is not blocking, mark it as a risk or open question instead of burying it in the implementation plan.

## Plan Folder Contract Template

Every generated spec must include a section like this, adapted to the concrete plan folder:

```markdown
## Plan Folder Contract

This plan lives in `_plans/<short-slug>/`. Use this folder as the durable working area for the goal:

- `spec.md`: controlling implementation contract and index of any supporting files
- `original-spec.md`: original source spec or plan text preserved before goal-ready rewriting
- `progress.md`: compact current state, active phase, next action, blockers, and what to read next
- `audit.md`: completion evidence and verification state only
- additional phase, decision, context, or evidence files may be created only when useful for context management

Use progressive disclosure for context management. On continuation, load `spec.md` controlling sections and `progress.md` first; open `audit.md` when updating evidence or considering completion. Open supporting files only when they are referenced by the current task, needed to resolve uncertainty, or needed for completion verification. When adding files, reference them from `spec.md`, `progress.md`, or `audit.md` with a one-line description of when to read them so the next continuation can recover relevant state after compaction without loading everything.
```

## Supporting File Naming

Only create supporting files when they improve context management. Use these paths when needed:

- `phases/phase-<n>-<short-name>.md` for detailed phase contracts that are only needed while executing or auditing that phase
- `decisions/<short-name>.md` for durable decisions that affect later implementation
- `context/<short-name>.md` for bulky analysis, codebase notes, API notes, or external research summaries
- `evidence/<short-name>.md` for longer verification notes that would bloat `audit.md`

Each supporting file must start with:

```markdown
Read when: <specific condition for loading this file>
```

## Goal Runtime Handoff Template

Every generated spec must include a section like this, adapted to the concrete spec:

```markdown
## Goal Runtime Handoff

This spec is the controlling contract for the implementation goal. At the start of execution, read the full spec. On continuation or resume, use progressive disclosure: reread the controlling sections of this spec, inspect the current repo state, and read `<progress path>` before choosing the next action. Open `<audit path>` when updating evidence or considering completion. For large specs, reread only the detailed sections and supporting files relevant to the next task instead of rereading every line. Keep `<progress path>` short and current by rewriting it as the active work changes. Keep `<audit path>` focused on concrete completion evidence, not a running log. Use `_plans/<short-slug>/` for additional phase, decision, context, or evidence files only when that helps the goal survive compaction, and link each file from `spec.md`, `progress.md`, or `audit.md` with when to read it. Do not call `update_goal` or otherwise mark the goal complete until every Completion Criteria item and every Completion Audit file item is satisfied with evidence or marked N/A with a reason.
```

## Recommended Goal Objective Template

Every generated spec must include concise goal wording the user can paste into `/goal` or ask Codex to create:

```markdown
## Recommended Goal Objective

Read and implement `<spec path or title>` as the controlling contract. On each continuation, use progressive disclosure: read the spec's controlling sections, `<progress path>`, current repo state, and only the relevant supporting files for the next action. Use `<audit path>` for evidence and completion checks. Do not call `update_goal` until every Completion Criteria and Completion Audit file item is satisfied or marked N/A with a reason.
```

## Progress File Template

Every generated plan must include a separate progress checkpoint file beside the spec as `_plans/<short-slug>/progress.md`. Keep it short and current; rewrite it as work progresses rather than appending a long diary.

```markdown
# Progress

Current state:
- ...

Active phase or focus:
- ...

Completed phase summaries:
- ...

Next action:
- ...

Blockers or questions:
- ...

Read next:
- `spec.md` controlling sections
- ...

Do not read unless needed:
- ...
```

## Completion Audit File Template

Every generated spec must define a separate Completion Audit file path beside the spec as `_plans/<short-slug>/audit.md`. Executing the goal includes completing the audit file with evidence; the goal is incomplete until the audit is filled out or each irrelevant item is marked N/A with a reason.

```markdown
# Completion Audit

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
- Supporting Files Consulted: list any `_plans/<short-slug>/` supporting files read for this audit, or state "none"
- Residual Risk: list any remaining uncertainty, skipped test, or environment limitation
```

## Goal Execution Handoff

When the user asks Codex to execute one of these specs as a goal, the implementation goal should cite the spec path or title. The execution agent should first read the full spec, project instructions, and referenced files, then treat the spec as the controlling implementation contract unless it conflicts with newer user instructions, repository instructions, or implementation facts. It should maintain the separate progress file as compact resume state and the separate Completion Audit file as evidence. It should refuse to mark the goal complete until every audit item and completion criterion is filled with concrete evidence or marked N/A with a reason. If the goal continues across turns or resumes after compaction, the agent should use progressive disclosure: read the spec's controlling sections, progress file, current repo state, and only the detailed sections or supporting files needed for the next action or completion decision. When finishing a phase, it should summarize the phase outcome in `progress.md` before moving on. When considering completion, it should load `audit.md`, Completion Criteria, Acceptance Criteria, Verification Plan, and any supporting files referenced by missing or uncertain audit items before calling `update_goal`. The agent may create additional phase, decision, context, or evidence files under the same `_plans/<short-slug>/` folder only when useful for context management, and should reference those files from the spec, progress file, or audit with when to read them so future continuations can recover state without loading unnecessary context.
