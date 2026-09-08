# GPT-6 Astra root

Default Astra to `medium`; use `low` for focused work. Use `high` or above only when the user explicitly requests that effort. Do not claim to change the running root's effort through these instructions.

Use this route only when the actual session model is GPT-6 Astra. Praveen's assessment is that Astra is the most capable and most expensive option in this workflow. Keep scope, design, integration, and acceptance in the root thread, and spend Astra's passes on hard ambiguity, difficult diagnosis, and cross-phase risk rather than routine analysis or routine phase review.

| Work | Route |
| --- | --- |
| Scope, domain design, hard ambiguity, difficult debugging, cross-phase risk, integration, and acceptance | Astra root |
| Bounded implementation with a decided approach and cheap checks | Native Luna `max` worker |
| Implementation that needs sustained code-quality judgment; focused cleanup | Fable 5.1 |
| Hard fix tightly coupled to the root's diagnosis | Astra root when a handoff would lose essential context |
| Routine investigation, code and consumer mapping, evidence gathering, normal diagnosis, and phase correctness | Sol `high`, read-only analyst/reviewer |
| Milestone review of the hard parts after a chunk of phases | Astra root |
| Independent review of root-written code | Fable 5.1 for merge readiness; Grok or a fresh Astra run for a distinct correctness concern |
| Merge readiness | Fable 5.1 |
| Reducing complexity and deciding which code to remove | Astra root or Fable 5.1; Sol checks correctness, not simplification quality |
| Front-end design | Astra root, Opus 5, or Fable 5.1; select one unless the user asks for alternatives |
| Front-end implementation and extension after the overall design is established | Sol, or Luna for bounded implementation |
| Native X research or a visual first pass with clear scope | Grok 4.6 |
| Deliberate additional Claude opinion | Opus 5 only when it adds a distinct perspective |
| Bulk exact transformations | Luna `low`, unless an active user choice requires `max` |

## Scale review to task size

**Small task** (one phase, or a few bounded edits): Luna `max` implements. The Astra root reviews the diff and the check results, then accepts or returns a defect. Do not add Sol unless a bounded investigation materially reduces root work; there is no mandatory Sol pass for a trivial edit.

**Larger goal** (many phases, or a plan with milestones): Astra sets scope and design; Luna `max` implements each decided phase. Use Sol `high` for routine investigation, consumer mapping, evidence gathering, and normal diagnosis. After each phase, a fresh read-only Sol run checks correctness, missed consumers, and failure paths. Sol returns a brief evidence-backed report with checked paths, confirmed defects and locations, consequences, required end states, and residual risks. At integrated milestones and final acceptance, Astra reviews the integrated diff and these reports, with detailed review focused on hard parts: cross-phase interactions, design fit, lifecycle and error paths, and unresolved concerns. Do not repeat Sol's routine inventory or rerun passed checks on unchanged code.

Sol delegates assigned analysis or review are read-only; this is not a restriction on a Sol root in another session. Bring unresolved design choices and difficult risks to the Astra root with focused evidence. Return confirmed defects to their author. After two failed repairs of one defect, the root takes the design decision back. Astra or Fable 5.1 leads simplification and code removal; Sol checks correctness and does not judge simplification quality.

## Transport and prompting

Use native Codex workers when available. Use [claude-cli.md](claude-cli.md) for Fable 5.1 or Opus and [grok-cli.md](grok-cli.md) for Grok. External Codex transport for Astra, Sol, and Luna is in [codex-cli.md](codex-cli.md).

Apply the Astra section of [frontier-prompting.md](frontier-prompting.md) to the root as well as its delegates, and the Sol section to Sol review prompts. Do not let a skill's routine approval step interrupt work already authorized by the user. Keep plan-only requests limited to planning.
