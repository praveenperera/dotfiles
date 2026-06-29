---
name: visual-plan
description: >-
  Turn ordinary text plans into rich interactive visual plans with diagrams,
  file maps, annotated code, open questions, and UI/prototype review when
  useful.
metadata:
  visibility: exported
---

## Installed Mode

Default storage for this installation: local files reviewed through Planport.
Create and update plans and recaps as MDX folders under `plans/<slug>/` when
they should be checked in, or under a repo-ignored/temp folder when they should
stay private scratch. Before authoring structured MDX, read the bundled
reference docs for block components, canvas, wireframe, and document rules. Then
run `env -u PORT planport serve plans/<slug> --open` and report the printed LAN
URL.
`planport` serves the review UI and the local MDX files from this machine, binds
to the local network, and chooses a random available TCP port by default so many
local plans can run at once. Use `PORT` or `--port` only when the user
explicitly asks for a fixed port. It writes review feedback to
`plans/<slug>/comments.json`. The printed URL includes a per-run token; do not
commit tokenized URLs. If `planport` is not installed, run `cmd release
planport` first. No external Plan UI, no remote database, and no sharing by
default.


# Planport Visual Plans

Planport visual plans are structured local planning artifacts for coding agents.
Build the plan you would normally write in Markdown, but as a scannable document
with editable blocks mixed in: inline diagrams, code snippets,
open questions, and an optional top visual review area (wireframe canvas, live
prototype, or both in tabs). Architecture and backend plans stay document-only;
UI and product plans start with the top canvas/prototype (the Visual Surface
Choice section owns that rule).

`/visual-plan` is the packaged command and main entry point. Choose the review
mode from the task: UI-first when the work is primarily product UI and review
should start with screens, prototype-first when review should start with a
functional live prototype, design-first when review needs full-fidelity branded
screens, or visual-intake when the user explicitly wants a questionnaire before
planning. When a Codex, Claude Code, Markdown, or pasted plan already exists,
`/visual-plan` uses that source plan as the starting point and builds the review
surface from it instead of starting over.

## When To Use

Create or adapt a visual plan whenever the plan would be better as a reviewable
artifact than a chat paragraph. This includes modest work such as a single UI
surface with states, a small workflow, a before/after product change, or a
component/API/data-shape decision that needs alignment, plus larger multi-file,
ambiguous, long-running, risky, or UI-heavy work. Use it when architecture /
data flow / UI direction / options / open questions would benefit from inline
diagrams or structured blocks, when the user needs to react to a direction
before you implement, or when an existing text plan needs a richer review
surface.

## Plan Discipline

- **Gate thoughtfully.** A visual plan is a richer review surface, not only a
  tool for giant projects. Use it when the user needs to see, compare, comment
  on, or approve a direction before code, even for a modest UI/state/workflow
  change. Skip it for truly trivial, unambiguous work — typos, one-line fixes, a
  single well-specified function, anything whose diff you could describe in one
  sentence — and just make the change. Never pad a plan with filler and never
  ship a single-step plan.
- **Research before you draft.** Read the real files, actions, schema, and
  patterns first; name actual files, symbols, and data shapes instead of
  inventing them. Check existing `actions/` before proposing endpoints and prefer
  named client helpers over raw fetch. Delegate wide exploration to a sub-agent.
  Lead with reuse: for each step, name what it reuses — existing actions, schema,
  components, helpers — before what it adds, so the plan explains the genuinely new
  delta instead of redescribing what already exists.
- **Decide the hard-to-reverse bets first.** For non-trivial backend, data, or API
  work, sketch where the feature is headed, then call out the decisions that are
  expensive to undo once data or callers depend on them — wire format, public ids,
  data-model shape, auth and ownership boundaries — and get those right in the plan
  even if most of the feature ships later. Then scope to the smallest first cut that
  proves the approach without foreclosing it, stating both what is in and what is
  explicitly deferred.
- **Keep examples at the right altitude.** When the user's idea is a broad
  framework, product, or operating-model change, do not collapse it into the
  first concrete example, provider, or sync path they mention. Separate the core
  abstraction from motivating examples and app/provider adapters. Use examples
  to make the plan legible, but label them as examples unless they are the whole
  requested scope.
