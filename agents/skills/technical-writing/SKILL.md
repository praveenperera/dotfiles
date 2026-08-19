---
name: technical-writing
description: Draft or revise technical documentation, RFCs, ADRs, READMEs, runbooks, pull request descriptions, and similar engineering prose. Use when document purpose, reader tasks, precise terminology, or information structure matter. Do not use for product marketing or general creative writing.
---

# Technical writing

Write for the engineer who must understand or act without asking the author for help.

## Set the document contract

Identify the reader, the decision or task, the required facts, the intended action, and the source of truth. Preserve exact symbols, commands, paths, numbers, constraints, and uncertainty.

Read [document-modes.md](references/document-modes.md) before choosing structure. Use one primary mode per document. Split and link when the material serves different modes.

## Draft from evidence

Inspect the relevant code, configuration, tests, and existing documents. Do not invent behavior, commands, ownership, recovery steps, or compatibility claims.

Use the codebase's exact terms. Keep one name for each thing. Put conditions before instructions. State the actor when it matters. Use numbered lists for sequences and bullets for unordered sets.

Apply repository writing rules, including ASD-STE100 when required. Apply `deslop-writing` after the content is correct when that skill is available. Keep necessary technical terms even when a general prose rule would replace them.

## Verify the artifact

Check every command, path, symbol, count, and link that can be checked. Confirm that instructions work in their stated order. Verify that warnings appear before the step that creates the risk.

Return only the requested document by default. Put edit notes, source notes, or a change summary outside public-facing copy and only when the user asks for them.
