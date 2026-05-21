---
name: goal-ready-spec
description: Use only when the user explicitly invokes $goal-ready-spec or specifically asks to make a spec goal-ready. Creates guarded, auditable implementation specs for Codex or another agent.
---

# Goal-Ready Spec

## Overview

Create file-backed specs that an execution agent can follow without replacing explicit requirements with merely equivalent behavior. The goal is to collaborate with the user until the spec is clear, executable, and faithful to their intent, not to finish a spec in one pass. The output must make architectural ownership, state location, scope boundaries, verification commands, and completion evidence auditable before any goal is marked complete. Plan files should be compact-resilient and context-efficient: keep `spec.md` as a small router and controlling contract, keep hot-path files focused on current state, and route detailed phase, context, audit, and evidence material into smaller files that are opened only when needed. The final response should hand the user a concise prompt that asks Codex to set its own goal from the generated spec file, using the goal-design advice from OpenAI's "Using Goals in Codex" cookbook: define the outcome, evidence surface, constraints, boundaries, iteration policy, and terminal condition.

Do not use this skill for ordinary planning, brainstorming, outlining, refining, reviewing, or implementation-plan requests unless the user explicitly invokes `$goal-ready-spec` or specifically asks to make the spec goal-ready.

## Workflow

1. Read the user request, prior plan text, and relevant project instructions before writing the spec.
2. If working inside a repository, inspect existing docs/config/code that affect the requested work. Do not write from general assumptions when project files can answer the question.
3. Capture code-location anchors for binding requirements when the current codebase can identify likely owners, APIs, modules, tests, generated files, or verification surfaces. Keep anchors compact: prefer file, module, type, function, command, or test names that help execution and audit; do not turn the spec into an exhaustive call-site map.
4. Classify requirements before writing the spec. Treat requirements as binding only when they are current, unrejected, and material to the requested goal. Do not promote examples, discarded alternatives, speculative ideas, or stale plan text into Hard Requirements.
5. Treat architecture, ownership, lifecycle, state placement, API shape, generated-file expectations, and test requirements as first-class requirements when they are binding.
6. Identify any binding requirement where the user asked for a specific component, module, actor, manager, owner, or data model. Mark these as "no equivalent substitution" requirements.
7. Front-load clarification during spec creation. Ask focused questions and incorporate the answers before finalizing whenever missing information would prevent a clear, executable, faithful spec. Only leave non-blocking uncertainty in Risks and Open Questions.
8. Assign compact IDs to requirements, ownership invariants, verification gates, and reusable evidence. Prefer IDs like `REQ-01`, `OWN-01`, `VER-01`, and `EV-cloud-mqtt-smoke` over repeating long requirement or command prose.
9. Preserve the original spec or plan text as `_plans/<short-slug>/original-spec.md` before rewriting it into the goal-ready contract. If the source is already a durable repo file, `original-spec.md` may point to that path plus commit or snapshot context and a short binding-requirements summary instead of copying hundreds of lines. If the source was only provided in chat, copy the relevant source text into that file so later agents can audit the transformed spec against the original.
10. Write the goal-ready spec to `_plans/<short-slug>/spec.md`, the progress checkpoint to `_plans/<short-slug>/progress.md`, and the audit checklist to `_plans/<short-slug>/audit.md`. This skill is for the repo-local `_plans` workflow; do not produce an unfixed chat-only spec unless the user explicitly asks for that fallback.
11. The `_plans/<short-slug>/` folder is the goal's durable working area. Start with hot entry-point files `original-spec.md`, `spec.md`, `progress.md`, and `audit.md`. Make `spec.md` a router by default and put detailed phase, audit, decision, context, or evidence material in supporting files before the hot files become expensive to read. Prefer small, named files with clear purposes over copying large context into hot-path files.
12. Split independent workflows into child phase files or child goal-ready specs. The parent `spec.md` should track phase status, read routing, shared invariants, and completion gates rather than embedding every implementation detail.
13. Shape the goal handoff using the cookbook goal pattern. The handoff must define the outcome, evidence surface, constraints, boundaries, iteration policy, and terminal condition without embedding the whole spec in the goal text.
14. Before finalizing, audit the spec against the source request and confirm every binding material requirement appears in an acceptance criterion, completion criterion, or completion-audit item.
15. End the final response with a "Set Your Goal Prompt" that references the final `_plans/<short-slug>/spec.md` path and tells Codex to set its own goal from that spec.

