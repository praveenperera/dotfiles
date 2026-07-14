# Goal-Ready Contract

## Plan folder

Create these hot files under `_plans/<short-slug>/`:

- `original-spec.md`: source text or a durable source pointer with snapshot context and a binding-requirement summary
- `spec.md`: controlling router and implementation contract
- `progress.md`: short, current resume state
- `audit.md`: completion dashboard and latest verification snapshot

Add `phases/`, `audits/`, `context/`, `decisions/`, or `evidence/` only when detail would bloat a hot file or serves a distinct phase. Start every support file with:

```markdown
Read when: <specific condition>
Do not read when: <specific condition>
Temperature: Hot|Warm|Cold
```

Reference every support file from a hot file. Keep `spec.md` under 200 lines, `progress.md` under 60, and `audit.md` under 150 when practical.

## Requirements and ownership

Classify source statements as binding, inferred, rejected or non-goal, or unresolved. Treat a requirement as binding only when it is current, unrejected, and material.

Give binding requirements stable IDs. Model architecture, state placement, ownership, lifecycle, concurrency, persistence, API shape, generated artifacts, and tests as first-class requirements when material. If the user names a specific component, module, actor, manager, owner, or data model, add a no-equivalent-substitution requirement: matching behavior alone does not satisfy it.

Use compact code anchors such as paths, modules, types, functions, commands, and test names. Do not create exhaustive call-site maps.

Treat newer user instructions, repository instructions, and verified implementation facts as higher priority than the spec. Require explicit user approval before changing a hard requirement, ownership invariant, no-equivalent-substitution requirement, boundary, acceptance criterion, or verification scope.

## Evidence and completion

Define exact automated checks, generated artifacts, inspections, and manual checks. State the evidence each produces and map it to requirement IDs. Include counter-evidence checks where a superficially passing result could hide a violated owner, stale path, fallback, compatibility issue, or missing generated output.

Require all of the following for completion:

- the objective is achieved
- every binding requirement and acceptance criterion is satisfied
- required verification passes, or an explicitly irrelevant check is marked N/A with a reason
- `audit.md` contains concrete evidence references for every row
- no unapproved deviation or unresolved blocker remains

Call `update_goal` with `complete` only after the objective and required evidence are achieved. If an irreducible blocker remains after all other in-scope work, call `update_goal` with `blocked` only when the active runtime policy permits that transition. Otherwise leave goal status unchanged and report the blocker, attempts, evidence, remaining criteria, and required input. Never use completion as an escape from blocked work.

## Runtime and resumption

At execution start, read repository instructions, `spec.md`, `progress.md`, and current repository state. On continuation, read only the router sections and files selected by the Read Map for the next action. Open `audit.md` when updating evidence or evaluating completion. Do not reload every support file.

Keep `progress.md` current and concise after meaningful state changes, phase handoffs, or new blockers. Keep `audit.md` as a latest-state dashboard rather than a command diary. Archive detailed evidence in support files.

After a failed or partial check, inspect the evidence, make the smallest contract-preserving change, rerun focused verification, and continue. Reserve broad verification and final-only audits for milestones and completion.

Inherit the current user, repository, and runtime delegation policies. Add a runtime-policy section only for task-specific deviations or genuinely required review gates. Do not require generic delegation behavior, model selection, worker waves, or independent review unless the task or user requires it.
