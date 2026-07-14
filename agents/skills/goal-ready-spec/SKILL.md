---
name: goal-ready-spec
description: Create or revise a guarded, auditable implementation spec for Codex or another agent. Use only when the user explicitly invokes $goal-ready-spec or specifically asks to make a spec goal-ready.
---

# Goal-Ready Spec

Create a durable implementation contract under `_plans/<short-slug>/` that preserves explicit requirements, ownership, and proof of completion.

## Workflow

1. Read the request, source plan, repository instructions, and targeted code, documentation, configuration, and tests that can resolve material facts.
2. Read [references/contract.md](references/contract.md) before classifying requirements or designing the plan folder.
3. Ask focused questions when unresolved intent would change scope, ownership, acceptance, or verification. Do not finalize a weak contract.
4. Preserve the source in `original-spec.md`, then create or revise `spec.md`, `progress.md`, `decisions.md`, and `audit.md` from [references/templates.md](references/templates.md).
5. Keep `spec.md` a compact controlling router. Record design and interpretation choices in `decisions.md` when they are not fully spelled out as binding requirements. Move phase detail, context, long decision writeups, audits, and evidence into support files only when needed, and link each from the Read Map.
6. Trace every binding material requirement to acceptance criteria, completion criteria, or an audit row. Preserve explicit owner, state-location, lifecycle, API-shape, and no-equivalent-substitution requirements.
7. Define exact verification commands and evidence. Require objective and architectural evidence, not behavior-only equivalence.
8. Run the hygiene check and fix every error before handoff:

   ```sh
   python3 agents/skills/goal-ready-spec/scripts/plan_file_hygiene.py _plans/<short-slug>
   ```

9. Return the created paths, unresolved decisions or risks, hygiene result, and the paste-ready Set Your Goal prompt from the template.

Keep the generated spec imperative and task-specific. Inherit current user, repository, and runtime policies. Record only task-specific deviations; do not embed a generic delegation or tool-usage manual.
