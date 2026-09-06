---
name: subagent-workflow
description: Route work across Astra, Fable 5.1, Opus, Grok, Sol, and Luna. Use only when the user explicitly requests subagent-workflow by name.
---

# Subagent Workflow

## Select the root and scope

Activate only when the user explicitly requests `subagent-workflow`, including `$subagent-workflow` or `/subagent-workflow`. A general request for review or delegation does not activate it.

Use the actual session model as root. The client name does not identify the model. An explicit root choice is a routing request, not proof that the running model changed. If the requested root is unavailable, report that limitation without silently substituting a model.

The root owns scope, design decisions, integration, and acceptance. Delegate independent work only when it saves time or improves the result. Continue useful independent work while delegates run. A small local task can stay in the root thread.

## Choose by task

These are local routing defaults, not benchmark scores. Fable 5.1's slight advantage in code ready for merge and cleanup is Praveen's working assessment. Read [model-routing.md](references/model-routing.md) when a model choice needs supporting evidence or a tradeoff is unresolved.

| Model | Strengths and preferred work | Limits to account for | Default effort |
| --- | --- | --- | --- |
| GPT-6 Astra | Architecture, difficult diagnosis and review, broad investigation, work across code, tools, and applications | Can pause early, follow conflicting skill rules too strictly, and run excess checks; code cleanup still benefits from Fable | `high` |
| Claude Fable 5.1 | Slightly better code ready for merge, focused cleanup, API shape, maintainability, and writing | Can expand scope, add excess tests, rewrite whole files, or stop before completion | `high` |
| Claude Opus 5 | Long-running coding, bug finding, and a deliberate Claude second opinion | Can add process, verification, subagents, or prose beyond the task | `high` |
| Grok 4.6 | Native X research, live evidence, visual or interactive first passes, and a third-provider review | Check project-specific code quality; do not infer production readiness from a good demo | `high` |
| GPT-5.6 Sol | Read-only phase review in multi-phase work: correctness, missed consumers, failure paths, and inventory re-derivation | Overbuilds when it implements, and is not an authority on taste or simplification; in this workflow it reviews rather than writes | `high` |
| GPT-5.6 Luna | Bounded implementation after design is settled; exact mechanical work | Literal execution cannot replace diagnosis, architecture, or final acceptance | `max` for bounded work; `low` for bulk mechanical work |

Use Luna for ordinary bounded implementation with cheap checks. Use Fable 5.1 when implementation needs sustained code-quality judgment or cleanup. Reserve Astra for difficult reasoning, architecture, investigation, and review; it can implement a hard coupled change when separating diagnosis from the fix would lose essential context. Do not route all former implementation work to Astra. Use Sol as the per-phase reviewer of Luna-written code when a goal has many phases, so Astra reviews less often and only the hard parts; a small task does not need Sol.

Read only the active root's reference:

| Root | Reference |
| --- | --- |
| GPT-6 Astra | [root-astra.md](references/root-astra.md) |
| Claude Fable 5.1 | [root-fable.md](references/root-fable.md) |
| Claude Opus 5 | [root-opus.md](references/root-opus.md) |
| Grok 4.6 | [root-grok.md](references/root-grok.md) |

If another model is root, retain it and use the task table without pretending it is one of these roots.

## Honor model choices

A user-selected model takes precedence over these defaults. `use astra`, `use fable`, `use opus`, `use grok`, or `use sol` selects that model for the requested work until the user changes the selection; it does not change the running root. Fable means **Fable 5.1** in this workflow. `use luna` selects Luna `max` for implementation under the root's design and acceptance; follow the `use-luna-max` skill when active.

Do not silently fall back to another model or lower effort. Keep explicit user budgets and effort choices. Compare lower effort only when tuning is requested or observed task cost justifies a scoped comparison.

## Give each delegate a short contract

Specify the outcome, observable completion conditions, mode (`read-only analysis` or `implementation`), owned and excluded scope, evidence, required checks, and material stop conditions. Preserve exact user constraints. Point to code and task-specific references instead of copying skill files. Luna also needs the decided approach and exact acceptance checks; frontier models need room to choose the method.

Require delegates to preserve unrelated work, follow applicable repository instructions, and report results, changed files, check outcomes, and unresolved issues. Keep staging, commits, publication, external writes, and nested delegation with the root unless separately authorized. A plan-only request ends with a plan. An implementation request includes the authorized repair and verification needed to meet its completion conditions.

Never assign overlapping writing scopes. Read-only reviewers can share a snapshot. Transfer ownership before another model edits the same files.

## Use the available transport

Prefer native subagents when they expose the requested model and effort. For Codex workers, default to `fork_turns="none"` and a self-contained prompt; use a full-history fork only when the user requests it. Inspect the available model choices instead of assuming another provider is supported.

Read the relevant transport reference only before using it:

- [codex-cli.md](references/codex-cli.md): external Codex runs, shared CLI prompt template, and evidence capture; do not launch a second CLI when native workers suffice
- [claude-cli.md](references/claude-cli.md): Claude CLI or Agent-tool model selection, read-only review, and scoped implementation
- [grok-cli.md](references/grok-cli.md): Grok headless permission preflight and run commands

For Astra, Fable, or Sol delegates, read [frontier-prompting.md](references/frontier-prompting.md) when constructing model-specific instructions. For Opus, use [opus5-prompting.md](references/opus5-prompting.md). Load only the selected model's section.

## Accept and repair the result

Check the actual diff and completion evidence. Reject read-only mutations and changes outside the owned scope. Capture relevant baseline and final state plus the delegate result under `_scratch/subagent-workflow/<run-id>/`. For CLI runs, retain the exit status and tool trace or logs; a final message alone is not proof.

Complete required checks on the integrated code. Reuse reliable results for unchanged code; repeat or broaden checks only for new changes, failures, missing evidence, or unresolved concerns. Add permanent tests for behavior or non-obvious invariants, not to reproduce edited literals.

For substantial work, select independent review by the main risk: Astra for correctness and missed consumers, Sol for per-phase correctness review when a multi-phase goal makes Astra review of every phase too costly, Fable 5.1 for merge readiness and simplification, or Grok for a distinct third-provider concern. Scale review to task size; the active root's reference states the tiers. Review the actual code and failure scenarios, not only the plan's checklist. Check relevant lifecycle changes, error paths, and runtime limits. Report behavior that still needs runtime or device evidence. Do not add a review panel to a trivial edit.

A reviewer must not be the same run that wrote the code. Fable may simplify its own work, but that is not independent review. When Fable implements, use Astra for consequential correctness review; request a fresh Fable view only when independent code-quality judgment adds value.

Return a concrete defect to its author with its location, consequence, required end state, owned scope, and relevant checks. Use a fresh pass for independent reasoning or a follow-up when context helps. After two failed repairs of the same defect, return the design decision to the root or move the work to a more suitable model. Do not repeat the same failed instructions.

Remove only confirmed out-of-scope changes made by the delegate, preserving concurrent edits. The root can repair an urgent break or take back work when integration costs exceed the benefit of another handoff. State a material change of route briefly.
