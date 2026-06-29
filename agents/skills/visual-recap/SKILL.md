---
name: visual-recap
description: >-
  Turn a PR, branch, commit, or git diff into an interactive visual recap with
  diagrams, file maps, API/schema summaries, annotated diffs, and focused review
  notes.
metadata:
  visibility: exported
---

## Installed Mode

Default storage for this installation: local files reviewed through Planport.
Create and update plans and recaps as MDX folders under `plans/<slug>/` when
they should be checked in, or under a repo-ignored/temp folder when they should
stay private scratch. Before authoring structured MDX, read
`../visual-plan/references/blocks.md` plus the bundled wireframe and recap
rules. Then run
`env -u PORT planport serve plans/<slug> --open` and report the printed LAN URL.
Planport serves the review UI and the local MDX files from this machine, binds
to the local network, and chooses a random available TCP port by default so many
local recaps can run at once. Use `PORT` or `--port` only when the user
explicitly asks for a fixed port. It writes review feedback to
`plans/<slug>/comments.json`. The printed URL includes a per-run token; do not
commit tokenized URLs. If `planport` is not installed, run `cmd release
planport` first. No sharing, all local.


# Visual Recap

`/visual-recap` creates a visual plan built **from** a diff, not toward one. It
is the reverse of forward planning: instead of describing the change you are
about to make, you describe the change that was just made, at a higher altitude
than line-by-line review. The same plan data model serves both directions —
schema, API, file, and architecture changes become the same `data-model`,
`api-endpoint`, `file-tree`, and `diagram` blocks a forward plan would use, only
now they summarize work that exists. A reviewer scans the shape of the change
before spending attention on the literal lines.

## Planport Local Mode

Planport local mode is the default for this installation. Use it whenever the
user needs a recap. It provides fully local files, LAN access, and
repo-owned/source-controlled recap artifacts.

In Planport local mode:

- Read the diff/stat/source context from local files and shell commands only.
  Use `git diff`, `git show`, `git status`, `rg`, and focused file reads to
  collect the recap evidence locally.
- Read `../visual-plan/references/blocks.md` and the relevant bundled
  references before writing structured MDX. For `checklist` and
  `question-form`, follow the required shapes exactly: checklist items need
  `id` and `label`; question-form questions need `id`, `title`, and `mode`; and
  each option needs `id` and `label`.
- Write the recap as a local MDX folder: use `plans/<slug>/` when the user
  wants the artifact checked into the repo, or use a repo-ignored/temporary
  folder such as `_scratch/plans/<slug>/` or `/tmp/planport-plans/<slug>/` when
  it should not be checked in. The folder contains `plan.mdx`, optional
  `canvas.mdx`, optional `prototype.mdx`, and optional `.plan-state.json`. Set
  `kind: "recap"` and `localOnly: true` in frontmatter/state when authoring
  the source.
- Run `env -u PORT planport serve plans/<slug> --open`. Report the printed LAN
  URL. The URL includes a per-run token and should not be
  committed. Planport binds to the LAN (`0.0.0.0`), chooses a random available
  TCP port when `PORT` is unset and no `--port` is passed, and writes review
  feedback to `comments.json` beside `plan.mdx`. Use `PORT` or `--port` only
  when the user explicitly asks for a fixed port. If `planport` is missing, run
  `cmd release planport`.
- Planport is the default workflow for this installation.
- For headless verification, fetch the Planport API using the printed token:
  `curl '<lan-or-local-url>/api/plan?token=<token>'`. Confirm the response
  includes the expected title/files. If the browser cannot load the recap, use
  this endpoint to read the concrete server error.
- Treat review feedback as file or chat feedback: read `comments.json` or the
  user's pasted Planport `Copy` payload, update the MDX files directly, and keep
  serving the same plan folder with Planport.

Planport local mode keeps recap content local. It does not by itself make the
coding agent's language model local; for that stronger
privacy boundary, the host agent/model must also be local or otherwise approved
by the user.

## Always Create A Structured Local Recap — Never Inline

The deliverable is ALWAYS a structured local Planport recap. NEVER hand the
recap to the user as inline chat content — not Markdown prose, not an ASCII
sketch, not a table, not a fenced "wireframe", not a "here's the recap" summary.
A recap's value is the interactive, annotatable plan; an inline summary is not a
recap, it is the thing a recap replaces. Planport is the only collaboration
surface.

