---
name: visual-refine-plan
description: Create or refine a reviewable visual implementation plan with refine-plan depth. Use when a rough plan, spec, feature idea, or implementation direction needs evidence-first investigation, diagrams, UI/prototype review when useful, file maps, annotated code, risks, verification, and comprehensive unresolved questions collected at the bottom for batch answers.
---

# Visual Refine Plan

Turn a rough plan, idea, or existing spec into a standalone visual plan. This skill combines the review surface of `visual-plan` with the requirement depth of `refine-plan`: inspect evidence first, decide what can be decided, and collect every unresolved decision in one bottom questions section. Do not run an interactive interview unless the user explicitly asks for one.

## Core Standard

- Produce a durable plan artifact, not a chat-only outline
- Make the plan stand alone for a reviewer who has not seen the conversation
- Keep planning read-only; do not edit implementation files while refining the plan
- Use visuals only when they improve review: UI surfaces, prototypes, diagrams, file maps, data models, or annotated code
- Put every answerable unresolved decision at the bottom as a single questions form or section
- Use `next.md` beside the plan only for deferred follow-ups that should survive but do not belong in the current scope

## Evidence Before Questions

Before adding a question to the plan, decide whether the answer should already exist in code, docs, tests, config, design artifacts, or prior plans.

- Inspect the current repo first with targeted file reads and searches
- If the plan depends on an external repo, upstream project, or unfamiliar library, use `btx`, generated docs, installed crate/package sources, or official docs before asking about behavior
- Treat current APIs, schemas, config defaults, tests, UI patterns, logs, commands, and architectural conventions as evidence to investigate, not hypotheticals for the user
- Ask only for intent, product decisions, scope boundaries, preference tradeoffs, approval, or missing context that cannot be answered from source
- When evidence answers part of a question, write the finding into the plan and ask only for the unresolved decision
- State assumptions explicitly when proceeding with a recommended default

## Output Contract

Use a structured Agent-Native visual plan when the Plan tools or local Plan CLI are available. If the user gives a target file or existing plan/spec, refine that artifact or import it as source material. If no target is provided, choose a conservative local plan path such as `plans/<slug>/` for repo-owned plans, or a repo-ignored/private plan path when the user asks for scratch/private output.

The plan should include the sections and blocks that make the work reviewable:

- Objective, done state, goals, and non-goals
- Current evidence with concrete file paths, commands, APIs, symbols, data shapes, configs, or UI components
- Proposed approach with key decisions and rationale
- Implementation plan ordered by dependency and risk
- UI and UX behavior, including states and error paths when relevant
- Data/API contracts, migrations, lifecycle, cleanup, and compatibility risks when relevant
- Security, permissions, privacy, performance, scalability, and maintenance concerns
- Verification with concrete commands and at least one workflow-level smoke check when the feature crosses UI, persistence, sync, providers, or app boundaries
- Bottom `Open Questions` form/section with stable question IDs and recommended defaults when appropriate

Do not include unresolved questions as accepted requirements. Do not hide product or architecture choices in vague steps such as "wire this up" or "make it work."

## Rendering Contract

This skill uses `visual-plan` for the final review surface whenever available.
The final deliverable must be an Agent-Native local plan, not plain Markdown.

When local Plan CLI output is available:

1. Read the `visual-plan` skill instructions.
2. Run `npx -y @agent-native/core@0.75.5 plan blocks --out plan-blocks.md`.
3. Author the plan as `plan.mdx` in a local plan folder.
4. Run `npx -y @agent-native/core@0.75.5 plan local check --dir <plan-dir>`.
5. Run `npx -y @agent-native/core@0.75.5 plan local serve --dir <plan-dir> --kind plan --open`.
6. Also create a static HTML fallback with `npx -y @agent-native/core@0.75.5 plan local preview --dir <plan-dir> --kind plan --out _scratch/<slug>-preview.html`.
7. Report both the local Plan URL and the `file://` HTML preview path.

