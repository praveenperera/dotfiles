---
name: visual-recap
description: >-
  Turn a PR, branch, commit, git diff, or completed work unit into a local
  interactive Planport recap with mechanically grounded diagrams, file maps,
  API/schema summaries, UI states, annotated diffs, and focused review notes.
metadata:
  visibility: exported
---

# Visual Recap

Build a structured recap from completed work. The deliverable is the Planport
artifact and review URL, not an inline summary.

## Boundary

Inspect the implementation and history, but update only recap MDX/state and
feedback artifacts. Review comments do not authorize implementation changes.
Change source only when the user separately requests that work.

## Workflow

1. Establish the whole work unit: gather its implementation, follow-up fixes,
   tests, migrations, generated artifacts, and instruction changes. Separate
   those changes from unrelated dirty work.
2. Derive the recap mechanically from the actual diff and resulting source.
   Never invent paths, fields, routes, code, UI labels, or before/after states.
3. Inventory changed UI surfaces and access states, schema/API contracts,
   architecture, files, and load-bearing hunks before authoring.
4. Read the references required by the routing below, then create a substantial
   but lean recap with the visual or structural headline first, a file tree, and
   focused key-change diffs or annotated code.
5. Serve and verify the recap through Planport. For UI-impact work, visually
   inspect the rendered wireframes when a browser is available.
6. Apply feedback surgically to the recap artifact and keep coverage of the
   whole work unit unless the user explicitly narrows scope.

Redact secrets and credential-looking literals from every block, excerpt, note,
and caption. Treat the tokenized LAN URL and recap contents with the same
visibility care as the source they summarize.

## Reference Routing

- Always read the shared [Planport workflow](../visual-plan/references/planport.md)
  before creating, serving, or updating a recap.
- Read [recap construction](references/recap.md) for whole-work-unit grounding,
  canonical shape, diff mapping, UI coverage, security, and budgets.
- Read the shared [block components](../visual-plan/references/blocks.md) before
  authoring or changing structured MDX blocks.
- Read the shared [wireframe quality](../visual-plan/references/wireframe.md)
  only when the diff has rendered UI impact and a wireframe is required.

Do not author block syntax or wireframes from memory when the relevant reference
applies.
