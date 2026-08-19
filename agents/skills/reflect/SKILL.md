---
name: reflect
description: Review completed or difficult work, identify durable lessons, and propose the smallest skill, instruction, test, lint, or tooling changes that prevent repeated mistakes. Use when the user asks what was learned, why work needed repeated loops, how to avoid a recurrence, or to reflect on a task.
---

# Reflect

Find lessons that will change future work. Do not turn every correction into a permanent rule.

## Scope the review

Use the current conversation and its artifacts first. If context was compacted or the user names earlier work, inspect only the relevant local Codex sessions and repository history. Keep unrelated projects and conversations out of scope.

Reconstruct:

- the intended result
- the approaches tried
- the evidence that caused each change of direction
- repeated corrections or failures
- the final verification state

## Classify each lesson

Accept a lesson only when it is reusable and supported by the record.

- **Structural:** A type, API, test, lint, script, or runtime check can prevent the failure. Prefer this route.
- **Workflow:** An existing skill or repository instruction needs a precise change.
- **Local:** The lesson belongs in project documentation or a project-local skill.
- **One-off:** The event does not justify a durable change.

Repeated symptoms with one cause produce one lesson. Do not preserve a workaround when the record shows that the model or abstraction was wrong.

## Propose before changing policy

Unless the user already asked to apply the findings, present a compact proposal and wait before editing global skills or instructions. Repository-local tests, lints, and scripts still require the authority of the active task.

Use `skill-creator` for substantive skill changes. Preserve existing skill scope and remove stale guidance that the new lesson replaces.

## Return

Report:

- **Keep:** lessons worth encoding, with evidence and destination
- **Drop:** tempting conclusions that were one-off, unsupported, or already covered
- **Next:** the smallest approved change that prevents the highest-cost repeat

If the user asked for changes, apply them, validate the affected artifacts, and report the exact files and checks.