- **Publish standalone plans.** If the user pasted, referenced, or already has a
  Codex / Claude Code / Markdown plan, treat it as source material, but rewrite
  the published plan as a clean standalone proposal. Preserve the source plan's
  useful intent and codebase facts, label inferred visuals as inferred, and avoid
  revision language such as "preserve the prior plan", "do not drop the old
  idea", "unlike the previous version", or "this revision changes...". A reader
  who never saw the chat or earlier drafts should understand the plan.
- **Make the first read concrete.** If the plan is meant to be shared with
  someone outside the chat, or if the concept is abstract, lead near the top with
  one concrete product example before mode tables, architecture, or roadmaps. For
  UI-capable concepts, that usually means a top-canvas app state that shows the
  real user workflow in product terms. Do not rely on phrases that only make
  sense in conversation, and do not frame the plan as "not the old idea"; state
  the positive model directly.
- **Planning is read-only.** Make no source edits while building or reviewing the
  plan. Start editing only after the user approves the direction.
- **Clarify vs. assume.** Do not ask how to build it — explore and present the
  approach and options in the plan. Ask a clarifying question only when an
  ambiguity would change the design and you cannot resolve it from the code; use
  the host agent's normal ask-user-question flow and batch 2-4 high-leverage
  questions before finalizing. Do not create a separate visual intake artifact
  unless the user explicitly asks for one. Otherwise state the assumption
  explicitly and proceed, and keep anything unresolved in the plan's single
  bottom `question-form` Open
  Questions block. For complex plans, do a final open-question pass before
  handoff: if a decision would affect architecture, scope, UX, data shape, or
  rollout, either decide it in the plan with rationale or put it in that bottom
  form with a recommended default.
- **The plan is the approval gate.** After surfacing it, ask the user to review
  and approve before you write code, and name which files/areas the work touches.
  Presenting the plan and requesting sign-off is the approval step — do not ask a
  separate "does this look good?" question.
- **The document is the source of truth, not the chat.** When scope shifts,
  update the local MDX plan rather than only changing course in chat, and make
  the updated document stand alone. Do not describe the update as a correction
  to an earlier draft inside the plan itself. Re-read the approved plan before
  major steps.

## Create A Structured Local Plan — Never Inline

The deliverable is ALWAYS a structured local Planport visual plan, not a
chat-only plan. By default, create it as a local MDX folder and review it
through `planport`. Plans are portable source artifacts (`plan.mdx`, optional
`canvas.mdx`, optional `prototype.mdx`, optional `.plan-state.json`, and
`comments.json` feedback). NEVER hand the plan over as inline chat content — no
Markdown prose, ASCII sketch, table, or fenced wireframe as the final artifact.
Planport is the only collaboration surface.

## Core Workflow

1. Follow the host agent's normal planning flow: inspect the codebase, delegate
   wide exploration when useful, gather the info needed, and ask native
   clarifying questions as needed before generating the plan. If a source plan
   already exists, gather its exact text from the user's paste, a referenced
   file, or recent visible agent context; do not invent source text.
2. Read `references/blocks.md`, the relevant bundled references, and existing
   local plan examples before writing structured MDX — do not author from
   memorized tags. When a source plan already
   exists, preserve its useful intent while producing a standalone plan document,
   not a revision memo.
3. For UI/product plans, compose the top canvas first with the primary
   wireframes and annotated states, then write the document with native blocks
   (see `references/canvas.md` and `references/document-quality.md`). For
   broad product architecture plans with a user-facing implication, add a
   concrete "what this looks like in the app" visual before the abstract
   architecture or mode tables. Keep the document close to the standalone
   Markdown plan the agent would normally output. If an existing plan was
   provided, carry forward the right facts and decisions without referring to
   the previous draft or explaining how this version differs. For non-visual
   plans, skip the top visual surface (Visual Surface Choice below owns the rule)
   and put `diagram`, `data-model`,
   `api-endpoint`, `diff`, `file-tree`, `code`, and `annotated-code` blocks
   directly next to the relevant prose.
4. Serve with `env -u PORT planport serve plans/<slug> --open`. Include the
   printed LAN URL in chat so the next step is a click in CLI or other text-only
   hosts. Do not pass a fixed port by default; let Planport choose a random
   available TCP port. When the host exposes an embedded browser/preview panel
   and a tool can open arbitrary URLs there, open the URL automatically for
   convenient review and smoke-test the render; when no browser is available,
   fetch the Planport API with the printed token. For high-stakes plans
   (architecture, backend, data, multi-file, or risky), also kick off the
   self-review pass in **Self-Review Before Handoff** while the user reads,
   instead of blocking the handoff on it.