## Required Spec Sections

Use this structure unless the user asks for a different format:

- Objective: the concrete end state in one short paragraph
- Source Context: files, docs, plans, current code-location anchors, and user statements the spec is based on
- Scope: what must change
- Non-Goals: what must not change
- Requirement Classification: binding requirements, inferred requirements, non-goals or rejected alternatives, and open questions
- Hard Requirements: binding requirements copied or faithfully paraphrased from the plan, with compact code-location anchors when known
- Architecture and Ownership Invariants: where state, lifecycle, orchestration, APIs, and boundaries must live, with owner or module anchors when known
- No Equivalent Substitutions: requirements where matching behavior is insufficient unless the named owner/design is implemented, with the relevant current owner/design anchor when known
- Execution Contract: the implementation goal must reference this spec by path or title and treat it as the controlling implementation contract
- Plan Folder Contract: define `_plans/<short-slug>/` as the durable working area for spec, progress, audit, and optional supporting files, with progressive-disclosure loading rules
- Read Map: a small table that tells the execution agent which file to read for each common need, and which files to skip
- Goal Runtime Handoff: instructions that make the spec durable across `/goal` continuations, including rereading controlling sections, maintaining a separate Completion Audit file, and not marking the goal complete until audit evidence exists
- Implementation Plan: ordered phases with files or modules likely involved
- Deviation Protocol: contract-level changes that require stopping to ask before implementation continues
- Verification Plan: exact commands, tests, generated outputs, manual checks, and expected evidence
- Progress File: required progress checkpoint path and template the execution agent should keep short and current
- Completion Audit File: required audit file path and template the execution agent must fill out with evidence, deviation review, counter-evidence scan, and any final completion-review result before committing or marking the goal complete
- Completion Criteria: process requirements before the goal is complete, including verification results, every Completion Audit file item filled with concrete evidence or marked N/A with a reason, no unresolved audit blocker that the agent can act on, and a terminal manual-intervention record if manual work is the only remaining blocker
- Acceptance Criteria: behavior plus required architecture, not behavior alone
- Risks and Open Questions: unresolved assumptions that must be answered before or during execution
- Recommended Goal Objective: concise goal wording that references the spec as the controlling contract
- Set Your Goal Prompt: user-facing prompt that asks Codex to set its own goal from the final `spec.md`

Keep these sections router-sized in `spec.md`. Move detailed implementation slices, source comparisons, codebase notes, long verification notes, and phase-specific audit details into supporting files and link them from the relevant router section.

## Goal Design Checklist

Apply this checklist when writing the Recommended Goal Objective and the final Set Your Goal Prompt. These checks are based on OpenAI's "Using Goals in Codex" cookbook guidance for high-quality goals:

- Outcome: name the concrete end state the goal should achieve
- Evidence Surface: point to the files, commands, tests, generated artifacts, or audit items that prove completion
- Constraints: preserve required architecture, ownership, APIs, scope limits, style rules, and repository instructions
- Boundaries: identify what the agent may change and what it must leave untouched
- Iteration Policy: tell the agent how to continue after partial progress, failed checks, or compaction
- Terminal Condition: tell the agent when to mark the goal complete after success or when only irreducible blockers remain, and what evidence, attempts, and requested input to record