## When To Use

Build a recap when a PR or commit is large, multi-file, or touches schema, API
contracts, or architecture, and a reviewer would benefit from seeing the change
mapped to structured blocks before reading the raw diff. A GitHub Action can
generate one automatically from a PR diff; an agent can generate one on request
("recap this PR", "show me what this branch changed"). Skip it for small,
single-file, or obvious diffs — a recap is review overhead, and a tiny change
reviews faster as plain diff.

## Recap The Whole Work Unit

When `/visual-recap` is invoked in a chat thread after work has already happened,
the default scope is the whole current work unit/thread, not only the most recent
user message, tool action, or follow-up fix. Gather the thread-owned changes
across the conversation: original implementation work, later bug fixes, UI
follow-ups, tests, changesets, skill/instruction updates, generated plan/source
artifacts, and any local import/linking fixes needed to make the recap open.

Use the current diff plus conversation context to separate thread-owned changes
from unrelated dirty work that existed before the thread. Exclude unrelated
pre-existing edits. If the scope is genuinely ambiguous and cannot be inferred,
state the assumption or ask a concise question before handoff.

When updating an existing recap after feedback, revise the recap so it still
covers the whole thread/work unit plus the new correction. Do not replace a broad
recap with a narrow recap of only the latest feedback unless the user explicitly
asks for that narrower scope.

## Keep The Recap Body Lean

Do not add boilerplate intro, disclaimer, provenance, or summary prose blocks to
the generated plan body. In particular, do not create a `rich-text` block just to
say the recap is an aid, that the reviewer should still review the diff, how many
files changed, or which ref/working tree generated the recap. The plan title,
brief, and `file-tree` (which carries the per-file change stats) already carry
that context.

Only add prose blocks when they tell the reviewer something specific about the
change that the structured blocks do not: the objective, a real compatibility
risk, an important decision visible in the diff, or a grounded review note.

## Recaps Must Be Substantial

Lean is not the same as thin. A recap is not a single wireframe plus one
sentence — that under-serves the reviewer as much as boilerplate prose over-serves
them. Alongside the visual/structural headline (wireframes, `data-model`,
`api-endpoint`, `diagram`), a substantial recap also carries the implementation
evidence:

- A short surface/state inventory before authoring: list the changed routes,
  components, popovers/dialogs, role/access states, empty/error states, and
  shared abstractions visible in the diff. The final recap must either represent
  each meaningful item with a block or intentionally omit it because it is tiny,
  redundant, or not user-visible.
- A `file-tree` of the changed files with each entry's `change` flag, so the
  reviewer sees the footprint of the work at a glance.
- The split `diff` of the KEY changed files, grouped under a `## Key changes`
  `rich-text` heading in a single horizontal `tabs` block (the default
  orientation, one file per tab), with a one-line `summary` and a few
  `annotations` on each — so the reviewer can drop from the high-altitude shape
  straight into the load-bearing code. Use horizontal file tabs, not a vertical
  side rail, so the selected file has enough width for the side-by-side diff.

Skip the diff appendix only for a genuinely tiny change that reviews faster as
plain diff (see "When To Use"); for any change worth recapping, the file-tree and
key-change diffs belong in the plan.

## Canonical Shape And Budgets

A strong recap follows one skeleton, top to bottom:

1. UI-impact headline — wireframes first, when the diff changed rendered UI.
2. Short outcome narrative (`rich-text`): what changed and why, 1-3 paragraphs.
3. `data-model` / `api-endpoint` blocks for schema and contract changes.
4. `file-tree` of the changed files with `change` flags.
5. `## Key changes` — one horizontal `tabs` block of `diff` / `annotated-code`.

Budgets that keep the recap reviewable:

- 3-8 key-change tabs. Fewer than 3 on a large change under-serves the
  reviewer; more than 8 stops being a summary.
- Keep each diff/annotated-code excerpt focused — prefer under ~150 lines per
  tab; summarize or link the rest of a long file instead of dumping it.
- Title at most ~70 characters; brief 1-3 sentences.