Plain Markdown may be used only as scratch/source notes, never as the final artifact, unless the user explicitly asks for Markdown or the Plan CLI is unavailable.

## Visual Surface Choice

Choose the review surface based on the work.

- Use no top visual surface for backend-only, architecture-only, data migration, config, CLI, or copy-only plans. Put diagrams, data models, file maps, and annotated code inline near the relevant decision.
- Use a canvas for static UI review: one screen, a before/after comparison, a component state, an empty/loading/error state, or a small popover/sheet.
- Use canvas plus prototype for multi-step UI flows, onboarding, wizards, approval/review flows, navigation changes, or behavior the reviewer needs to operate.
- Use prototype-first when interaction is the main uncertainty or the user explicitly asks to try the flow.
- For UI plans touching an existing app, inspect the current app shell/components before drawing. Preserve real navigation, sidebars, toolbar placement, density, labels, and framework chrome.
- Keep product wireframes separate from architecture diagrams. The UI surface shows what users see; the document explains files, data, contracts, risks, and rollout.
- Avoid decorative visuals, marketing hero layouts, filler diagrams, and duplicate representations of the same idea.

## Open Questions

Place all unresolved decisions at the bottom of the artifact in a single section titled `Open Questions` or `Questions For User`.

- Prefer the Agent-Native `question-form` block when using visual plan tooling
- Group related questions only when it improves scanning
- Give each question a stable ID such as `Q1`, `Q2`, or a semantic ID supported by the plan tool
- Include a short evidence note explaining why the decision matters
- Use single-choice, multi-choice, or freeform mode based on the decision type
- Mark a recommended default when the evidence supports one
- Do not add an explicit `Other` option when the renderer already provides a write-in field
- Make the set comprehensive enough that the user can answer in one pass
- Do not ask live `AskUserQuestion` questions for items already captured in the bottom section unless the user asks for an interactive follow-up

Run a final question audit before handoff. For architecture, scope, UX, data shape, rollout, ownership, permissions, provider mapping, migration, or compatibility decisions, either commit to a recommendation with rationale or add the decision to the bottom questions section.

## Thoroughness Checklist

Cover the non-obvious dimensions that would otherwise cause rework:

- Edge cases, failure modes, retries, empty states, partial success, rollback, and recovery paths
- Integration points, ownership boundaries, compatibility constraints, and public API or wire-format commitments
- Security implications, attack surfaces, authz/authn behavior, privacy, and data retention
- Performance, scalability, concurrency, caching, rate limits, and operational maintenance
- User mental models, accessibility, keyboard/screen-reader behavior, responsive states, and copy implications
- Data lifecycle, migrations, backfills, cleanup, import/export, sync, and idempotency
- Tradeoffs and alternatives considered, especially hard-to-reverse decisions
- Test strategy that protects user-visible behavior and non-obvious invariants

## Workflow

1. Gather source context from the repo, docs, tests, configs, prior plans, and external sources as needed.
2. Identify the output mode: hosted Plan tool, local Agent-Native plan folder, existing plan update, or explicit fallback requested by the user.
3. Choose the visual surface: no top surface, canvas, canvas plus prototype, or prototype-first.
4. Draft a standalone plan with confirmed evidence, decisions, implementation steps, risks, and verification.
5. Add every unresolved decision to the single bottom questions section with evidence notes and recommended defaults when possible.
6. Put deferred or adjacent follow-ups in `next.md` beside the plan instead of expanding the active scope.
7. Verify the artifact renders or validates with the available plan tooling before handoff.
8. Report the plan path or URL and summarize the highest-impact open questions.

## Important

- Keep the plan current if the user answers questions and asks for refinement
- Do not ask the user to speculate about behavior that can be checked in source or an inspectable external reference
- Do not leave a plan as a list of questions; make every evidence-backed decision the plan can responsibly make
- Keep questions at the bottom so the user can answer them in one pass
- Let the caller decide whether to approve, revise, or implement from the refined visual plan