Do not let the Set Your Goal Prompt become the full spec. It should point to `spec.md` as the controlling contract and summarize only the goal-shaping details needed to make the goal self-contained.

## Guardrails

- If the user explicitly requires component X to own state, lifecycle, cadence, cancellation, retries, persistence, reconciliation, or API shape, the spec must require X to own it.
- Do not promote implementation guesses, examples, tentative wording, or local plan mechanics into Hard Requirements or No Equivalent Substitutions. Those categories are only for explicit, current, material user requirements or requirements forced by inspected project facts.
- Passing tests or matching user-visible behavior does not satisfy an explicit architecture or ownership requirement by itself.
- The spec is the controlling implementation contract for the goal unless it conflicts with newer user instructions, repository instructions, or facts discovered during implementation.
- The implementation goal must cite the spec path or title and must not treat the spec as optional background context.
- Resolve blocking ambiguity during spec creation whenever possible. The execution agent should treat the finalized spec as the contract and continue through routine implementation choices without asking.
- During execution, the agent must stop and ask only when the next step would change ownership, boundaries, required APIs, acceptance criteria, verification scope, or contradict the spec, repo facts, or newer user instructions.
- If the agent cannot make defensible progress within the goal contract, it must first complete any other agent-actionable work that can still be done within the contract. Mark the goal complete as terminal only when every remaining unsatisfied item is an irreducible blocker. The blocker report should include the failing evidence, paths or commands tried, the specific missing decision or resource, and the next user input needed. Do not present blocked work as implemented or verified.
- Budget exhaustion is not success and is not terminal completion by itself. If the goal budget is reached before completion, mark the goal complete only when every remaining unsatisfied item is an irreducible blocker; otherwise summarize completed work, unsatisfied criteria, verification state, blockers, and the next best action without claiming completion.
- If the spec is not ready yet, ask the user focused questions and work with them until it is ready instead of finalizing a weak draft.
- The goal is not complete until the Completion Audit file is filled out with concrete evidence for every item or an N/A reason for irrelevant items.
- The spec must instruct the execution agent to update the separate Completion Audit file before calling `update_goal`.
- Deviations that affect Hard Requirements, Architecture and Ownership Invariants, No Equivalent Substitutions, lifecycle, concurrency, persistence, API shape, verification scope, or acceptance criteria are blockers unless the user explicitly approved the deviation.
- Do not amend the controlling spec during execution to make the current goal completable. If the contract needs to change and no other agent-actionable work remains, record the contract-change blocker, mark the current goal complete as terminal, and start a new goal from the updated spec.
- Residual risk that touches a Hard Requirement or No Equivalent Substitution blocks completion. Do not mark that risk as low while also marking the goal complete.
- `progress.md` must not say Done, complete, ready to commit, or only pending review while `audit.md` has unapproved deviations, unresolved residual risk, unsatisfied criteria, or missing evidence, unless the only remaining blocker is terminal manual intervention. In that case it must say the agent-actionable work is complete, name the manual intervention still required, and state that the manual work was not performed by the agent.
- Do not require subagent review, independent boundary review, or any other approval-only review gate unless the user explicitly asks for that gate. Verification and audit requirements should be satisfiable by the executing agent through commands, file inspection, documented evidence, or an N/A reason.
- If the user explicitly asks for independent or subagent completion review, define it as a final-only read-only completion gate, not a routine phase or progress-review step. It should run only after the implementing agent believes all criteria are satisfied and `audit.md` has been filled.
- A completion-review subagent, when requested and available, should compare `spec.md`, `original-spec.md`, `progress.md`, `audit.md`, supporting files referenced by uncertain audit items, and the current uncommitted diff. It must search for counter-evidence and classify Hard Requirements, Architecture and Ownership Invariants, No Equivalent Substitutions, deviations, residual risk, verification scope, and acceptance criteria.
- If the requested completion-review subagent is unavailable, define a fallback where the executing agent performs the same final read-only counter-evidence audit itself and records the fallback in `audit.md`.
- Any unresolved completion-review blocker means the implementation is not successfully complete. The agent must update `progress.md` and `audit.md`, then continue within the contract if any agent-actionable work remains. If every remaining unsatisfied item is an irreducible blocker, record it as terminal and mark the goal complete so it does not loop.
- Never write a Completion Criteria or Completion Audit item whose only possible next action is "ask the user to approve subagent review". If approval is truly required and missing, the execution agent should ask once and continue any other agent-actionable work. Mark the goal complete as terminal only when that approval is the only remaining unsatisfied item.
- If a blocker requires manual intervention and the agent detects it is repeatedly trying the same impossible or unauthorized step, treat that as a loop condition. Once all other criteria are satisfied or marked N/A with evidence, record the manual blocker in `progress.md` and `audit.md`, then mark the goal complete to stop the loop. Do not present the manual intervention as completed.
- The spec must instruct the execution agent to reread controlling spec sections and inspect current repo state on continuation or resume before deciding the next work. Do not require rereading every line on every continuation; require rereading Objective, Hard Requirements, Architecture and Ownership Invariants, Deviation Protocol, Verification Plan, Completion Criteria, the Read Map, and any routed supporting file relevant to the next task.
- The recommended goal objective and final Set Your Goal Prompt should stay short and point to the spec path or title rather than embedding the whole implementation contract.
- The final Set Your Goal Prompt must start with language like "Set your own goal to..." so the next Codex instance creates a durable goal instead of treating the text as ordinary chat instructions.
- Plan-folder files must support progressive disclosure: put the smallest durable router and controlling contract in `spec.md`, current state in `progress.md`, parent completion status in `audit.md`, and bulky analysis or context in smaller supporting files linked from the relevant router section.
- Do not instruct the execution agent to load every file in `_plans/<short-slug>/` on every continuation, and never tell it to read all supporting files "for context". It should read the router, `progress.md`, current repo state, and only the file or two selected by the Read Map for the next action.
- Classify plan files and sections by read temperature: Hot files are read every continuation, Warm files are read only while working that phase, and Cold files are read only for final audit, ambiguity, or debugging a blocker.
- A long `spec.md` is a smell. Make `spec.md` a router by default with a Read Map, requirement IDs, shared invariants, acceptance/completion gates, and links to phase/context/audit files. Move phase detail, source comparisons, and long code-location maps out of `spec.md`.
- Use four required hot entry-point files: `original-spec.md`, `spec.md`, `progress.md`, and `audit.md`. Use `phases/`, `audits/`, `decisions/`, `context/`, and `evidence/` as the normal homes for detailed phase work, support context, and bulky evidence.
- Required files must always have these roles:
  - `original-spec.md`: original user-provided spec, plan, or source text preserved before goal-ready rewriting
  - `spec.md`: controlling contract, router, scope, requirement IDs, architecture, verification gates, read map, completion criteria, and recommended goal objective
  - `progress.md`: short resume checkpoint, active focus, completed phase summaries, next action, blockers, and read-next guidance
  - `audit.md`: compact parent completion dashboard, latest verification snapshot, deviations, residual risk, and final proof checklist
