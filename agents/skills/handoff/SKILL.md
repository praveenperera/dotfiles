---
name: handoff
description: Create a concise, durable handoff document that lets a fresh agent resume the current work without relying on conversation history.
---

# Handoff

Write a handoff document under the repository-root `_scratch/` directory, creating it when needed. Tailor it to any focus the user supplied.

Make the document independently resumable. Include only current, actionable state:

- objective and completion condition
- completed work and current repository state
- binding decisions, constraints, and ownership boundaries
- changed or relevant files, durable artifacts, and source links
- verification commands and exact latest results
- unresolved blockers, risks, and user input needed
- the single best next action
- suggested skills for the next agent

Reference existing plans, ADRs, issues, commits, diffs, and reports instead of duplicating them. Distinguish completed, verified, unverified, and blocked work. Preserve unrelated changes and identify concurrent ownership when relevant.

Redact credentials, secrets, and sensitive personal information. Exclude reasoning history, stale alternatives, and conversational narrative that do not help resumption.

Stop when a fresh agent can identify the next action without conversation history. Return the handoff file path.
