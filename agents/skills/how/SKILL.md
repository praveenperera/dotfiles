---
name: how
description: Trace and explain how a code subsystem, feature, state transition, or data flow works. Use for code walkthroughs, ownership and placement questions, runtime-flow explanations, and onboarding to an unfamiliar area. Use `why` when the question is about design motivation or historical rationale.
---

# How

Explain the real execution path from an entry point to an observable result.

## Trace the system

Start from the user action, public API, event, command, or failing symptom. Follow the path through:

- entry points and callers
- domain types and state ownership
- important transformations and side effects
- persistence, network, process, or language boundaries
- the returned value or visible result

Inspect pinned dependency source or documentation when behavior depends on an unfamiliar library. Do not infer library behavior from its name or call site.

Distinguish the normal path from error, retry, cancellation, and cleanup paths when they affect the question. Name hidden coupling and misplaced ownership when the trace exposes it.

## Explain for the question

Lead with a plain definition and the shortest complete flow. Add detail only where it helps the user change, review, or debug the system. Use one name for each concept.

Use a diagram only when three or more components or state transitions are easier to understand visually. Build the diagram around the traced path, not the file tree.

## Evidence

Cite repository files and symbols. State what you ran or inspected. Separate confirmed behavior from an inference and list any path that could not be verified.

This is a read-only skill. Do not change the code unless the user also asks for a change.
