---
name: architect
description: Settle the domain model, ownership, caller usage, public interfaces, and module boundaries before a non-trivial implementation. Use when a change crosses boundaries, introduces state, exposes repeated conditionals, or risks locking in the wrong abstraction. Do not use for a small mechanical edit with an established shape.
---

# Architect

Design the shape that makes the implementation direct. Keep architecture and integration with the primary agent.

## Ground the design

Trace the existing system with `how`. Use `why` when the current ownership or constraint may be intentional. Inspect callers, tests, schemas, wire formats, persistence, and error boundaries that constrain the design.

State the user-visible behavior and the invariants before choosing types.

## Sketch from the caller inward

Write the intended caller usage first. Then define:

- domain states and transitions
- types that exclude invalid combinations
- the owner of each state and operation
- public signatures and error contracts
- module boundaries and dependency direction
- boundary parsing and validation
- migration and deletion of replaced APIs
- verification points

Keep sketches in the response or under the repository `_scratch/` directory. Do not add production stubs or `not implemented` bodies unless the user explicitly asks for source scaffolding.

When the decision has no clear precedent and materially different shapes are viable, compare two or three complete alternatives. Do not run a multi-model panel unless the user asks for one or an applicable instruction requires it.

## Continue or stop

If the user asked for implementation, use the selected sketch as the contract and continue. Revisit the design when implementation repeatedly needs the same escape hatch, caller-specific branch, cast, optional field, or ownership workaround.

If the user asked only for architecture, return the selected shape, rejected alternatives, remaining risks, and the evidence that would change the decision.