- Use one trace surface as the source of truth, usually a requirement or workflow matrix. Do not duplicate the same proof across Requirement Trace, Ownership Trace, Completion Criteria, Acceptance Criteria, Verification Results, and phase notes; those sections should reference IDs, summarize exceptions, or point to the source-of-truth row.
- `progress.md` is overwrite-only compact resume state, not an append-only diary. Update only the fields that changed during routine work. Rewrite or compress it only when it exceeds the target size, starts repeating old phase history, reaches a phase handoff, prepares for final audit, or the agent is about to leave the goal in a state that must survive compaction.
- Keep `progress.md` short enough to read every continuation. Target under 60 lines. If it grows into a diary, compress it back into current state, milestone summaries, next action, blockers, and read-next; do not spend tokens re-compressing an already compact file.
- `audit.md` is a compact dashboard, not a running task log. Target under 120-150 lines. It should store current completion status, latest relevant verification snapshot, deviations, final-only checks, and unresolved blockers.
- Verification evidence should be latest-snapshot only. Record the latest relevant pass or fail for each required command, probe, or evidence ID; keep earlier failures only when they explain a current blocker, deviation, or important fix.
- Successful repeated checks should collapse to one latest-result row. If a probe emits JSON, smoke summary keys, or generated artifacts, reference the key, artifact, or evidence ID instead of translating the whole result into prose.
- Use risk-triggered verification during implementation: after each slice, rerun the smallest relevant focused check. Reserve full builds, smoke suites, release readiness, original-spec coverage, counter-evidence scans, and completion review for milestones or the final completion pass unless a failure indicates broader risk.
- When a phase is complete or a hot file exceeds its target, compress the completed phase to one row or one short milestone in the parent hot files. Move any detailed phase notes or verification history to `evidence/` or the phase audit file and mark that file Cold unless debugging or final-auditing that phase.
- If `spec.md` grows beyond a compact router, roughly 100-200 lines, split phase/detail content into supporting files and keep `spec.md` as the authoritative router and summary.
- Do not copy large command outputs, generated files, logs, or broad code excerpts into plan files. Summarize the relevant evidence and reference file paths, commands, snapshot names, artifacts, or key lines.
- When leaving a phase, write a short phase handoff summary in `progress.md`. Compress `progress.md` or `audit.md` only if the handoff would leave them above target, duplicate old evidence, or make later continuations reread old phase detail. Compression should be a targeted edit to the bloated section, not a reread-and-rewrite of every plan file.
- When considering completion, load the parent `audit.md`, Completion Criteria, Acceptance Criteria, Verification Plan, and only the phase audit or supporting files referenced by missing or uncertain audit items before calling `update_goal`.
- The completion audit must include a "Deviations from plan" section. If there are no deviations, it must explicitly say so.
- The completion audit must include a counter-evidence scan that looks for files, commands, or diff hunks contradicting claimed completion. This scan belongs on the final completion pass, not every routine continuation.
- "Supporting Files Consulted" belongs to final audit only. During normal execution, use `progress.md` Read next and the Read Map instead of listing every source or support file touched.
- Do not phrase critical checks as vague intentions like "verify it works". Name the command, file, assertion, generated artifact, or manual observation that proves the requirement.
- If a requirement cannot yet be verified and is not blocking, mark it as a risk or open question instead of burying it in the implementation plan.
- If the repository has or can cheaply add a plan-file validator, use it to warn on hot files that exceed target size, append-only progress histories, repeated command logs in `audit.md`, missing `Read when` or `Do not read when` headers, or support files that are not reachable from the Read Map.

