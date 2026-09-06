# Astra, Fable 5.1, and Sol prompts

Read only the section for the selected model. Use the short task contract in [SKILL.md](../SKILL.md); add only the corrections relevant to this task.

## GPT-6 Astra

State the intended result, scope, evidence, and completion conditions. Leave routine method choices to Astra. For implementation, include the authorized repair and checks needed for completion; for planning, make the deliverable a plan.

When unnecessary pauses occur, make existing authority explicit. A skill guideline does not cancel a user's request. Ask for the exact rule and source when it blocks progress. Preserve real external-action limits.

Resolve conflicting instructions at their source. Keep required checks, and expand them only for changed code, failures, or an unresolved concern. Give an independent delegate a concrete task when it adds value; do not prescribe a fixed team size.

Source: [OpenAI Astra prompt guidance](https://developers.openai.com/api/docs/guides/latest-model#prompting-best-practices).

## Claude Fable 5.1

Describe the full requested behavior and completion conditions. Keep existing code examples and exact constraints accessible. For cleanup, identify the intended quality improvement and the behavior that must stay unchanged.

Limit unrequested nearby fixes and permanent tests. Prefer targeted edits for small changes. Ask for brief progress updates if the run is silent, and batch independent reads when one-at-a-time calls slow it down. Keep the root working on independent tasks while delegates run. Retain original user constraints in handoffs.

Start at `high`; higher effort can delay long output. State current search needs explicitly at low effort. Use plain language for reports.

Source: [Anthropic Fable 5.1 prompt guidance](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5-1).

## GPT-5.6 Sol

Use Sol for read-only review passes. Give it the phase's objective, the diff or changed files, the consumers and call sites it must re-derive, and the failure paths to check. Ask for concrete defects with location, consequence, and the observable end state that would resolve each one; ask for a short list of residual risks that need runtime evidence.

Sol overbuilds when it implements and can turn a review into a redesign proposal. Keep the mode read-only, tie findings to the current design, and ask it to separate confirmed defects from speculative concerns. Do not ask Sol for taste, surface design, or simplification judgment; that stays with Fable 5.1.

Run Sol at `high`. There is no local evidence that `xhigh` or `max` improves review quality.
