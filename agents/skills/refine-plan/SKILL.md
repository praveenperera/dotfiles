---
name: refine-plan
description: Deep interview for gathering requirements to refine the current plan (user)
---

# Refine Plan

Conduct an in-depth interview to gather requirements using the AskUserQuestion tool.

## Topics to Cover

- Technical implementation details
- UI & UX considerations
- Concerns and edge cases
- Tradeoffs and alternatives

## Question Guidelines

Ask non-obvious, probing questions - not surface-level questions:

- Edge cases, failure modes, and integration points
- Non-obvious implications of design decisions
- "What if" scenarios
- Performance, scalability, and maintenance concerns
- Assumptions that might be hiding complexity
- User mental models and expectations
- Error states and recovery paths
- Security implications and attack surfaces
- Data lifecycle and cleanup
- UI/UX specifics and user flows
- Tradeoffs between approaches

## Interview Loop

1. Ask 1-4 probing questions at a time using AskUserQuestion
2. Incorporate answers into your understanding
3. If refining an existing file such as `spec.md`, update that file with the new answers and clarified decisions before asking the next question round
4. When a follow-up, idea, risk, or adjacent task is worth preserving but should not block the current refinement, add it to `next.md` beside the working plan/spec instead of expanding the active scope
5. Repeat until you have comprehensive understanding
6. Do NOT stop interviewing prematurely - thorough exploration is important

## Deferred Follow-Ups

Create or update `next.md` when useful to capture deferred work that should survive the refinement session without becoming part of the active plan. Use it for follow-up questions, later investigation, adjacent feature ideas, cleanup tasks, risks to revisit, or decisions intentionally postponed.

Keep `next.md` concise and actionable:

- Include enough context for the item to make sense later
- Group related follow-ups under short headings when helpful
- Mark items as deferred, blocked, or candidate follow-ups rather than treating them as accepted requirements
- Do not move an item from `next.md` into the working plan/spec unless the user confirms it belongs in the current scope

## Important

- When an existing file is the working plan/spec, keep it current after every question/answer round
- Use `next.md` for deferred follow-ups so the current plan/spec does not accumulate unresolved side quests
- Just conduct the interview and provide context
- Let the caller decide what to do with the gathered information
