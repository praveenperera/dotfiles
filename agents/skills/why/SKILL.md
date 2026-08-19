---
name: why
description: Find evidence for why code, an architecture, a threshold, or a product decision has its current shape. Use for design rationale, regressions, postmortems, historical decisions, or questions about why an implementation exists. Use `how` for mechanics without motivation.
---

# Why

Recover rationale from evidence. Do not turn the current code shape into a story about its intent.

## Anchor the question

Identify the exact files, symbols, behavior, change, or decision in question. Establish the mechanics with a focused code trace when they are unclear. Do not invoke `how` only because this skill is active.

## Search the sources that can answer

Start with source control because it connects rationale to the change that shipped:

- current code, tests, and comments
- `git log`, `git blame`, commits, pull requests, and review threads

Then inspect only the available records that can materially change the answer:

- issues and project trackers for product or operational pressure
- design documents, ADRs, and meeting notes for considered alternatives
- team chat for decisions that never reached a document
- observability and error tracking for runtime evidence
- analytics for usage, scale, or threshold evidence

Use the relevant connector or CLI when one exists. A source that is unavailable is a gap. A relevant search with no result is a finding. Do not require a large source sweep for a narrow question whose answer is already explicit.

## Weigh the result

Classify every claim:

- **Direct evidence:** a source states the reason
- **Supported inference:** several facts point to a reason that no source states
- **Unknown:** the record does not support an answer

Give stable citations such as commit hashes, pull request numbers, ticket IDs, document links, chat permalinks, metric names, and file symbols. Do not cite code behavior as proof of its own motivation.

## Return

State the answer first. Then give the direct evidence, supported inferences, competing explanations when needed, and material gaps. If the user plans to change the code, finish with the constraints that the evidence says to preserve.

This is a read-only skill. Do not change external records or code unless the user separately asks for that action.
