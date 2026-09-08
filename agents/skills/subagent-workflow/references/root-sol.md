# GPT-5.6 Sol root

Use this route only when the actual session model is GPT-5.6 Sol. The actual session model is the root; a client name or an explicit model preference does not silently change it. Sol owns scope, design decisions, integration, and acceptance. Sol is the usual root for normal diagnosis and bounded implementation, and it may make a small root edit when a handoff costs more than it saves.

| Work | Route |
| --- | --- |
| Normal diagnosis, bounded implementation, integration, and acceptance | Sol root |
| Bounded implementation after the approach is settled | Luna `max` by default; Sol root may keep a small or tightly coupled edit when a handoff costs more than it saves |
| Quality-sensitive implementation, merge readiness, focused cleanup, and API shape | Fable 5.1 `high` |
| High-level complex planning, architecture, hard ambiguity, or difficult or high-risk review | Astra at `medium` by default; `low` for focused bounded questions; `high` or above only when explicitly requested |
| Front-end design | Astra, Opus 5, or Fable 5.1; select one based on user preference and task complexity |
| Front-end implementation after the overall design is established | Sol root can implement and extend it |
| Reducing complexity and deciding which code to remove | Astra or Fable 5.1 |
| Integrating an approved removal | Sol root |
| Checking an approved removal for correctness | A fresh Sol review run, read-only for that assignment |
| Native X research or a visual or interactive first pass | Grok 4.6 |
| Deliberate additional Claude opinion | Opus 5 when it adds a distinct perspective |
| Bulk exact transformations | Luna `low`, unless an active user choice requires `max` |

## Scale review to task size

**Small task** (one phase, or a few bounded edits): Sol may implement, integrate, and accept the work directly. A delegated edit still receives the root's checks. Do not add an extra reviewer unless a specific risk justifies it.

**Larger goal** (many phases, or a plan with milestones): Sol sets scope and design, then assigns each settled bounded phase to Luna `max` by default. Sol may keep a tightly coupled phase or established front-end implementation when a handoff would lose context or cost more than it saves. Use a fresh Sol run after each phase for routine correctness, missed consumers, and failure paths when that reduces risk. The fresh run is read-only for that review assignment and returns checked paths, confirmed defects with consequences and required end states, and residual risks. Sol reviews the integrated milestones and final acceptance in the root run. Independent means a separate run, not a different model; do not repeat routine inventories or checks that already have sufficient evidence.

Sol is not the authority for simplification or removal decisions. Astra or Fable 5.1 decides what to delete or how to reduce complexity. Sol can integrate an approved removal and check its correctness. Do not send Fable-written work to Astra automatically. Reserve automatic Astra escalation for evidence that leaves a hard or high-risk question unresolved, and make that request focused on the evidence and decision. These review escalation limits do not restrict the Astra planning, architecture, front-end design, or simplification routes above. A boundary signal alone is not enough.

## Prompting and transport

Apply the Sol section of [frontier-prompting.md](frontier-prompting.md) to Sol root work and to fresh Sol review runs. Keep the root's scope, design, integration, and acceptance in the root thread. Use native workers when available, keep owned scopes separate, and keep staging, commits, publication, external writes, and nested delegation with the root unless separately authorized.
