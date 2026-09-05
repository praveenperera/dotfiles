# GPT-6 Astra root

Use this route only when the actual session model is GPT-6 Astra. Keep architecture, diagnosis, integration, and acceptance in the root thread.

| Work | Route |
| --- | --- |
| Domain design, ambiguous decisions, hard debugging, broad investigation | Astra root |
| Bounded implementation with a decided approach and cheap checks | Native Luna `max` worker |
| Implementation that needs sustained code-quality judgment; focused cleanup | Fable 5.1 |
| Hard fix tightly coupled to the root's diagnosis | Astra root when a handoff would lose essential context |
| Independent review of root-written code | Fable 5.1 for merge readiness; Grok or a fresh Astra run for a distinct correctness concern |
| Native X research or a visual first pass with clear scope | Grok 4.6 |
| Deliberate additional Claude opinion | Opus 5 only when it adds a distinct perspective |
| Bulk exact transformations | Luna `low`, unless an active user choice requires `max` |

Use native Codex workers when available. Use [claude-cli.md](claude-cli.md) for Fable 5.1 or Opus and [grok-cli.md](grok-cli.md) for Grok. External Codex transport is in [codex-cli.md](codex-cli.md).

Apply the Astra section of [frontier-prompting.md](frontier-prompting.md) to the root as well as its delegates. Do not let a skill's routine approval step interrupt work already authorized by the user. Keep plan-only requests limited to planning.
