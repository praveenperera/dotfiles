# Mechanically grounded recap construction

Build the recap from a diff, not toward one. Structured blocks summarize work
that exists and must remain traceable to actual changed lines and resulting
source.

## Contents

- Scope the whole work unit
- Grounding, security, and inventory
- Canonical shape and budgets
- Diff-to-block mapping
- Feedback boundaries

## Scope the whole work unit

By default, recap the whole current thread or requested PR, branch, commit, or
diff: initial implementation, later fixes, UI follow-ups, tests, migrations,
changesets, skill or instruction updates, and generated artifacts. Use the diff
plus conversation or repository context to exclude unrelated pre-existing dirty
work. If scope cannot be inferred, state the assumption or ask one concise
question.

When revising an existing recap, retain its whole-work-unit coverage and add the
correction. Do not replace it with a recap of only the latest feedback unless
the user explicitly narrows the scope.

## Grounding and security

- Build `Diff`, `DataModel`, `Endpoint`, `FileTree`, and UI states mechanically
  from real paths, fields, methods, routes, labels, permissions, and before/after
  text. Leave absent facts out. Mark prose conclusions that extend beyond the
  diff as inferred.
- Redact API keys, tokens, webhook URLs, signing secrets, `.env` values, and
  credential-looking literals everywhere, including code excerpts and file-tree
  snippets. Use obvious placeholders such as `<redacted>`.
- Keep tokenized Planport links scoped to the intended local reviewers. Do not
  place them in public issues or PR comments without explicit direction.

## Inventory before authoring

List the meaningful changed surfaces and ensure the final recap represents each
or intentionally omits it as tiny or redundant:

- routes, components, dialogs, popovers, sheets, navigation, and shared UI
- entry, interaction, destination, empty, loading, error, and saved states
- owner, admin, editor, viewer, denied, public, and private access variants
- entities, migrations, API/actions, wire formats, compatibility, and lifecycle
- architectural boundaries, files, and load-bearing code hunks

UI-impact changes require wireframes. Show the entry point, the changed
interaction, and the resulting state for a flow; a single entry-surface mockup
is insufficient. Use before/after only when direct comparison adds value, and
use the smallest real surface for popovers, panels, dialogs, or routes. Ground
labels and chrome in the changed product. Mark pixel-level visuals as inferred
when they were reconstructed rather than captured.

## Canonical shape and budgets

Order the recap as follows when each part applies:

1. UI-impact wireframes or the primary structural headline
2. one to three paragraphs explaining what changed and why
3. changed data models, endpoints, or architecture
4. a `FileTree` with a `change` flag for every included file
5. `## Key changes` followed by one horizontal `TabsBlock` of focused `Diff` or
   `AnnotatedCode` blocks

Use three to eight key-change tabs for substantial work. Keep excerpts focused,
preferably under about 150 lines per tab. Use a title under about 70 characters
and a one-to-three-sentence brief. Skip the recap itself for a tiny, obvious
change that reviews faster as a raw diff; do not omit the file tree or key-change
evidence from work substantial enough to recap.

Keep prose lean. Do not add generic provenance, disclaimers, file counts, or
instructions to review the diff. Add prose only for the objective, a decision,
compatibility risk, or grounded review note that structured blocks do not carry.

## Diff-to-block mapping

- Schema or migration change: use `DataModel` with entity/field `change` values
  and `was` for changed types. Include literal SQL only when it matters.
- API, action, or route change: use `Endpoint` with the resulting method, path,
  params, request, and responses. Mark root and nested changes. Use one valid
  JSON value per example and separate distinct message shapes.
- Compatibility-sensitive behavior: place a short `RichText` risk note beside
  its contract block and include the literal hunk when useful.
- Meaningful before/after code: use `Diff` in split mode with a one-line summary
  and a few high-signal annotations. Use unified mode only for a narrow hunk
  where split view harms legibility.
- A new file or large addition without meaningful before text: use
  `AnnotatedCode`, not a one-sided split diff.
- File footprint: use `FileTree` with `added`, `removed`, `modified`, or
  `renamed` flags and concise notes.
- Rendered UI or interaction: use grounded `WireframeBlock` states before the
  implementation evidence. Cover role and permission differences.
- Architecture or data-flow change: use `Diagram` or `Mermaid`. Prefer layered,
  swimlane, matrix, or before/after layouts over a default linear chain. Never
  use a diagram as a substitute for rendered UI.

For structured before/after comparisons, use `Columns` labeled `Before` and
`After`. For code, use `Diff`. For UI, use wireframes. Do not recreate these
layouts with `CustomHtml`.

Group multiple key files in one horizontal `TabsBlock` under a separate
`RichText` `## Key changes` heading. Use the file path or a short unambiguous
label for each tab. Give every block a unique `id`; use only canonical component
shapes from the shared block reference.

## Feedback

Feedback may refine recap coverage, wording, blocks, and visual states. Update
only the recap artifact unless the user separately asks to implement a source
change. Preserve the whole-work-unit view while incorporating corrections.
