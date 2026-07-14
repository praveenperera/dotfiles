# Evidence-first planning and refinement

Use this guide for both new plans and deep refinement of rough plans, specs, and
existing Planport artifacts.

## Evidence before questions

- Inspect the real repository first with targeted searches and file reads.
  Ground the plan in current APIs, actions, schemas, config defaults, tests, UI
  components, logs, commands, and architectural conventions.
- For an unfamiliar dependency or external repository, inspect generated docs,
  installed sources, `btx`, or official documentation before asking the user to
  speculate about behavior.
- Separate confirmed evidence, inferred conclusions, recommended decisions, and
  unresolved user intent. Label inferences instead of presenting them as facts.
- Ask only about intent, scope, product behavior, preference tradeoffs, approval,
  or context unavailable from inspectable sources. When evidence answers part
  of a question, record that finding and ask only for the remaining decision.
- Lead with reuse. Name existing actions, schema, components, helpers, and
  conventions before describing the genuinely new delta.

## Plan discipline

- Make the artifact standalone, outcome-first, and specific. Include objective,
  done state, goals, non-goals, evidence, chosen approach, ordered implementation
  steps, risks, and concrete verification.
- Preserve the prompt's level of abstraction. Keep a reusable core separate from
  motivating examples, providers, or adapters unless the example is the entire
  scope.
- Decide hard-to-reverse choices early: wire formats, public identifiers, data
  model shape, auth and ownership boundaries, migrations, and compatibility.
  Scope the smallest first cut that proves the direction without foreclosing it.
- Do not write revision language tied to chat history. Fold source-plan intent
  and later answers into a clean proposal that stands alone.
- Cover applicable edge cases, failure and recovery paths, partial success,
  retries, lifecycle and cleanup, accessibility, responsive states, privacy,
  security, performance, concurrency, operations, and maintenance.
- Protect user-visible behavior and non-obvious invariants in the verification
  strategy. Include a workflow-level smoke check when work crosses UI,
  persistence, sync, provider, or application boundaries.

## Visual surface selection

- Use no top visual surface for backend-only, architecture-only, migrations,
  config, CLI, copy-only, or other non-visual plans. Put only useful diagrams,
  data models, file maps, contracts, and annotated code inline.
- Use a canvas for static UI review: a screen, before/after comparison, component
  state, popover, sheet, or empty/loading/error state.
- Use canvas plus prototype for multi-step flows, onboarding, navigation,
  wizards, or review/approval behavior.
- Use prototype-first when interaction is the central question or the user asks
  to operate the flow.
- Inspect the real product shell before drawing an existing app. Preserve its
  navigation, chrome, density, labels, menus, and role-specific behavior. Keep
  user-facing screens separate from architecture and implementation diagrams.

## Batch questions and `next.md`

Resolve every issue the evidence can answer. Put all remaining decisions in one
bottom section titled `Open Questions`, preferably as one `QuestionForm`.

- Give each question a stable ID and the minimum useful evidence, impact,
  options, tradeoffs, and recommended default.
- Choose single-choice, multi-choice, or freeform mode to match the decision.
  Do not add an explicit `Other` option when the renderer supplies write-in.
- Do not duplicate questions in prose, another question section, `next.md`, or a
  live ask-user flow. Run a final audit across architecture, scope, UX, data,
  rollout, ownership, permissions, migration, and compatibility.
- Do not run a live interview unless the user explicitly asks for interactive
  refinement. Otherwise let the reviewer answer the consolidated form in one
  pass.

Create `next.md` only for concrete adjacent work that should survive but is
deliberately outside the active plan. Keep it short, explain why each item is
deferred, and omit it when no such follow-up exists. Never move required work,
open decisions, or verification debt into `next.md`.
