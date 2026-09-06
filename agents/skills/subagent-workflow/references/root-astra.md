# GPT-6 Astra root

Use this route only when the actual session model is GPT-6 Astra. Keep architecture, diagnosis, integration, and acceptance in the root thread. Astra is the most expensive model in this workflow, so spend its passes on design, hard diagnosis, and the final review of the hard parts, not on routine implementation or routine phase review.

| Work | Route |
| --- | --- |
| Domain design, ambiguous decisions, hard debugging, broad investigation | Astra root |
| Bounded implementation with a decided approach and cheap checks | Native Luna `max` worker |
| Implementation that needs sustained code-quality judgment; focused cleanup | Fable 5.1 |
| Hard fix tightly coupled to the root's diagnosis | Astra root when a handoff would lose essential context |
| Per-phase review of Luna-written code in a multi-phase goal | Sol `high`, read-only |
| Milestone review of the hard parts after a chunk of phases | Astra root |
| Independent review of root-written code | Fable 5.1 for merge readiness; Grok or a fresh Astra run for a distinct correctness concern |
| Merge readiness and simplification when the change needs it | Fable 5.1 |
| Native X research or a visual first pass with clear scope | Grok 4.6 |
| Deliberate additional Claude opinion | Opus 5 only when it adds a distinct perspective |
| Bulk exact transformations | Luna `low`, unless an active user choice requires `max` |

## Scale review to task size

**Small task** (one phase, or a few bounded edits): Luna `max` implements. The Astra root reviews the diff and the check results, then accepts or returns a defect. Do not add Sol; a second reviewer costs more than it saves here.

**Larger goal** (many phases, or a plan with milestones): Luna `max` implements each phase. After each phase, a fresh read-only Sol `high` run reviews that phase for correctness, missed consumers, failure paths, and re-derived inventories, and returns concrete defects to the root. The Astra root does not review every phase. After a larger chunk of phases, the root reviews the integrated result and concentrates on the hard parts: cross-phase interactions, design fit, lifecycle and error paths, and any concern Sol reported as unresolved. Read Sol's reports and the integrated diff; do not repeat Sol's inventory work or rerun checks that passed on unchanged code.

Sol is a reviewer in this workflow, not an implementer. Return Sol's defects to Luna with location, consequence, and required end state. After two failed repairs of one defect, the root takes the design decision back. Fable 5.1 still owns merge readiness and simplification when a phase or milestone needs that judgment; Sol is not an authority on taste.

## Transport and prompting

Use native Codex workers when available. Use [claude-cli.md](claude-cli.md) for Fable 5.1 or Opus and [grok-cli.md](grok-cli.md) for Grok. External Codex transport for Astra, Sol, and Luna is in [codex-cli.md](codex-cli.md).

Apply the Astra section of [frontier-prompting.md](frontier-prompting.md) to the root as well as its delegates, and the Sol section to Sol review prompts. Do not let a skill's routine approval step interrupt work already authorized by the user. Keep plan-only requests limited to planning.
