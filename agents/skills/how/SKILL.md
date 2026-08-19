---
name: how
description: Trace and explain a non-obvious multi-step subsystem or cross-boundary data flow when the user asks how it works. Use for system walkthroughs, ownership, placement, and onboarding. Do not use for implementation, local bugs, or answers found in one symbol, rule, or component. Use `why` for rationale.
---

# How

Explain the real execution path from an entry point to an observable result.

## Confirm the need

Use a focused source search and answer directly when one symbol, configuration value, style rule, or component explains the behavior. Load this workflow only when the answer requires a path across several meaningful steps or owners.

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
