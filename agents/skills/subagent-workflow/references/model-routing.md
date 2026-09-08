# Model routing evidence

Routing updated on 2026-09-08 from Praveen's assessment. Use the task table in [SKILL.md](../SKILL.md) as the single source of routing defaults. The choices below combine provider guidance with local experience; they are not a cross-provider benchmark or a fixed cost ranking.

## Astra and Fable 5.1

OpenAI positions Astra for complex work across reasoning, coding, research, computer use, and documents. Its prompt guide identifies early approval pauses, sensitivity to skill instructions, lower-than-desired delegation, and excess testing. These support its local role in architecture, diagnosis, broad investigation, and difficult review. See [Using GPT-6 Astra](https://developers.openai.com/api/docs/guides/latest-model#prompting-best-practices).

Anthropic positions Fable 5.1 for demanding reasoning, coding, and long-running work. Its current guide documents scope expansion, excess test files, premature completion, and whole-file rewrites. These are concrete limits to check, not a reason to assume every run fails. See [Fable 5.1 capabilities](https://platform.claude.com/docs/en/models/fable-5-1/whats-new-fable-5-1) and [Fable 5.1 prompting](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5-1).

Praveen's assessment is that Astra is the most capable and most expensive option in this workflow. Reserve Astra for high-level complex planning, architecture, hard ambiguity, difficult or high-risk review, difficult diagnosis, reducing complexity, and cross-phase risk. Keep this as a local routing assessment, not an external capability or benchmark claim.

Praveen's assessment on 2026-09-04 is that Fable 5.1 is still slightly better at writing code ready for merge and cleaning up code. Treat that as local evidence for implementation and cleanup routing, not a published result or proof that it leads on every coding task.

Use the distinction as follows:

- Normal diagnosis, bounded implementation, integration, and acceptance: Sol root at `high`; a small root edit is allowed when a handoff costs more than it saves
- Routine investigation, code and consumer mapping, evidence gathering, or normal phase diagnosis under any root: a fresh Sol `high` run when a separate review materially reduces risk
- High-level complex planning, architecture, hard ambiguity, difficult or high-risk review, or an unresolved difficult scope or design decision: Astra provides the recommendation; the root owns the decision
- Design is understood but implementation quality needs sustained judgment: Fable 5.1 owns the writing
- Approach is settled and correctness is cheap to check: Luna `max` owns the bounded edit
- Code works but needs focused simplification, removal, or a merge-readiness review: Astra or Fable 5.1 decides the reduction; Fable usually owns quality cleanup
- Front-end design: choose Astra, Opus 5, or Fable 5.1; after the overall design is established, Sol can implement and extend it

Keep the design with the current root when its context is already sufficient. Do not hand off merely to satisfy a model label. Astra can implement a tightly coupled hard fix; Fable can design and debug. Sol can integrate an approved removal and check correctness, but does not decide what to simplify or remove. Do not duplicate investigations or checks that already have sufficient evidence. The table is a preferred division of work, not a capability boundary.

## Other models

**Opus 5:** Anthropic's current guide supports long-running coding and bug finding, while warning about scope growth, over-verification, verbosity, and excessive delegation. Use it for an explicit user choice, an Opus-root task, or a distinct Claude second opinion. Do not carry launch-era claims of weak instruction following into the current defaults without current local evidence. See [Opus 5 prompting](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5).

**Grok 4.6:** Keep native X research and live evidence with Grok when those tools are useful. Its visual and interactive work can supply a first pass. Fable 5.1 checks code quality; Astra checks difficult repository interactions when warranted. Do not extrapolate a provider's aggregate scores into a universal code-quality ranking. See [Grok 4.6](https://docs.x.ai/developers/grok-4-6).

**Sol:** Praveen's assessment is that Sol is the usual strong, lower-cost root for normal work. Use Sol at `high` for diagnosis, bounded implementation, integration, and acceptance. A Sol run assigned to review is read-only and returns a brief evidence-backed report with checked paths, confirmed defects, and residual risks; this assignment mode is not a Sol-wide prohibition on implementation. In a larger multi-phase goal, a fresh Sol run can review each settled phase while the Sol root reviews integrated milestones and final acceptance. Independent means a separate run, not a different model. Sol can implement established front-end design and integrate approved removal, but Astra or Fable 5.1 decides simplification and removal. Escalate to Astra only when evidence leaves a hard or high-risk question unresolved after investigation; a boundary signal alone is not enough. Do not inspect every phase deeply or repeat a routine inventory by default.

**Luna:** Preserve the local workload split: `max` for bounded implementation or low-count analysis, `low` for high-volume exact transformations. The root settles diagnosis and design, and owns acceptance. Do not assign architecture, subtle security decisions, or open-ended cleanup to Luna. An explicit `use luna` request keeps `max` for implementation, including mechanical work, until the user changes that choice.

## Effort and cost

Use Astra at `medium` by default and `low` for focused bounded questions; never select `high` or above unless the user explicitly requests it. Keep Fable 5.1, Opus 5, and Sol at `high` by default, and Luna at `max` for bounded implementation. Equal effort names do not imply equal reasoning across models. Do not infer capability or task cost from external benchmarks or API price alone. Include retries, context, review, and subscription limits before changing a route.

If tuning is requested, compare the same representative tasks and acceptance conditions at different efforts. Record completion, defects, extra changes, elapsed time, and available usage figures. Do not change the user's defaults from a launch anecdote.

## Instruction maintenance

[Provencher's Astra article](https://x.com/pvncher/status/2095991462416490862) recommends concise skill descriptions, conditional reference loading, and clear completion boundaries. [Codex best practices](https://learn.chatgpt.com/guides/best-practices) likewise favors short, accurate instructions grounded in repeated problems.

Keep transport mechanics separate from model judgment. Add a behavioral correction only when it addresses a documented model behavior or a repeated local failure. Recheck the relevant source when a model version or observed behavior changes.