## Plan Folder Contract Template

Every generated spec must include a section like this, adapted to the concrete plan folder:

```markdown
## Plan Folder Contract

This plan lives in `_plans/<short-slug>/`. Use this folder as the durable working area for the goal:

- `spec.md`: hot router, controlling implementation contract, requirement IDs, shared invariants, read map, and completion gates
- `original-spec.md`: original source spec or plan text preserved before goal-ready rewriting
- `progress.md`: hot compact current state, active phase, next action, blockers, last milestone, and one or two files to read next
- `audit.md`: hot compact parent completion dashboard, latest verification snapshot, deviations, final-only checks, and unresolved blockers
- `phases/`: warm phase contracts or child-goal routers for independent work streams
- `audits/`: warm or cold phase-specific audit dashboards so parent `audit.md` stays compact
- `context/`: cold source comparisons, codebase notes, API notes, and external research summaries
- `evidence/`: cold detailed verification notes, archived phase history, or generated summary references

Use progressive disclosure for context management. On continuation, load `spec.md` and `progress.md` first, then follow the Read Map. Do not read all supporting files for context. Open `audit.md` only when updating evidence, checking blockers, or considering completion. Open phase, audit, context, or evidence files only when the Read Map, current phase, a blocker, or final completion pass points to them. When adding files, reference them from `spec.md`, `progress.md`, or `audit.md` with a one-line `Read when` rule so the next continuation can recover relevant state after compaction without loading everything.

## Read Map

| Need | Read | Skip |
| --- | --- | --- |
| Resume work | `spec.md`, `progress.md`, current repo state | `audit.md` unless updating evidence or checking blockers |
| Implement `<phase>` | `phases/<phase>.md`, phase-relevant source files | unrelated phase, context, audit, and evidence files |
| Update evidence for `<phase>` | `audit.md` and `audits/<phase>.md` if present | unrelated phase audits and old evidence archives |
| Debug a failing check | the failing command output, owning phase file, referenced evidence file | completed phase archives not related to the failure |
| Final completion pass | `audit.md`, Completion Criteria, Acceptance Criteria, Verification Plan, referenced phase audits, `original-spec.md` | detailed evidence logs unless a missing or uncertain audit item points to them |
```

