---
name: architect
description: Design durable domain state, ownership, interfaces, or module boundaries before a non-trivial change. Use for API or schema changes, migrations, competing designs, or repeated caller patches. Do not use for local UI state, sorting, styling, one-component work, or established patterns.
---

# Architect

Design the shape that makes the implementation direct. Keep architecture and integration with the primary agent.

## Confirm the need

Use this workflow only when the decision affects a durable domain boundary or when choosing the wrong owner would create migration or compatibility cost. Ordinary feature work can still use good types and ownership without invoking this skill.

Do not invoke `how` only because this skill is active. Use `how` separately only when the user asks for a system explanation or the execution path is independently complex enough to need a full trace.

## Ground the design

Trace only the callers and boundaries that constrain the decision. Inspect relevant history when the current ownership or constraint may be intentional. Check the applicable tests, schemas, wire formats, persistence, and error boundaries.

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
