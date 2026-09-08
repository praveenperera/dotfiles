---
name: subagent-workflow
description: Route work across Astra, Fable 5.1, Opus, Grok, Sol, and Luna. Use only when the user explicitly requests subagent-workflow by name.
---

# Subagent Workflow

## Select the root and scope

Activate only when the user explicitly requests `subagent-workflow`, including `$subagent-workflow` or `/subagent-workflow`. A general request for review or delegation does not activate it.

Use the actual session model as root. The client name does not identify the model. An explicit root choice is a routing request, not proof that the running model changed. If the requested root is unavailable, report that limitation without silently substituting a model.

The root owns scope, design decisions, integration, and acceptance. Delegate independent work only when it saves time or improves the result. Continue useful independent work while delegates run. A small local task can stay in the root thread, and the root may make a small edit when a handoff costs more than it saves.

## Choose by task

These are local routing defaults, not benchmark scores. Praveen's working assessment is that Astra is the most capable and most expensive Codex option, Sol is the usual lower-cost root for normal work, and Fable 5.1 is an expensive Claude option that is slightly better at quality-sensitive implementation and cleanup. Fable is opt-in only. Read [model-routing.md](references/model-routing.md) when a model choice needs supporting evidence or a tradeoff is unresolved.

| Model | Strengths and preferred work | Limits to account for | Default effort |
| --- | --- | --- | --- |
| GPT-6 Astra | High-level complex planning, architecture, hard ambiguity, difficult or high-risk review, reducing complexity, and cross-phase risk; can establish front-end design | Most expensive Codex option; can pause early, follow conflicting skill rules too strictly, and run excess checks | `medium` by default; `low` for focused bounded questions |
| Claude Fable 5.1 | Opt-in only. Strong at public API shape, simplification or removal, merge-ready cleanup, and front-end design | Expensive Claude 5-hour limit; can expand scope, add excess tests, rewrite whole files, or stop before completion | `high` |
| Claude Opus 5 | Long-running coding, bug finding, front-end design, and a deliberate Claude second opinion | Can add process, verification, subagents, or prose beyond the task | `high` |
| Grok 4.6 | Native X research, live evidence, visual or interactive first passes, and a third-provider review | Check project-specific code quality; do not infer production readiness from a good demo | `high` |
| GPT-5.6 Sol | Usual root for normal diagnosis, bounded implementation, integration, and acceptance; fresh read-only phase review when assigned | Not an authority on simplification or removal; a reviewer is read-only only for that assignment | `high` |
| GPT-5.6 Luna | Bounded implementation after design is settled; exact mechanical work | Literal execution cannot replace diagnosis, architecture, or final acceptance | `max` for bounded work; `low` for bulk mechanical work |

Use Luna `max` for ordinary bounded implementation with cheap checks. Use Sol as the usual root for normal diagnosis, bounded implementation, integration, and acceptance; a small root edit is allowed when a handoff costs more than it saves.

Do not launch Fable 5.1 unless the user opts in. The root may suggest specific remaining work for Fable, with a short reason, then continue on the default route until the user accepts.

Reserve expensive Astra passes for high-level complex planning, architecture, hard ambiguity, difficult or high-risk review, reducing complexity, and cross-phase risk. Run Astra at `medium` by default, use `low` for focused bounded questions, and never select `high` or above unless the user explicitly requests it. Use Astra to decide simplification and code removal unless the user opts into Fable for that work. Sol may integrate an approved removal and check its correctness, but does not judge the simplification.

For front-end design, choose Astra or Opus 5 based on user preference and task complexity; the root may suggest Fable. Once the overall design is established, Sol can implement and extend it. Do not add a model panel by default. In a larger multi-phase goal, a fresh Sol run can review each phase while the root reviews integrated milestones and final acceptance. Independent means a separate run, not necessarily a different model; do not duplicate routine investigations or checks that already have sufficient evidence. A small task does not need an extra reviewer, but the root still checks delegated work.

Read only the active root's reference:

| Root | Reference |
| --- | --- |
| GPT-6 Astra | [root-astra.md](references/root-astra.md) |
| Claude Fable 5.1 | [root-fable.md](references/root-fable.md) |
| Claude Opus 5 | [root-opus.md](references/root-opus.md) |
| Grok 4.6 | [root-grok.md](references/root-grok.md) |
| GPT-5.6 Sol | [root-sol.md](references/root-sol.md) |

If another model is root, retain it and use the task table without pretending it is one of these roots.

## Honor model choices

A user-selected model takes precedence over these defaults. `use astra`, `use fable`, `use opus`, `use grok`, or `use sol` selects that model for the requested work until the user changes the selection; it does not change the running root. Fable means **Fable 5.1** in this workflow and is opt-in: launch it only after `use fable` or an explicit accept of a Fable suggestion. A root suggestion is not an opt-in. `use luna` selects Luna `max` for implementation under the root's design and acceptance; follow the `use-luna-max` skill when active.

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

The root keeps a complexity-check list in `_scratch/subagent-workflow/<run-id>/complexity-check.md`. Add an entry when new code, a compatibility path, a duplicate owner, or an oversized change may be reducible later: path, short reason, and when it appeared. Do not scan the rest of the repository. When the work is otherwise done, Astra checks that list and decides what to remove. The root may suggest Fable for the same list; launch Fable only after opt-in. Skip the pass when the list is empty.

Complete required checks on the integrated code. Reuse reliable results for unchanged code; repeat or broaden checks only for new changes, failures, missing evidence, or unresolved concerns. Add permanent tests for behavior or non-obvious invariants, not to reproduce edited literals.

For substantial work, select independent review by the main risk: Astra for hard or high-risk scope, design, integration, acceptance, ambiguity, diagnosis, and cross-phase risk; Sol for bounded routine analysis, phase correctness, and ordinary merge-readiness when a multi-phase goal warrants a fresh run; or Grok for a distinct third-provider concern. The root may suggest Fable for simplification or a quality pass; launch it only after opt-in. Scale review to task size; the active root's reference states the tiers.

Sol reports should be brief and evidence-backed, with checked paths or consumers, confirmed defects, and residual risks. Escalate automatically to Astra only when the evidence leaves a hard or high-risk decision unresolved after Sol's investigation. A boundary signal alone is not enough. These review escalation limits do not restrict the Astra planning, architecture, front-end design, or simplification routes above. Focus the Astra request on the evidence and decision; do not repeat investigations or checks that already have sufficient evidence. Review relevant code and failure scenarios, using sufficient Sol evidence instead of repeating a complete routine inventory. Check relevant lifecycle changes, error paths, and runtime limits. Report behavior that still needs runtime or device evidence. Do not add a review panel to a trivial edit.

A reviewer must be a separate run from the run that wrote the code; independent does not require a different model. A Sol delegate assigned review is read-only, but Sol as root may implement and integrate ordinary work. Route simplification and removal judgment to Astra from the complexity-check list unless the user opts into Fable. Sol may integrate an approved removal and check correctness. Do not send Fable-written changes to Astra automatically. Use Astra for a hard or high-risk question, or honor an explicit user choice for Astra simplification or overall front-end design, and use a focused evidence-backed request.

Return a concrete defect to its author with its location, consequence, required end state, owned scope, and relevant checks. Use a fresh pass for independent reasoning or a follow-up when context helps. After two failed repairs of the same defect, return the design decision to the root or move the work to a more suitable model. Do not repeat the same failed instructions.

Remove only confirmed out-of-scope changes made by the delegate, preserving concurrent edits. The root can repair an urgent break or take back work when integration costs exceed the benefit of another handoff. State a material change of route briefly.
