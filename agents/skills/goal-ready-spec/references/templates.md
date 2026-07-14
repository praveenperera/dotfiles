# Goal-Ready Templates

Adapt these templates to the task. Remove empty optional sections instead of leaving boilerplate.

## Contents

- `spec.md`
- `progress.md`
- `decisions.md`
- `audit.md`
- Set Your Goal prompt

## `spec.md`

```markdown
# <Title>

## Objective
<Concrete end state>

## Source Context
- `original-spec.md`
- <relevant source and code anchors>

## Scope
- ...

## Non-Goals
- ...

## Requirement Classification
- Binding: ...
- Inferred: ...
- Rejected or non-goal: ...
- Unresolved: ...

## Hard Requirements
- REQ-01: ...

## Architecture and Ownership Invariants
- OWN-01: ...

## No Equivalent Substitutions
- NES-01: ...

## Plan Folder and Read Map
| Need | Read | Skip |
| --- | --- | --- |
| Resume work | `spec.md`, `progress.md`, `decisions.md`, current repo state | unrelated support files |
| Implement <phase> | <phase file and source anchors> | unrelated phases and audits |
| Choose among alternatives | `decisions.md`, relevant phase or context | unrelated evidence |
| Investigate a failure | failing evidence and owning phase | completed archives |
| Evaluate completion | `audit.md`, criteria, referenced evidence, `original-spec.md` | unrelated context |

## Implementation Plan
1. ...

## Deviation Protocol
Stop and request approval before changing a hard requirement, ownership invariant, no-equivalent-substitution requirement, boundary, acceptance criterion, or verification scope.

## Verification Plan
| ID | Command or inspection | Expected evidence | Requirements |
| --- | --- | --- | --- |
| VER-01 | ... | ... | REQ-01 |

## Completion Criteria
- ...

## Acceptance Criteria
- ...

## Goal Runtime Handoff
Treat this spec as the controlling router and contract. Keep `progress.md` current, `decisions.md` updated for material non-binding choices, and `audit.md` evidence-backed. Follow the Read Map on continuation. Call `update_goal` with `complete` only after the objective and all required evidence are achieved. If irreducibly blocked, use a blocked transition only when active runtime policy permits it; otherwise report the blocker without prescribing a status change.

## Risks and Open Questions
- ...

## Recommended Goal Objective
Implement `_plans/<short-slug>/spec.md` as the controlling router and contract. Achieve <outcome>, proven by <evidence>, while preserving <constraints and boundaries>. Follow the Read Map, iterate from current evidence, and keep progress, decisions, and audit state current.
```

Add a `Runtime Policy Deviations` section only when the task requires behavior beyond current user, repository, or host policy.

## `progress.md`

```markdown
# Progress

Current state:
- ...

Active focus:
- ...

Completed summaries:
- ...

Next action:
- ...

Blockers or questions:
- ...

Read next:
- `spec.md` controlling sections
- `decisions.md` when choosing among alternatives
- ...

Skip unless needed:
- ...
```

## `decisions.md`

```markdown
# Decisions

Track material choices that are not fully covered as binding requirements in `spec.md`. Newest first. Promote into `spec.md` if a decision becomes binding.

## DEC-02: <short title>
- Date or phase: ...
- Decision: ...
- Why: ...
- Alternatives considered: ...
- Spec coverage: partial | none | promoted to REQ-XX / OWN-XX
- Status: active | reversed | promoted

## DEC-01: <short title>
- Date or phase: ...
- Decision: ...
- Why: ...
- Alternatives considered: ...
- Spec coverage: partial | none | promoted to REQ-XX / OWN-XX
- Status: active | reversed | promoted
```

Seed at least one entry when creating the plan if any interpretation, approach, or deferred scope was chosen during planning. Use `none yet` only when every material choice is already a binding requirement in `spec.md`.

## `audit.md`

```markdown
# Completion Audit

Status: in progress
Spec: `_plans/<short-slug>/spec.md`

## Requirement Status

| ID | Status | Evidence refs | Remaining blocker |
| --- | --- | --- | --- |
| REQ-01 | pending | | |

## Verification Snapshot

| Check | Latest result | Evidence ref |
| --- | --- | --- |
| VER-01 | pending | |

## Deviations
- none

## Generated Artifacts
- none

## Final Pass Only
- Original spec coverage: pending
- Counter-evidence scan: pending
- Completion criteria: pending
- Required review result: N/A unless task-specific
- Supporting files consulted: pending
- Residual risk: pending
```

## Set Your Goal prompt

```text
Set your own goal to implement `_plans/<short-slug>/spec.md` as the controlling router and contract. Achieve <outcome>, verified by <evidence surface>, while preserving <constraints> and staying within <boundaries>. Keep `_plans/<short-slug>/progress.md` current, `_plans/<short-slug>/decisions.md` updated for material choices not fully covered in the binding contract, and `_plans/<short-slug>/audit.md` evidence-backed. On continuation, read the router, progress, decisions, current repository state, and only the files selected by the Read Map. After failed or partial verification, inspect the evidence, make the smallest contract-preserving change, rerun the focused checks, and continue. Call `update_goal` with `complete` only after the objective and every required audit item are satisfied. If all remaining work is irreducibly blocked, report attempts, evidence, unmet criteria, the blocker, and the input needed; use a blocked transition only when active runtime policy permits it.
```
