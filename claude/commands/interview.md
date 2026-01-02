---
description: Deep interview for gathering requirements
argument-hint: [topic or feature]
---

Interview me in detail using the AskUserQuestionTool about $ARGUMENTS covering:

- Technical implementation details
- UI & UX considerations
- Concerns and edge cases
- Tradeoffs and alternatives

Ask non-obvious, in-depth questions. Continue interviewing until the topic is fully explored.

After the interview is complete use the AskUserQuestion
to ask if I want the feedback incorporated into the plan if we are in plan mode or to create a spec.md file.

If the user asks to create a spec.md file do the following:

1. Derive a short kebab-case feature name from the topic being explored
2. Create the spec file at `./_cl_plans/<feature-name>/spec.md` (in the root of the project)
3. Add this note at the end of spec.md:

```
---

## Implementation Notes

When starting implementation, create these companion files in this directory:

- `progress.md` - Checkbox list of implementation tasks (all pending initially)
- `context.md` - Important learnings, known blockers, remaining questions

Update these files proactively as you work.
```
