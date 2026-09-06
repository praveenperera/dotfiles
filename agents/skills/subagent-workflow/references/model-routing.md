# Model routing evidence

Reviewed on 2026-09-05. Use the task table in [SKILL.md](../SKILL.md) as the single source of routing defaults. The choices below combine provider guidance with local experience; they are not a cross-provider benchmark or a fixed cost ranking.

## Astra and Fable 5.1

OpenAI positions Astra for complex work across reasoning, coding, research, computer use, and documents. Its prompt guide identifies early approval pauses, sensitivity to skill instructions, lower-than-desired delegation, and excess testing. These support its local role in architecture, diagnosis, broad investigation, and difficult review. See [Using GPT-6 Astra](https://developers.openai.com/api/docs/guides/latest-model#prompting-best-practices).

Anthropic positions Fable 5.1 for demanding reasoning, coding, and long-running work. Its current guide documents scope expansion, excess test files, premature completion, and whole-file rewrites. These are concrete limits to check, not a reason to assume every run fails. See [Fable 5.1 capabilities](https://platform.claude.com/docs/en/models/fable-5-1/whats-new-fable-5-1) and [Fable 5.1 prompting](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5-1).

Praveen's assessment on 2026-09-04 is that Fable 5.1 is still slightly better at writing code ready for merge and cleaning up code. Treat that as local evidence for implementation and cleanup routing, not a published result or proof that it leads on every coding task.

Use the distinction as follows:

- Unknown cause, unclear domain boundary, difficult interactions, or many consumers to inspect: Astra owns the reasoning
- Design is understood but implementation quality needs sustained judgment: Fable 5.1 owns the writing
- Approach is settled and correctness is cheap to check: Luna owns the bounded edit
- Code works but needs focused simplification or a merge-readiness review: Fable 5.1
- Fable wrote a consequential change: Astra independently checks correctness and missed cases

Keep the design with the current root when its context is already sufficient. Do not hand off merely to satisfy a model label. Astra can implement a tightly coupled hard fix; Fable can design and debug. The table is a preferred division of work, not a capability boundary.

## Other models

**Opus 5:** Anthropic's current guide supports long-running coding and bug finding, while warning about scope growth, over-verification, verbosity, and excessive delegation. Use it for an explicit user choice, an Opus-root task, or a distinct Claude second opinion. Do not carry launch-era claims of weak instruction following into the current defaults without current local evidence. See [Opus 5 prompting](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5).

**Grok 4.6:** Keep native X research and live evidence with Grok when those tools are useful. Its visual and interactive work can supply a first pass. Fable 5.1 checks code quality; Astra checks difficult repository interactions when warranted. Do not extrapolate a provider's aggregate scores into a universal code-quality ranking. See [Grok 4.6](https://docs.x.ai/developers/grok-4-6).

**Sol:** GPT-5.6 Sol was the Codex root before Astra and served as the adversarial and inventory-style reviewer in many local runs. Praveen's assessment on 2026-09-05 is that Sol review is a cost-effective middle tier under an Astra root: it checks each phase of a multi-phase goal so that Astra reviews only the integrated milestones and the hard parts. Local notes from the Sol-root period record that Sol overbuilds when it implements and is not a taste or simplification authority, so it reviews rather than writes in this workflow, and Fable 5.1 keeps taste. A small task skips Sol; two reviewers on a bounded edit cost more than they save.

**Luna:** Preserve the local workload split: `max` for bounded implementation or low-count analysis, `low` for high-volume exact transformations. The root settles diagnosis and design, and owns acceptance. Do not assign architecture, subtle security decisions, or open-ended cleanup to Luna. An explicit `use luna` request keeps `max` for implementation, including mechanical work, until the user changes that choice.

## Effort and cost

Keep Astra and Fable 5.1 at `high` by default. Anthropic recommends starting Fable 5.1 at `high` and evaluating other settings; equal effort names do not imply equal reasoning across models. API price alone does not determine total task cost. Include retries, context, review, and subscription limits before changing a route.

If tuning is requested, compare the same representative tasks and acceptance conditions at different efforts. Record completion, defects, extra changes, elapsed time, and available usage figures. Do not change the user's defaults from a launch anecdote.

## Instruction maintenance

[Provencher's Astra article](https://x.com/pvncher/status/2095991462416490862) recommends concise skill descriptions, conditional reference loading, and clear completion boundaries. [Codex best practices](https://learn.chatgpt.com/guides/best-practices) likewise favors short, accurate instructions grounded in repeated problems.

Keep transport mechanics separate from model judgment. Add a behavioral correction only when it addresses a documented model behavior or a repeated local failure. Recheck the relevant source when a model version or observed behavior changes.
