---
name: how
description: Trace and explain how a multi-step subsystem or cross-boundary data flow works, from entry point to observable result. Use when the user invokes $how or /how for a system walkthrough, ownership, placement, or onboarding question. Use `why` for rationale.
disable-model-invocation: true
---

# How

Explain the real execution path from an entry point to an observable result.

## Size the question

If the scope is unclear, state your reading and proceed; the user can redirect.

- **Narrow:** one module, function, or flow. Trace it yourself in one pass.
- **Broad:** a subsystem that crosses several files, services, or languages. Split it into two to four distinct angles, such as the request path, state ownership, and persistence. Send each angle to a read-only GPT-6.1 Sol `high` explorer routed through the `use-agents` skill, then synthesize their findings in the root. Each explorer returns file and symbol citations, the path it traced, and what it could not confirm.

When in doubt, take the narrow path.

## Trace the system

Start from the user action, public API, event, command, or failing symptom. Follow the path through:

- entry points and callers
- domain types and state ownership
- important transformations and side effects
- persistence, network, process, or language boundaries
- the returned value or visible result

Inspect pinned dependency source or documentation when behavior depends on an unfamiliar library. Use `btx` or the local registry; do not infer library behavior from its name or call site.

Distinguish the normal path from error, retry, cancellation, and cleanup paths when they affect the question. Name hidden coupling and misplaced ownership when the trace exposes it.

## Explain for the question

Lead with a plain definition and the shortest complete flow. Then cover only the sections that help the user change, review, or debug the system:

- **Overview:** what the subsystem is and what it is for
- **Key concepts:** the few types or ideas everything else depends on, one name each
- **How it works:** the traced path, step by step
- **Where things live:** files and symbols for each step
- **Gotchas:** non-obvious coupling, ordering, or failure behavior

Use a diagram only when three or more components or state transitions are easier to understand visually. Build it around the traced path, not the file tree.

## Evidence

Cite repository files and symbols. State what you ran or inspected. Separate confirmed behavior from inference and list any path you could not verify.

This is a read-only skill. Do not change the code unless the user also asks for a change.