**GOOD.** A 25-file auth change: Before/After wireframes of the login surface,
a two-paragraph narrative, a diff-aware `data-model` of the sessions table, an
`api-endpoint` for the new refresh route, a `file-tree` with change flags, and
`## Key changes` with five focused tabs, each with a one-line `summary` and a
few annotations on the load-bearing hunks.

**BAD.** One giant unsegmented diff dump with no summaries or annotations; or a
sparse three-block recap of a 40-file change (one wireframe, one sentence, one
file list) that forces the reviewer back into the raw diff anyway.

## UI Impact Needs Wireframes

When the diff changes rendered UI, layout, density, visual state, interaction
affordances, navigation, controls, menus, dialogs, or design tokens, the recap
MUST include one or more wireframes. Prose and file diffs are not a substitute
for showing what changed visually.

Before choosing wireframes, make a UI coverage pass from the diff:

- Identify the entry surface where the change appears, such as a page header,
  list row, toolbar, route shell, or menu trigger.
- Identify the interaction surface that opens or changes, such as a popover,
  dialog, tab, sheet, dropdown, inline editor, or toast.
- Identify the resulting destination or persistent state, such as a public page,
  read-only view, empty state, error state, loading state, permission-denied
  state, or saved/shared state.
- Identify access or role variants when permissions change. Owner/admin/editor
  versus viewer/non-manager differences are visual behavior and need a compact
  matrix, paired wireframes, or clearly labeled state sequence.

For UI-heavy PRs, a single before/after of the entry surface is not enough.
Show the changed entry point, the main changed interaction surface, and the
resulting/destination state. Add more states when the diff adds tabs, role-based
controls, public/private visibility, invite/manage flows, destructive controls,
or empty/error branches.

Choose the smallest visual surface that makes the review clear:

- Use a `Before` / `After` wireframe pair when the reviewer benefits from direct
  comparison, such as a removed or added control, a changed state, layout
  density, ordering, navigation, or a visible component replacement.
  `references/wireframe.md` owns how to lay that pair out (columns vs.
  vertical stack by geometry).
- Use an after-only wireframe when the change is purely additive or the "before"
  state would only show absence without adding review value.
- Use more than two wireframes when the UI change is flow-dependent, responsive,
  or stateful; show the meaningful states in order instead of forcing a single
  before/after pair.
- For tiny surfaces like menus, popovers, dialogs, toasts, or panels, use the
  matching `surface` (`popover`, `panel`, etc.) and show the focused sub-surface.
  Do not redraw a full page unless placement in the page is itself part of the
  change.

Ground each wireframe in the changed UI behavior, component names, file paths,
and diff-visible labels/states. If exact pixels are inferred rather than
captured, say so in the wireframe caption or a concise annotation. For
local/manual recaps, import or update the plan source that holds the wireframes
so the rendered recap opens with the UI visual available.

## Wireframe Quality — read `references/wireframe.md`

UI recap/plan wireframes must meet a strict quality bar — full-width chrome,
pinned bottom bars, real product content, before/after comparability, the right
`surface` preset, `--wf-*` tokens instead of hex, and no `<html>`/`<style>`/font
tags. Before authoring ANY wireframe / `<Screen>` / `WireframeBlock`, READ
`references/wireframe.md` in this skill directory — it is the single source of
truth for HTML wireframe quality, shared word for word with `/visual-plan`
and `/visual-recap`. Do not author wireframes from memory.

Use the standard `WireframeBlock` / `<Screen>` format so the Plan viewer owns the
surface frame, theme, and sketchy/clean toggle. HTML wireframes are appropriate
when placement precision matters, especially popovers, menus, dialogs, and dense
forms. For HTML
wireframes, keep `renderMode` unset or `wireframe` unless a design-only editable
mockup is explicitly required, because `renderMode="design"` disables the
sketchy rough overlay.

When a browser tool is available, render a UI-impact recap in Planport and
visually inspect it at the current theme before handoff. If any label,
annotation, toolbar, or wireframe content overlaps another element, fix the MDX
and recheck before reporting the link. A text-match screenshot is not enough;
visually inspect the captured image. When no browser is available (for example a
headless CI agent), state that in the recap handoff instead.

## Open And Report The Recap

Open the recap through Planport:

```bash
env -u PORT planport serve plans/<slug> --open
```

