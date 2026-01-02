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
3. Repeat until you have comprehensive understanding
4. Do NOT stop interviewing prematurely - thorough exploration is important

## Important

- Do NOT create any spec files or other files
- Just conduct the interview and provide context
- Let the caller decide what to do with the gathered information
