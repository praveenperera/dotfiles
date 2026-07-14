---
name: visual-plan
description: >-
  Turn a feature idea, rough plan, spec, or implementation direction into a
  standalone Planport visual plan with evidence-backed decisions, diagrams,
  file maps, annotated code, open questions, and UI or prototype review when
  useful. Use to create a new reviewable plan or deeply refine an existing one.
metadata:
  visibility: exported
---

# Visual Plan

Create or refine a structured local Planport plan. The deliverable is the plan
artifact and its review URL, never an inline substitute.

## Boundary

Treat planning as read-only with respect to implementation. Inspect source,
tests, config, docs, and history, but update only the plan folder and its
feedback artifacts. Do not edit application code until the user separately asks
for implementation.

## Workflow

1. Establish the source: the user's idea, an existing plan/spec, or the exact
   referenced artifact. Inspect repository evidence before drafting or asking
   questions. Preserve useful intent, but publish a standalone plan rather than
   a revision memo.
2. Choose the smallest useful review surface: document-only for non-visual
   work, canvas for static UI, canvas plus prototype for flows, or
   prototype-first when interaction is the main uncertainty.
3. Load only the references needed for this task, following the routing below.
4. Author an outcome-first plan grounded in real files, symbols, data shapes,
   UI patterns, and verification commands. Decide evidence-backed issues and
   explain hard-to-reverse choices.
5. Collect every genuinely unresolved decision in one bottom `Open Questions`
   `QuestionForm`. Use stable IDs, decision aids, and recommended defaults. Do
   not run a live interview unless the user explicitly requests one.
6. Put only durable, out-of-scope follow-ups in `next.md` beside the plan. Do
   not use it as a second question list, backlog dump, or substitute for a
   complete active plan.
7. Serve and verify the artifact through Planport, report its printed LAN URL,
   and apply review feedback surgically to the same plan folder.
8. Ask for approval before implementation and name the implementation areas the
   approved plan would touch.

For architecture, data, migration, multi-file, or otherwise risky plans, run
one skeptical review of the written artifact. Fix clear omissions in the plan;
route genuine judgment calls to the single bottom question form.

## Reference Routing

- Always read [Planport workflow](references/planport.md) before creating,
  serving, or updating a plan.
- Read [planning and refinement](references/planning.md) for evidence-first
  investigation, scope, surface selection, question discipline, and coverage.
- Read [block components](references/blocks.md) before introducing or changing
  structured MDX blocks; skip it for a feedback-only prose edit whose existing
  block shape is unchanged.
- Read [document quality](references/document-quality.md) when creating or
  substantially restructuring the technical document; skip it for a narrow,
  well-anchored patch.
- Read [canvas](references/canvas.md) only when the plan needs a top canvas or
  canvas annotations.
- Read [wireframe quality](references/wireframe.md) only before authoring or
  editing a `Screen` or `WireframeBlock`.
- Read the [exemplar](references/exemplar.md) only when the requested artifact
  needs quality calibration or the right plan shape is unclear.

Do not author component syntax or visual layouts from memory when the relevant
reference applies.