Report the printed Planport LAN URL. It is the primary review link for local
recaps and can be opened from other devices on the local network while the
server is running. The command unsets inherited `PORT` so Planport chooses a
random available TCP port by default; use `PORT` or `--port` only when the user
explicitly asks for a fixed port. The URL includes a per-run token; do not
commit it. Local mirror files like `plans/<slug>/plan.mdx` may be mentioned only
as secondary source-control artifacts, not as the main way to open the recap.

When running in Codex and the Browser/in-app side browser tools are available,
open the Planport URL there automatically after creation. Still include the same
LAN URL in the final response.

## Diff → Block Mapping

Map each kind of change to the block that carries it, derived mechanically from
the actual diff. The names below are the CONCEPTUAL block types, not the JSX
tags — resolve every conceptual name to its exact canonical tag with the local
block reference before authoring.

- **Schema / migration change** → `data-model` for the resulting entities,
  fields, and relations. Flag what moved per field/entity with
  `change: "added" | "modified" | "removed" | "renamed"`, and for a changed type
  set `was` to the prior value (e.g. the old column type) — grounded in the real
  migration diff. That diff-aware `data-model` is the headline; reach for a split
  `diff` of the literal SQL only when the exact statement still matters, not by
  default.
- **API / action / route change** → `api-endpoint` with the method, path,
  params, request, and responses as they are after the change. Flag each changed
  param/response with `change` (and `was` on a param whose type/shape changed),
  and set `change` on the endpoint root for a wholly added or removed route. Mark
  removed endpoints with `deprecated: true` and explain in prose.
  Keep multiple API endpoints in the normal single-column document flow unless
  they are an explicit before/after contract comparison.
  Author each request/response example as a SINGLE valid JSON value — one
  top-level object or array, parseable on its own — so it renders in the
  collapsible JSON explorer. Do not put `//` or `/* */` comments, prose,
  trailing commas, or two or more concatenated top-level objects inside one
  example; a non-parseable body falls back to flat text and loses the explorer.
  When an endpoint has several distinct message shapes (for example separate
  websocket frame types, or a success body versus an error body), give each its
  OWN example with its own label rather than cramming them into one body.
- **Compatibility-sensitive change** → short `rich-text` notes beside the
  relevant `data-model` / `api-endpoint` block. Name the changed field,
  endpoint, or behavior and mark whether it is breaking, risky, or non-breaking;
  pair that note with a split `diff` for the literal lines.
- **Any meaningful code hunk** → `diff` with `mode: "split"`, carrying the real
  `before` / `after` text and the `filename` / `language`. Split mode is the
  default for recap code review because before/after legibility is the point;
  use `mode: "unified"` only for a genuinely narrow standalone hunk where
  side-by-side would hide the code. Give every `diff` a one-line `summary`
  saying what the hunk changes and why; it renders as a description above the
  code so the reviewer reads intent first. Never leave a diff unlabeled.
  For the KEY changed files, attach `annotations` to the `diff` so the recap
  calls out what each important hunk does — this is the headline affordance for
  annotating the key files updated. Each annotation anchors to the AFTER-side
  line numbers by default (set `side: "before"` to point at removed lines). Keep
  it to a few high-signal notes per file, not one per line.
  When several key files each need a substantial diff, introduce the group with a
  `rich-text` heading block whose markdown is `## Key changes`, then place the
  `diff` blocks under it in a reusable `tabs` block with horizontal orientation
  (the default — omit `orientation`) so the selected file's split diff gets the
  full document width. Let that heading label the section — do NOT also set a
  `title` on the `tabs` block. Keep each tab label to the file path or a short
  basename plus directory hint.
  If the recap ends with more than one supporting diff, that trailing diff
  appendix should be one horizontal `tabs` block under its own `## Key changes`
  heading, not a stack of separate `diff` blocks.
- **Brand-new file or a substantial added block with no meaningful "before"** →
  `annotated-code` rather than a one-sided split `diff`. Carry the real new code
  with its `filename` / `language` and anchor a few high-signal notes to the lines
  that matter so the reviewer reads what the new code does, not code for code's
  sake. Keep split `diff` for true before/after hunks where the removed lines
  still carry meaning, and group several annotated walkthroughs in a horizontal
  `tabs` block the same way diffs are grouped.
