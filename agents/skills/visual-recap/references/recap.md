# Mechanically grounded recap construction

Build the recap from a diff, not toward one. Every section must stay
traceable to changed lines and the resulting source.

## Scope the whole work unit

By default, recap the whole current thread or requested PR, branch,
commit, or diff: initial implementation, later fixes, UI follow-ups,
tests, migrations, changesets, skill or instruction updates, and
generated artifacts. Use the diff plus conversation or repository
context to exclude unrelated pre-existing dirty work. If scope cannot
be inferred, state the assumption or ask one concise question.

When revising an existing recap, keep whole-work-unit coverage and add
the correction. Do not replace it with a recap of only the latest
feedback unless the user explicitly narrows the scope.

Skip the recap for a tiny, obvious change that reviews faster as a raw
diff.

## Grounding and security

- Build file trees, contracts, UI states, and diffs from real paths,
  fields, methods, routes, labels, permissions, and before/after text.
  Leave absent facts out. Mark prose conclusions that extend beyond
  the diff as inferred.
- Redact API keys, tokens, webhook URLs, signing secrets, `.env`
  values, and credential-looking literals everywhere, including code
  excerpts and file-tree notes. Use placeholders such as `<redacted>`.

## Inventory before authoring

List the meaningful changed surfaces and ensure the final recap
represents each or intentionally omits it as tiny or redundant:

- routes, components, dialogs, popovers, sheets, navigation, and
  shared UI
- entry, interaction, destination, empty, loading, error, and saved
  states
- owner, admin, editor, viewer, denied, public, and private access
  variants
- entities, migrations, API/actions, wire formats, compatibility, and
  lifecycle
- architectural boundaries, files, and load-bearing code hunks

UI-impact changes require mockups. A flow must show entry, the changed
interaction, and the result. Prefer one mockup with a compact state
switch over stacked copies of the same surface. Use before/after only
when direct comparison adds value. Use the smallest real surface for
popovers, panels, dialogs, or routes. Ground labels and chrome in the
changed product. Mark pixel-level visuals as inferred when they were
reconstructed rather than captured.

## Canonical shape

Order the recap as follows when each part applies:

1. UI-impact mockups or the primary structural diagram
2. at most a short lede for an objective, decision, or risk the
   visual cannot carry; omit it when the headline visual is enough
3. changed data models, endpoints, or architecture
4. a file tree with a change flag for every included file
5. a `Key changes` heading with three to eight focused diffs or
   annotated new files

Keep excerpts focused, preferably under about 150 lines each. Use a
title under about 70 characters. Do not add a summary card row, score,
or second telling of what the mockup, diagram, tree, or diff already
shows.

## Diff-to-section mapping

- Schema or migration change: a table of entities and fields with
  `added`, `removed`, or `changed`, plus `was` for changed types.
  Include literal SQL only when it matters.
- API, action, or route change: method, path, params, request, and
  responses. Mark root and nested changes. Use one valid JSON value
  per example and separate distinct message shapes.
- Compatibility-sensitive behavior: a short risk note beside its
  contract and the literal hunk when useful.
- Meaningful before/after code: a split diff with a one-line summary
  and a few high-signal annotations. Use unified form only for a
  narrow hunk where split view harms legibility.
- A new file or large addition without meaningful before text:
  annotated code, not a one-sided split diff.
- File footprint: a tree with `added`, `removed`, `modified`, or
  `renamed` flags and concise notes.
- Rendered UI or interaction: grounded mockup states before the
  implementation evidence. Cover role and permission differences.
- Architecture or data-flow change: Mermaid or handwritten SVG.
  Prefer layered, swimlane, matrix, or before/after layouts over a
  default linear chain. Never use a diagram as a substitute for
  rendered UI.

## Feedback

Feedback may refine recap coverage, wording, visuals, and states.
Update only the recap unless the user separately asks to implement a
source change. Preserve the whole-work-unit view while incorporating
corrections.
