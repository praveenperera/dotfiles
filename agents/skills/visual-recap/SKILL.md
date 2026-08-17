---
name: visual-recap
description: >-
  Turn a PR, branch, commit, git diff, or completed work unit into a
  standalone HTML recap with grounded diagrams, file maps, API/schema
  summaries, UI mockups, annotated diffs, and focused review notes. Use
  when asked to recap completed work, visualize a PR or diff, produce a
  visual recap, or run /visual-recap.
---

# Visual Recap

Build a structured recap from completed work. The deliverable is one
standalone HTML file the user can open in any browser.

Use the composition bar of the Codex visualize skill, not its host
contract. The recap file is the explanation. Follow Composition in
the HTML reference.

Do not write Planport MDX. Do not emit Codex visualize fragments or
`visualize` content references. Do not paste the recap into chat.

## Boundary

Inspect the implementation and history. Write only the recap HTML.
Review comments do not authorize source changes.

## Workflow

1. Establish the whole work unit: implementation, later fixes, tests,
   migrations, generated artifacts, and instruction changes. Separate
   those from unrelated dirty work.
2. Derive every claim from the actual diff and resulting source. Never
   invent paths, fields, routes, code, UI labels, or before/after
   states.
3. Inventory changed UI surfaces and access states, schema/API
   contracts, architecture, files, and load-bearing hunks.
4. Read [recap construction](references/recap.md) and
   [standalone HTML](references/html.md). Inline
   [assets/recap.css](assets/recap.css) into the document.
5. Write `_scratch/visual-recap/<slug>.html`, creating the directory
   when needed.
6. Open the file when a browser is available and verify it against
   the HTML reference. Apply feedback only to the recap unless the
   user separately asks to change source.

Redact secrets and credential-looking literals from every section,
excerpt, note, and caption.

## Return

Report the absolute path and the work unit. Add at most one short
sentence the visual cannot say.