## Supporting File Naming

Create supporting files for detailed content so hot files stay small. Use these paths:

- `phases/phase-<n>-<short-name>.md` for detailed phase contracts that are only needed while executing or auditing that phase
- `audits/<short-name>.md` for phase-specific completion dashboards so the parent audit does not copy phase detail
- `decisions/<short-name>.md` for durable decisions that affect later implementation
- `context/<short-name>.md` for bulky analysis, codebase notes, API notes, or external research summaries
- `evidence/<short-name>.md` for longer verification notes that would bloat `audit.md`
- `evidence/archive-<short-name>.md` for compressed phase history that should not be read during normal continuation

Each supporting file must start with:

```markdown
Read when: <specific condition for loading this file>
Do not read when: <specific condition for skipping this file>
Temperature: Hot|Warm|Cold
```

## Goal Runtime Handoff Template

Every generated spec must include a section like this, adapted to the concrete spec:

```markdown
## Goal Runtime Handoff

This spec is the controlling router and contract for the implementation goal. At the start of execution, read this router spec, inspect the current repo state, and read `<progress path>` before choosing the next action. On continuation or resume, reread only the hot sections of this spec and follow the Read Map; do not read all supporting files for context. Open `<audit path>` when updating evidence, checking blockers, or considering completion. Open phase, context, or phase-audit files only when the Read Map, current task, blocker, or final completion pass points to them. Keep `<progress path>` short by updating changed fields during routine work and compressing only when it exceeds target size, starts acting like a diary, reaches a phase handoff, prepares for final audit, or must survive likely compaction. Keep `<audit path>` as a compact completion dashboard and latest verification snapshot, not a running log. Keep detailed phase, audit, decision, context, and evidence material in supporting files under `_plans/<short-slug>/`, and link each file from the Read Map with when to read and skip it. If this spec or the user requires independent completion review, run that review only on the final completion pass after `<audit path>` is filled; if the review cannot run, perform and record the specified fallback audit. Do not call `update_goal` or otherwise mark the goal complete until every Completion Criteria item and every Completion Audit file item is satisfied with evidence or marked N/A with a reason, with no unresolved deviation, residual-risk, or completion-review blocker, except for a recorded terminal manual-intervention blocker after all agent-actionable work is complete.
```

## Recommended Goal Objective Template

Every generated spec must include concise goal wording the user can use when asking Codex to create a goal:

