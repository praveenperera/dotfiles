# Astra, Fable 5.1, and Sol prompts

Read only the section for the selected model. Use the short task contract in [SKILL.md](../SKILL.md); add only the corrections relevant to this task.

For front-end design, choose Astra, Opus 5, or Fable 5.1 based on user preference and task complexity. Do not add a model panel by default. Once the overall design is established, Sol can implement and extend it.

## GPT-6 Astra

State the intended result, scope, evidence, and completion conditions. Leave routine method choices to Astra. Focus the request on evidence already gathered and the exact decision that remains; do not ask Astra to repeat routine investigations or checks. For implementation, include the authorized repair and checks needed for completion; for planning, make the deliverable a plan.

Under an Astra root, keep scope, design, integration, and acceptance in the root. Use a fresh Sol `high` read-only analyst/reviewer for routine investigation, code and consumer mapping, evidence gathering, normal diagnosis, or phase correctness when the task size warrants it. Do not repeat Sol's routine inventory or inspect every phase deeply by default. Escalate only when the evidence leaves a hard or high-risk decision unresolved after investigation; a boundary signal alone is not enough.

Run Astra at `medium` by default. Use `low` for focused bounded questions, and never select `high` or above unless the user explicitly requests it.

When unnecessary pauses occur, make existing authority explicit. A skill guideline does not cancel a user's request. Ask for the exact rule and source when it blocks progress. Preserve real external-action limits.

Resolve conflicting instructions at their source. Keep required checks, and expand them only for changed code, failures, or an unresolved concern. Give an independent delegate a concrete task when it adds value; do not prescribe a fixed team size.

Source: [OpenAI Astra prompt guidance](https://developers.openai.com/api/docs/guides/latest-model#prompting-best-practices).

## Claude Fable 5.1

Describe the full requested behavior and completion conditions. Keep existing code examples and exact constraints accessible. For cleanup, identify the intended quality improvement and the behavior that must stay unchanged.

Limit unrequested nearby fixes and permanent tests. Prefer targeted edits for small changes. Ask for brief progress updates if the run is silent, and batch independent reads when one-at-a-time calls slow it down. Keep the root working on independent tasks while delegates run. Retain original user constraints in handoffs.

Start at `high`; higher effort can delay long output. State current search needs explicitly at low effort. Use plain language for reports.

Fable can own quality-sensitive implementation, cleanup, simplification, and front-end design. Do not send Fable-written changes to Astra automatically; use a fresh Sol run for normal correctness review and reserve automatic Astra escalation for a hard or high-risk unresolved question. Honor an explicit user choice for Astra simplification or overall front-end design.

Source: [Anthropic Fable 5.1 prompt guidance](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5-1).

## GPT-5.6 Sol

When assigned a review, use Sol as a read-only analyst/reviewer for routine investigation, code and consumer mapping, evidence gathering, normal diagnosis, and phase correctness. Give it the objective, relevant diff or changed files, consumers and call sites to check, and failure paths in scope. A Sol root may implement ordinary work, integrate an approved removal, and check correctness; reviewer read-only mode is an assignment, not a Sol-wide prohibition. After an overall front-end design is established by Astra, Opus 5, or Fable 5.1, Sol may implement and extend that design. Ask a fresh review run for a brief evidence-backed report: paths or symbols checked, confirmed defects with location and consequence, the observable end state that would resolve each defect, and a short list of residual risks that need runtime evidence.

Keep an assigned review read-only, tie findings to the current design, and ask Sol to separate confirmed defects from speculative concerns. Route simplification and removal judgment to Astra or Fable 5.1; Sol may integrate an approved removal and check correctness, but does not decide what to remove. Escalate to Astra only when the evidence leaves a hard or high-risk question unresolved after Sol's investigation. A boundary signal alone is not enough. Do not ask Sol for taste or simplification judgment.

Run Sol at `high` for root work and assigned review. A fresh review is independent because it is a separate run, even when it uses Sol again. There is no local evidence that `xhigh` or `max` improves review quality.
