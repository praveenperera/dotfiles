---
description: Interview-driven spec writing with planning workflow
argument-hint: [vague idea or feature name]
---

You are helping the user turn a vague idea into a comprehensive spec through in-depth interviewing.

Feature idea: **$ARGUMENTS**

**Phase 1: Get the idea**

If the line above shows "Feature idea: \*\*\*\*" (empty), ask the user what they want to build and wait for their response.

**Phase 2: Research the codebase**

Before interviewing, research the codebase to understand:
- Relevant files, patterns, and existing implementations
- Dependencies, constraints, or related functionality
- Use WebSearch if needed for external context

**Phase 3: Interview (CRITICAL)**

Interview the user in detail using the AskUserQuestion tool about literally anything: technical implementation, UI & UX, concerns, tradeoffs, etc. Make sure the questions are NOT obvious.

Be very in-depth and continue interviewing continually until it's complete.

**Question Guidelines:**
- Don't ask surface-level questions - probe deeper
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

**Interview Loop:**
1. Ask 1-4 probing questions at a time using AskUserQuestion
2. Incorporate answers into your understanding
3. Repeat until you have comprehensive understanding
4. Do NOT stop interviewing prematurely - a good spec requires thorough exploration

**Phase 4: Create the spec files**

Once the interview is complete, derive a short kebab-case feature name and create:

```
temp_docs/<feature-name>/
├── spec.md        # The comprehensive spec
├── progress.md    # Task tracking with checkboxes
└── context.md     # Learnings, blockers, questions
```

**spec.md contents:**
- Overview and goals
- Detailed requirements from the interview
- Technical approach
- Edge cases and error handling
- UI/UX specifics (if applicable)
- Open questions or future considerations

**progress.md contents:**
- Checkbox list of implementation tasks
- All marked as pending initially

**context.md contents:**
- Important learnings from codebase research
- Known blockers or challenges
- Any remaining questions

**Phase 5: Ongoing maintenance**

Throughout the conversation, automatically maintain these files:
- Update `progress.md` as tasks are started and completed
- Add new learnings, blockers, surprising findings to `context.md`
- Update `spec.md` if requirements change
- Do this proactively without being asked