```markdown
## Recommended Goal Objective

Implement `<spec path or title>` as the controlling router and contract. Achieve `<outcome>` with completion proven by `<evidence surface>`. Preserve `<constraints>` and stay within `<boundaries>`. On each continuation, read the router spec's hot sections, `<progress path>`, current repo state, and only the files selected by the Read Map for the next action. Do not read all supporting files for context. Iterate from failing or partial verification by making the smallest contract-preserving change and rerunning the relevant focused checks; reserve broad verification and final-only audits for milestones or completion. Use `<audit path>` as the parent completion dashboard and latest verification snapshot, with detailed evidence in phase audits or `evidence/` only when needed. If independent completion review is required, run it only on the final completion pass after the audit is filled, or perform the specified fallback audit if unavailable. If progress requires changing the contract, missing user input, unavailable resources, an unresolved final-review blocker, or manual intervention, continue any other agent-actionable work that can be done within the contract. Only when every remaining unsatisfied item is an irreducible blocker, record the terminal blocker in `<progress path>` and `<audit path>`, then call `update_goal` so the goal does not loop. Report attempts, evidence, and the needed decision. Do not present blocked or manual work as completed. Otherwise, do not call `update_goal` until every Completion Criteria and Completion Audit file item is satisfied or marked N/A with a reason and no unresolved audit blocker remains.
```

## Set Your Goal Prompt Template

End the final response with a prompt like this, adapted to the concrete spec path. This is the text the user can send to Codex to start the execution goal:

```markdown
Set your own goal to implement `_plans/<short-slug>/spec.md` as the controlling router and contract. Use the spec's Objective, Hard Requirements, Architecture and Ownership Invariants, Verification Plan, Read Map, Completion Criteria, and Completion Audit File as the source of truth. Keep `_plans/<short-slug>/progress.md` current for continuation state and `_plans/<short-slug>/audit.md` focused on parent completion evidence. On continuation, read the router, progress, current repo state, and only the files selected by the Read Map; do not read all supporting files for context. Iterate by making the smallest contract-preserving change after each failed or partial check, then rerun the relevant focused verification. Use any required completion-review subagent only once, on the final completion pass after the audit is filled; do not spend subagent tokens earlier, and use the specified fallback audit if a subagent is unavailable. If progress is blocked by missing input, unavailable resources, a necessary contract change, an unresolved final-review blocker, or manual intervention, continue any other agent-actionable work that can be done within the contract. Only when every remaining unsatisfied item is an irreducible blocker, record the terminal blocker in `_plans/<short-slug>/progress.md` and `_plans/<short-slug>/audit.md`, then mark the goal complete so it does not loop. Report the blocker, evidence, attempts, and needed decision. Do not present blocked or manual work as completed. Otherwise, do not mark the goal complete until the audit proves every completion criterion is satisfied or marked N/A with a reason and no unresolved audit blocker remains.
```

## Progress File Template

Every generated plan must include a separate progress checkpoint file beside the spec as `_plans/<short-slug>/progress.md`. Keep it short and current. During routine work, update only changed fields. Rewrite or compress only when the file exceeds its target size, starts acting like a diary, reaches a phase handoff, prepares for final audit, or must survive likely compaction.

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

Every generated spec must define a separate Completion Audit file path beside the spec as `_plans/<short-slug>/audit.md`. Executing the goal includes completing the audit file with evidence; the goal is incomplete until the audit is filled out or each irrelevant item is marked N/A with a reason. Keep this file as a compact parent dashboard. Phase-specific details belong in `audits/<phase>.md` or `evidence/<name>.md`, and the parent audit links to those files only when needed.

