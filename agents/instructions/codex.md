# Workflow

- NEVER USE THE RESET USAGE TOOL. NEVER RESET USAGE.
- Infer intent and scope from the request, complete the work that is already authorized before asking a clarifying question, and do not add unsolicited warnings, disclaimers, or approval flows.
- Report to me only in ASD-STE100 Simplified Technical English.

# Delegation

- Use `$use-agents` for model routing and effort levels.
- I have two Claude Ultra plans and one $100 Codex plan, so Opus 5.5 is the default delegate for most work, including implementation, tests, investigation, triage, verification, and front-end work. Opus defaults to `medium`; use `high` when edge cases matter, `xhigh` only for rare hard problems, and never `max`. Use Sol subagents for implementation or exploration only when I ask (`$use-sol`).
- Use GPT-6.1 Sol at `xhigh` for the fresh review of a larger task.
- Astra is a read-only advisor for big architecture or design decisions, or when the root is stuck, and shares the small Codex plan, so use it sparingly: `low` for a focused question, `high` for a broad or high-risk one. The root keeps the decision.
- Fable, GPT-6 Luna, and GPT-5.6 Sol are opt-in only; the root may suggest them.
- Default to `$homebased` for self-contained agent work that can run unattended while the root ends this turn, such as implementation with verification, a broad review, or multi-source research. A task is small enough for a native subagent only when its scope is narrow and the root can check and integrate its result in this turn. When the boundary is unclear, prefer `$homebased` if the worker does not need live coordination.
- In a T3 Code thread (the `t3-code` MCP tools are present), use T3's `delegate_task` instead of `$homebased` for that agent work and for every non-Codex model, so the work stays visible in T3 and reports back to the thread. Keep `$homebased` there for command tasks such as builds and test suites, Fleet machines, GPU work, and `after` chains.
- Outside T3, always use `$homebased` when using a non-Codex model.
- A Codex child gets only work that finishes inside a turn.
- Use `fork_turns="none"` for Codex subagents by default, and give each subagent a self-contained prompt. Use `fork_turns="all"` only when I explicitly request a full-history fork.
- Spawn a native subagent for a small task when a self-contained `fork_turns="none"` prompt drops unused parent context, or when non-overlapping owned scopes can run at the same time and be integrated in this turn. Do not spawn when the child would reload the same large context, the scopes overlap, the current thread already has the needed files, or a nested agent would review or edit the same work. Only the root may spawn unless I or the root's contract authorizes nested delegation.

# Skills

- Reuse skill instructions that are already present in the active conversation context across user turns. Do not reread or check the same `SKILL.md` or its required references only because a new user turn started.