- **Files added / removed / renamed** → `file-tree` with each entry's `change`
  flag (`added`, `removed`, `modified`, `renamed`) and a short `note`; attach a
  `snippet` only when one tells the reviewer something the path does not.
- **Rendered UI / interaction change** → one or more wireframes showing the
  visible UI delta before the reviewer reads code. Use `Before` / `After`
  wireframes when the comparison clarifies the change; otherwise use after-only
  or a short state/flow sequence. Use realistic UI surfaces: for a popover
  change, show a popover with its title row, top-right actions, options/fields,
  tabs, selected/disabled states, people/lists/rows, and any opened prompt/menu
  anchored to the correct trigger. If a route was added, show the route body and
  the unavailable/empty state when the diff implements one. If permissions
  changed, show what managers can do and what viewers/non-managers see instead.
  Keep the body lean: the wireframe carries the UI story, while the file tree
  and `diff` blocks carry implementation evidence.
- **Architecture or data-flow shift** → `diagram` with `data.html` / `data.css`
  as a two-panel before/after, layered, or swimlane layout, or `mermaid` for a
  quick graph. Use two-dimensional layouts; do not reduce a structural change to
  a left-to-right chain. Do not use `diagram` as a stand-in for rendered UI
  controls; UI changes need `wireframe` blocks.
  Author diagram HTML/CSS with the renderer-owned `.diagram-*` primitives
  (`.diagram-panel`, `.diagram-node`, `.diagram-pill`, `[data-rough]`, …) and
  the same `--wf-*` theme tokens `references/wireframe.md` defines — never
  `font-family`, hex, rgb/hsl literals, or one-off dark/light palettes.
- **Outcome-first narrative** → `rich-text` for the "what changed and why" prose:
  the objective the diff served, the key decisions visible in it, and the risks a
  reviewer should weigh. This is the only place the model writes freely.

## Block reference — local Planport components

Before writing structured recap content, READ
`../visual-plan/references/blocks.md` from this skill directory. That file is
the local renderer contract for canonical tags and nested `tabs` block types.
Use conceptual names such as `api-endpoint`, `data-model`, `tabs`,
`question-form`, and `custom-html` in prose only; author the canonical MDX tags:
`Endpoint`, `DataModel`, `TabsBlock`, `QuestionForm`, `CustomHtml`,
`WireframeBlock`, `Screen`, `DesignBoard`, `Section`, `Artboard`, `Annotation`,
`Connector`, `Prototype`, and `PrototypeScreen`.

Legacy aliases render for old recaps but should not be emitted for new ones:
`ApiEndpoint`, `Tabs`, `JsonExplorer`, `CustomHTML`, `CodeTabs`,
`ImplementationMap`, `LegacyWireframe`, and `Wireframe`.

A few recap-specific authoring rules the local reference cannot encode:

- Every block takes a REQUIRED `id` (unique across the whole plan) plus the
  shared optional `summary` / `editable` envelope; give a document section a
  heading by placing a `RichText` block with a Markdown `###` heading directly
  above it.
- Every capitalized block component must be self-closing (`<RichText ... />`) or
  explicitly closed around children (`<RichText ...>...</RichText>`). Never
  leave a bare opening tag like `<RichText ...>` in a paragraph; MDX treats it
  as unclosed JSX and import fails before the recap can render.
- `Endpoint`: prose `description` is the MDX **children** (body between the
  tags), not an attribute; for a WebSocket upgrade use `method="GET"`. Each
  request/response `example` is a JSON **string** (the renderer parses it into
  the JSON explorer), so keep it a single parseable JSON value.
- `TabsBlock`: the whole `tabs` array (including nested child blocks) is ONE
  JSON `tabs={[…]}` prop — there is NO nested `<Tab>` element.
- `WireframeBlock`: its body is a single `<Screen surface ... html=… />` subtree
  (nested MDX, not a flat prop); `html` must be a single-quoted string or static
  template literal, never a dynamic `html={someVar}` expression. See
  `references/wireframe.md` for the HTML rules.
- `Diagram`: the whole payload is one `data={{ html?, css?, nodes?, edges?, … }}`
  attribute and requires either `html` or at least one node; `Mermaid` is its
  own separate block (`source` text), not a `Diagram` prop.
