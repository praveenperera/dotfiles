---
name: refine-plan
description: Refine an implementation plan or spec through evidence-first investigation and focused user decisions until no material ambiguity remains or the user ends the refinement.
---

# Refine Plan

Refine the current plan through targeted source inspection and a focused interview.

## Evidence first

Before asking a question, inspect the code, documentation, tests, configuration, plan artifacts, and inspectable upstream sources likely to contain the answer. Treat implementation details, public APIs, defaults, tests, and architectural conventions as evidence rather than hypotheticals.

Ask only for intent, product decisions, scope boundaries, tradeoffs, or context that available evidence cannot resolve. Briefly state relevant findings and isolate the remaining decision.

## Refinement loop

1. Identify the material decisions that affect scope, ownership, behavior, user experience, failure handling, security, data lifecycle, compatibility, verification, or maintenance.
2. Inspect available evidence for the next decisions.
3. Ask one to three focused questions with the host's available user-input tool. If the host provides no such tool, ask in normal chat.
4. Incorporate answers into the working plan or spec before the next round.
5. Put valuable but non-blocking follow-ups in `next.md` beside the working file. Do not expand active scope or promote them without user confirmation.
6. Repeat until the stop condition is met.

Probe non-obvious edge cases, integrations, recovery paths, user mental models, performance, scalability, security, cleanup, and tradeoffs only when they are material to the plan.

## Status and stop condition

When editing a plan or spec, preserve its YAML frontmatter and set `status: refining` while the interview is active. If it has no frontmatter, add only that field.

Stop when either:

- every material decision is resolved by source evidence or user choice, explicitly deferred to `next.md`, or deliberately assigned as an execution-time choice without weakening acceptance or verification; or
- the user explicitly ends the refinement.

Then set `status: refined`. Record any unresolved or deferred items in the plan or `next.md` so the status does not imply they disappeared.

Keep `next.md` concise, actionable, and clearly labeled as deferred, blocked, or candidate work. Do not add the status marker to `next.md`.
