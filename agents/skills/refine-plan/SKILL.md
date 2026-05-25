---
name: refine-plan
description: Deep interview for gathering requirements to refine the current plan (user)
---

# Refine Plan

Conduct an in-depth interview to gather requirements using the AskUserQuestion tool.

## Evidence Before Questions

Before asking the user a question, decide whether the answer should already exist in code, docs, tests, config, or plan artifacts.

- Inspect the current repo first with targeted file reads and searches
- If the plan depends on an external repo, upstream project, or library source, use `btx` to inspect that code before asking about its behavior
- Treat existing implementation details, public APIs, config defaults, test coverage, and architectural conventions as evidence to investigate, not hypotheticals for the user
- Ask the user only for intent, product decisions, scope boundaries, preference tradeoffs, or missing context that cannot be answered from source
- When source evidence answers part of a question, summarize the finding briefly and ask only for the unresolved decision

## Topics to Cover

- Technical implementation details
- UI & UX considerations
- Concerns and edge cases
- Tradeoffs and alternatives

## Question Guidelines

Ask non-obvious, probing questions - not surface-level questions:

- Edge cases, failure modes, and integration points
- Non-obvious implications of design decisions that remain after source inspection
- "What if" scenarios that are not already answered by existing code or tests
- Performance, scalability, and maintenance concerns
- Assumptions that might be hiding complexity
- User mental models and expectations
- Error states and recovery paths
- Security implications and attack surfaces
- Data lifecycle and cleanup
- UI/UX specifics and user flows
- Tradeoffs between approaches

## Interview Loop

1. Inspect available source evidence for likely answers before forming the next question round
2. Ask 1-4 probing questions at a time using AskUserQuestion
3. Incorporate answers into your understanding
4. If refining an existing file such as `spec.md`, update that file with the new answers and clarified decisions before asking the next question round
5. If the working file is a spec file (`spec.md` or a file the user identifies as a spec), add or update its YAML front matter with `status: refined` once the first refinement update is written
6. When a follow-up, idea, risk, or adjacent task is worth preserving but should not block the current refinement, add it to `next.md` beside the working plan/spec instead of expanding the active scope
7. Repeat until you have comprehensive understanding
8. Do NOT stop interviewing prematurely - thorough exploration is important

## Spec Status Marker

When refining a spec file, mark it as refined so later agents can tell the refinement pass has already touched it.

- If the spec already has YAML front matter, preserve existing fields and set `status: refined`
- If the spec does not have YAML front matter, add it at the top with only `status: refined`
- Do not add the status marker to `next.md` or deferred follow-up files

## Deferred Follow-Ups

Create or update `next.md` when useful to capture deferred work that should survive the refinement session without becoming part of the active plan. Use it for follow-up questions, later investigation, adjacent feature ideas, cleanup tasks, risks to revisit, or decisions intentionally postponed.

Keep `next.md` concise and actionable:

- Include enough context for the item to make sense later
- Group related follow-ups under short headings when helpful
- Mark items as deferred, blocked, or candidate follow-ups rather than treating them as accepted requirements
- Do not move an item from `next.md` into the working plan/spec unless the user confirms it belongs in the current scope

## Important

- When an existing file is the working plan/spec, keep it current after every question/answer round
- Do not ask the user to speculate about behavior that can be checked in the current repo or an inspectable external source
- Use `next.md` for deferred follow-ups so the current plan/spec does not accumulate unresolved side quests
- Just conduct the interview and provide context
- Let the caller decide what to do with the gathered information