```markdown
# Completion Audit

Status: in progress
Spec: `_plans/<short-slug>/spec.md`

## Requirement Status

Use this as the primary trace surface. Every row must include concrete evidence refs or an N/A reason. Evidence refs should be IDs, file/function anchors, test names, probe names, smoke-summary keys, generated artifacts, or phase audit paths, not long command transcripts.

| ID | Status | Evidence refs | Remaining blocker |
| --- | --- | --- | --- |
| REQ-01 | pending | | |

## Verification Snapshot

Record the latest relevant result for each required command, probe, or evidence ID. Do not keep repeated successful runs or stale failures unless they explain a current blocker, deviation, or important fix.

| Check or evidence ID | Latest result | Evidence ref |
| --- | --- | --- |
| VER-01 | pending | |

## Phase Audit Index

Use when phase audits exist. Parent audit rows should link to phase audits instead of copying phase detail here.

| Phase | Status | Audit file | Blocker |
| --- | --- | --- | --- |

## Deviations From Plan

- none, or list each deviation with explicit user approval

## Generated Artifacts

- none, or list generated outputs that are part of completion evidence

## Final Pass Only

Fill only when the implementation is believed complete or every remaining item is an irreducible blocker:

- Original spec coverage:
- Counter-evidence scan:
- Completion criteria:
- Final completion review:
- Terminal manual intervention:
- Supporting files consulted for final audit:
- Residual risk:
```

## Goal Execution Handoff

When the user asks Codex to execute one of these specs as a goal, the implementation goal should cite the spec path or title. The execution agent should first read the router spec, project instructions, `progress.md`, and current repo state, then follow the Read Map instead of preloading every referenced file. It should treat the spec as the controlling implementation contract unless it conflicts with newer user instructions, repository instructions, or implementation facts. It should maintain `progress.md` as compact resume state and `audit.md` as a compact parent completion dashboard.

The execution agent should refuse to mark the goal complete until every audit item and completion criterion is filled with concrete evidence or marked N/A with a reason, and no unapproved deviation, residual-risk blocker, or final-review blocker remains, except for a terminal manual-intervention blocker recorded in the audit after all agent-actionable work is complete. Deviations affecting hard requirements, architecture or ownership invariants, no-equivalent-substitution requirements, lifecycle, concurrency, persistence, API shape, verification scope, or acceptance criteria are blockers unless explicitly approved by the user. Do not amend the controlling spec during execution to make the current goal completable; if the contract needs to change and no other agent-actionable work remains, record the contract-change blocker, mark the current goal complete as terminal, and start a new goal from the updated spec.

If the goal continues across turns or resumes after compaction, the agent should use progressive disclosure: read the router spec's hot sections, `progress.md`, current repo state, and only the supporting files selected by the Read Map for the next action or completion decision. When finishing a phase, it should summarize the phase outcome in `progress.md` before moving on. It should compress hot files only when they exceed target size, duplicate old evidence, reach a phase handoff, prepare for final audit, or must survive likely compaction. After failed or partial verification, it should inspect the evidence, make the smallest contract-preserving next change, rerun the relevant focused verification, and update progress. If budget is exhausted or the next step requires missing input, unavailable resources, manual intervention, or a contract change, it should continue any other agent-actionable work that can still be done within the contract. Only when every remaining unsatisfied item is an irreducible blocker should it record the terminal blocker, report completed work, unsatisfied criteria, verification state, attempts, and the needed decision, then mark the goal complete so it does not loop.

On the final completion pass only, before calling `update_goal`, the agent should load `audit.md`, the relevant original-spec source or pointer, Completion Criteria, Acceptance Criteria, Verification Plan, and only the phase audits or supporting files referenced by missing or uncertain audit items. It should confirm the final implementation and completed spec covered every binding material requirement from the original spec source, then run the required counter-evidence scan against the current diff and relevant files. If the user or spec required independent completion review, launch the subagent review only at this final point after `audit.md` is filled; if the review is unavailable, perform the specified read-only fallback audit and record it in `audit.md`. It should not repeat the original-spec coverage pass, counter-evidence scan, or completion-review subagent during routine continuation, progress updates, or phase handoffs.

The agent should keep detailed phase, decision, context, and evidence material under the same `_plans/<short-slug>/` folder instead of expanding hot files, and should reference those files from the spec, progress file, or audit with when to read them so future continuations can recover state without loading unnecessary context.