5. Read `comments.json` before editing, after review, after any long pause, and
   before the final response. Treat the line/file anchors and comment body as the
   source of truth for exactly what each comment points at. If the user pasted a
   `Copy` payload from Planport, use that payload the same way.
6. Apply changes by editing the MDX files directly. Keep edits surgical and
   preserve every existing block and visual surface. Restart or keep using
   `planport` against the same folder and reload the review URL.

## Self-Review Before Handoff

For high-stakes plans — architecture, backend, data-model, migration, multi-file,
or otherwise risky work — run one adversarial self-review pass before treating the
plan as final. Skip it for small, UI-only, or single-decision plans where the cost
outweighs the value. Keep the pass cheap and non-blocking:

- **Surface the plan first, review concurrently.** Post the link and let the user
  start reading, then run the review in parallel — never make the user wait on it.
- **Review the written plan; do not re-research.** Critique the plan text and its
  own blocks. The grounding was already done while drafting, so the review checks
  the output instead of re-exploring the repo.
- **Spawn one skeptical reviewer** whose only job is to find what is weak, missing,
  or wrong — not to praise. Point it at: hard-to-reverse decisions made implicitly
  or not at all (wire format, public ids, data-model shape, auth, ownership); steps
  not anchored in real files or symbols; a menu of options where the plan should
  commit to one; obvious missing decisions ("what happens when X?", "why not Y?");
  and padding or single-step filler.
- **Fix vs. ask.** Apply clear-cut fixes yourself by patching the MDX files —
  vague non-goals, unanchored claims, an obvious missing decision. Route genuine
  judgment calls back to the user instead: add them to the bottom `question-form`
  Open Questions block or batch them into the normal ask-user-question flow. Do
  not silently decide them.
- **Do not surprise the user mid-read.** On a large plan, apply the patches before
  the editor loads; otherwise note briefly that a self-review is running so the
  plan changing under them is expected. When you next respond, summarize what the
  review changed and what it surfaced for the user to decide.

## Visual Surface Choice

Choose the surface before creating the plan or after reading the source plan. Do
not add visual chrome by default:

For UI/product plans, the top canvas is usually the primary review surface. Put
the first meaningful wireframes there, not buried as document-body blocks. Use
multiple canvas artboards when states matter, such as the default view, an
overflow menu or popover, a side panel, loading, or error. Put short annotations
beside frames with `targetId` plus `placement`; keep implementation details,
tradeoffs, file maps, data contracts, risks, and verification in the document
body below the canvas.

Keep product wireframes and explanatory/meta diagrams separate. Start with pure
screens that look like the app state under discussion, without callout prose or
architecture notes embedded inside the UI. Put arrows, labels, contracts, data
flow, and mode explanations in separate annotations, separate canvas diagrams,
or the document body.

When the plan touches an existing app, inspect the current shell/components
before drawing. The first artboard should look like the real app at the same
density: existing sidebars, toolbar placement, overflow menus, app chrome, and
framework agent chrome stay in their real places. Model secondary surfaces as
separate states, such as a top-right overflow popover, sheet, panel, loading
state, or separate AgentSidebar, rather than inventing a permanent inspector or
folding framework chrome into the product UI.

- **No visual surface** for architecture-only, backend-only, data migration,
  copy-only, or otherwise non-visual plans. Do not use the top canvas for
  architecture diagrams, dependency maps, file plans, API contracts, or
  data-flow-only reviews. Use a strong document with local inline diagrams
  only when relationships need a visual explanation, usually one spatial diagram
  per recommendation or decision. Prefer grouped regions, layers, quadrants,
  matrices, or before/after panels over a single-axis chain unless the
  relationship is truly sequential.
- **Canvas only** for one static screen, a before/after comparison, a component
  state, a small popover, or a visual direction that does not require clicking.
  Put those wireframes in `canvas.mdx` and omit `prototype.mdx`.
- **Canvas + prototype** for multi-step UI flows, onboarding, wizards,
  review/approval flows, navigation changes, or anything where the reviewer
  needs to operate the behavior. Keep the static wireframes in
  `canvas.mdx`, add the aligned functional prototype in `prototype.mdx`, and
  rely on the top visual tabs to switch between them.
