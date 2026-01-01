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

**Create the spec files:**

1. Derive the project ID from the current git repo name: `basename $(git rev-parse --show-toplevel) | sed 's/-wk.*$//'`
2. Derive a short kebab-case feature name from the topic being explored
3. Create the spec file at `~/.claude/plans/<project-id>/<feature-name>/spec.md`
