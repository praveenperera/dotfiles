---
name: technical-writing
description: Draft or materially restructure RFCs, ADRs, READMEs, runbooks, pull request descriptions, or similar technical documents when reader tasks or structure need deliberate design. Do not use for small factual, typo, label, or isolated paragraph edits, product marketing, or creative writing.
---

# Technical writing

Write for the engineer who must understand or act without asking the author for help.

## Set the document contract

Identify the reader, the decision or task, the required facts, the intended action, and the source of truth. Preserve exact symbols, commands, paths, numbers, constraints, and uncertainty.

For a new document or material restructure, read [document-modes.md](references/document-modes.md) before choosing structure. For a focused revision, keep the existing mode and structure without loading that reference. Use one primary mode per document. Split and link when the material serves different modes.

## Draft from evidence

Inspect the relevant code, configuration, tests, and existing documents. Do not invent behavior, commands, ownership, recovery steps, or compatibility claims.

Use the codebase's exact terms. Keep one name for each thing. Put conditions before instructions. State the actor when it matters. Use numbered lists for sequences and bullets for unordered sets.

Apply repository writing rules, including ASD-STE100 when required. Keep necessary technical terms even when a general prose rule would replace them.

Use `deslop-writing` only when the user asks for voice or AI-pattern cleanup, or when a material prose problem remains after the technical content is correct. Do not load it for routine technical documents or factual updates.

## Verify the artifact

Check every command, path, symbol, count, and link that can be checked. Confirm that instructions work in their stated order. Verify that warnings appear before the step that creates the risk.

Return only the requested document by default. Put edit notes, source notes, or a change summary outside public-facing copy and only when the user asks for them.
