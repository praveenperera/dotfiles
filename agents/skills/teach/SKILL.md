---
name: teach
description: Explain a change, subsystem, or body of work so the user actually understands what it is, how it works, and why it is built that way. Runs `how` and `why` and blends their findings into one plain account. Use when the user invokes $teach or /teach.
disable-model-invocation: true
---

# Teach

Explain what a thing is, how it works, and why it has its shape, in one plain account at the user's pace. The goal is understanding, not a change.

## Gather

1. Decide the few things the user should walk away understanding. Choose them from why they are asking, such as changing, reviewing, debugging, or onboarding, and from what the conversation shows they already know. Skip what they plainly know.
2. Read enough code to get oriented, then run [how](../how/SKILL.md) for how it works and [why](../why/SKILL.md) for why. Run them in parallel and size them to the question: both for a subsystem, one may be enough for a small change. Keep `why` narrow by default, such as git history plus one or two other sources, and widen it only when the reasons are the point.
3. Let those skills do the digging. Do not redo their work.

## Explain

- Start with a plain definition: name the thing, say what it is in general terms with its common name if it has one, then tie it to this codebase.
- Build from there: how it works, the deeper reasons, then edge cases. For each part, explain the problem it solves and the concrete mechanism. Walk through what happens as the user does the thing when that makes it land.
- Give the smallest complete answer first, a sentence or two, then stop. Add layers when the user asks. Never write a wall of text.
- Keep `why`'s confidence wording intact. Its hedges are findings, not style.
- Listing functions and constants is reference material, not teaching.

## Show

Open the diff or the code when that is the fastest way to land a point. For anything with three or more moving parts, draw a short series of diagrams instead of one: each redraws the last and adds one part, so the user watches the system assemble. Use a Mermaid diagram for flows and structure. A single simple point needs no figure.

## Voice

Write through the `deslop-writing` skill, in plain spoken English, the way you would explain it to a colleague. State the mechanism, not a metaphor or a preview of what is coming. Give each concept one name and keep it. Do not print framing labels such as "the key insight" or "TL;DR", and do not flag a part as tricky or important; just explain it.

Keep it a conversation. No quizzes. When you would pause, stop and let the user respond. When running without a live user, deliver the explanation cleanly and put any offer to go deeper at the end.

The reply is the explanation itself, not a report about what you did. Lead with the main point and end with the threads worth chasing with `how` or `why`.