- **Prototype-first** when the user asks to operate the UI or when interaction is
  the main question. Author `prototype.mdx` with `<Prototype>` and
  `<PrototypeScreen>` while still preserving static mocks where useful.

For mixed canvas + prototype plans, reuse the same real labels, app statuses,
and screen ids across both surfaces. The canvas is the inspectable static reference;
the prototype is the interactive version of that same flow, not a separate
design direction.

## Block components — read `references/blocks.md`

The local renderer supports a fixed MDX component set. Before authoring any
capitalized block tag or nested `tabs` block, READ `references/blocks.md` in
this skill directory. Use canonical tags such as `Endpoint`, `DataModel`,
`QuestionForm`, `CustomHtml`, `TabsBlock`, `WireframeBlock`, `DesignBoard`, and
`Prototype`; do not author legacy or alias tags unless preserving an existing
plan.

## Wireframe quality — read `references/wireframe.md`

UI recap/plan wireframes must meet a strict quality bar — full-width chrome,
pinned bottom bars, real product content, before/after comparability, the right
`surface` preset, `--wf-*` tokens instead of hex, and no `<html>`/`<style>`/font
tags. Before authoring ANY wireframe / `<Screen>` / `WireframeBlock`, READ
`references/wireframe.md` in this skill directory — it is the single source of
truth for HTML wireframe quality, shared word for word with `/visual-plan`
and `/visual-recap`. Do not author wireframes from memory.

## Canvas — read `references/canvas.md`

The canvas is the single source of truth for static UI mockups: the `surface`
locks each artboard's footprint, mixed surfaces lay out
in lanes, annotations are plain-text designer notes anchored by
`targetId`/`placement`, and edits are surgical local source patches. Before
authoring or editing ANY canvas, artboard, or annotation, READ
`references/canvas.md` in this skill directory — it is the single source of truth
for canvas/artboard mechanics. Do not author canvas layouts from memory.

## Document quality — read `references/document-quality.md`

The document is a serious technical plan, not marketing: outcome-first,
prose-first, self-contained, built from the right native blocks, with open
questions in a single bottom `question-form` and a pre-handoff visual check.
Before authoring the plan document, READ `references/document-quality.md` in this
skill directory — it is the single source of truth for the document quality bar.
Do not write the document from memory.

## Good vs. bad exemplar — read `references/exemplar.md`

For a worked example of the bar — a great UI-first plan and `/visual-plan`, plus
the anti-patterns to avoid — READ `references/exemplar.md` in this skill
directory before authoring a plan.

## Planport Local Mode

Planport local mode is the default for this installation. Use it whenever the
user needs a reviewable visual plan. It provides fully local files, LAN access,
and repo-owned/source-controlled planning artifacts. Planport is the only
default review server.

The Planport contract is:

- Read source context from local files and shell commands only.
- Read `references/blocks.md` and the relevant bundled references before
  writing structured MDX. For `checklist` and `question-form`, follow the
  required shapes exactly: checklist items need `id` and `label`; question-form
  questions need `id`, `title`, and `mode`; and each option needs `id` and
  `label`.
- Write the plan as a local MDX folder: use `plans/<slug>/` when the user
  wants the artifact checked into the repo, or use a repo-ignored/temporary
  folder such as `_scratch/plans/<slug>/` or `/tmp/planport-plans/<slug>/` when
  it should not be checked in. The folder contains `plan.mdx`, optional
  `canvas.mdx`, optional `prototype.mdx`, and optional `.plan-state.json`.
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
  includes the expected title/files. If the browser cannot load the plan, use
  this endpoint to read the concrete server error.
- Treat feedback as file or chat feedback: read `comments.json` or the user's
  pasted Planport `Copy` payload, update the MDX files directly, and keep
  serving the same plan folder with Planport.

Planport local mode keeps plan content local. It does not by itself make the
coding agent's language model local; for that stronger privacy boundary, the
host agent/model must also be local or otherwise approved by the user.

## Interpreting comment anchors

In Planport local mode, comments are file/line/text anchors stored in
`comments.json`, or serialized into the user's pasted Planport `Copy` payload.
Read those before acting on any comment.

## Local Links

Planport URLs are LAN URLs with per-run tokens. Do not paste them into public
issues, PR comments, or durable docs unless the user explicitly wants a
local-network review link there.

## Setup & Authentication

Planport is the default local review surface and does not require auth.
It is installed with this repo's `cmd release planport` workflow. The Planport
CLI serves local plan folders for review.