- `QuestionForm`: questions need `id`, `title`, and `mode`; options need `id`
  and `label`.
- `CustomHtml`: use only as a bounded fragment escape hatch. Do not use it for
  wireframes, before/after layout, or diagrams that have native blocks.

## Before / After Is The Headline

The recap's center of gravity is the before/after comparison. For document-body
comparisons there are two primitives, and they cover the whole need together:

- **`columns`** — the side-by-side container, for **structured** comparisons.
  Use two columns labeled `Before` and `After`, each holding a block (commonly a
  `data-model`, `api-endpoint`, or `rich-text`), so the reviewer reads the old
  shape against the new shape in one glance. This is the right primitive for
  "the schema went from X to Y" or "the endpoint contract changed like this."
  Do not use `columns` simply to compact or group a list of API endpoints.
- **`diff`** — for **code**. It renders the literal removed and added lines. Use
  it for the actual hunks. Use split mode by default for recap code review;
  reserve `mode: "unified"` for genuinely narrow standalone hunks where
  side-by-side would hide the code. Key-file diff groups should use horizontal
  tabs so split diffs get the full document width.

For UI diffs, wireframes are the visual comparison primitive. Use before/after
wireframes when the comparison clarifies the change; use after-only or a state
sequence when that better matches the change. The visual headline must show
exact placement, realistic chrome, and adequate padding before any abstract
explanation. Do not stop at the first visible affordance when the diff adds a
flow; show the entry point, the opened surface, and the resulting state or page
so the reviewer can trace the actual user path. `references/wireframe.md` owns
the before/after layout choice —
the `columns` renderer keeps narrow surfaces side by side and auto-stacks wide
`desktop`/`browser` frames vertically; never hand-build a side-by-side
wireframe layout in `custom-html`. For document-body
comparisons, there is no other multi-column primitive — `columns` plus the
`diff` block are the whole comparison vocabulary. Do not hand-build side-by-side
layouts in `custom-html`, and do not stack two `data-model` blocks vertically
and call it a comparison when `columns` exists to put them side by side.

## Grounding Rule

Structured blocks are **true by construction** only if they are derived from the
actual changed lines. The `diff`, `data-model`, `api-endpoint`, and `file-tree`
blocks MUST be built mechanically from the real diff — real paths, real fields,
real method/path, real before/after text — never inferred, rounded, or invented.
The model writes only the prose: the "why", the narrative, the risk read. A
confidently wrong recap is dangerous in a review context, because a reviewer who
trusts the summary may skip the very line the summary got wrong. When the diff
does not contain a fact, leave it out rather than guess; mark anything the model
inferred (not extracted) as inferred in prose.

## Security

- **Keep local links scoped.** Planport URLs are LAN URLs with per-run tokens.
  Do not paste them into public PR comments unless the user explicitly wants a
  local-network review link there. A recap can expose unreleased schema,
  internal endpoints, and architecture; treat it like the source it summarizes.
- **Never transcribe secrets.** A diff can contain API keys, tokens, webhook
  URLs, signing secrets, `.env` values, or credential-looking literals. Do not
  copy any of these into a `diff`, `file-tree` snippet, `api-endpoint`, or prose
  block — redact them (`sk-•••`, `<redacted>`). This mirrors the repo's
  hardcoded-secret rule: obviously fake placeholders only, never the real value,
  in any block, caption, or note.

## Bidirectional Loop

Because a recap is a real, editable plan, the same review loop as forward plans
applies: a reviewer can annotate the Planport view, and the coding agent reads
`comments.json` or the user's pasted Planport `Copy` payload to drive fixes back
into the code — annotation → agent → diff. After a reviewer annotates a recap,
read the local feedback, update the implementation or recap source as needed,
and keep serving the same folder through Planport.

## Related Skills

- **visual-plan** — the canonical command and the source of the shared Wireframe
  & Canvas and Document Quality cores; a recap follows the same block discipline
  in reverse.
- **comment anchors** — recap comments use file/line/text anchors stored in
  `comments.json`.
- **security** — data scoping, secret handling, and the hardcoded-secret rule the
  recap's redaction and visibility gating mirror.
- **sharing** — recap links are local LAN Planport URLs.
