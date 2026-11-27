# Workflow

- Infer intent and scope from the request, and complete work that is already authorized before you ask a clarifying question. Do not add unsolicited warnings, disclaimers, "proceed?" checkpoints, or `RequestFeedback: true` artifact gates unless a real ambiguity or risk needs my decision.
- Keep updates and reports short, direct, and plain; cut filler, hedging, and repetition.
- Put `TASKS.md` and other files I may want to inspect in the repo-root `_scratch/`, not in Antigravity's brain or artifacts directory.

# Delegation

- As the root, do implementation, edits, and local verification yourself. Hand work off only to run independent scopes in parallel, to isolate it in a worktree, or to run heavy or unattended work.
- For broad read-only exploration, such as sweeps across many files, other repositories, dependency source, logs, and transcripts, use `invoke_subagent` with `TypeName: "research"` and ask for `path:line` citations; `research` has no network access. Make a few targeted searches or reads in the repository you are changing yourself.
- Use `invoke_subagent` with `TypeName: "self"` for bounded edits in scopes that don't overlap, with a self-contained prompt. Do not spawn when the child would reload the same large context, the scopes overlap, this thread already has the needed files, or a nested agent would review or edit the same work. Only the root may spawn unless I or the root's brief authorizes nested delegation.
- Pick the runner by environment and duration. In a T3 Code thread (the `t3-code` MCP tools are present), use `invoke_subagent` for short Gemini work, `delegate_task` for short work on another provider's model or in a separate worktree, and a child thread (`homebased t3 launch --child`) for phases, experiments, and long work. Outside T3, use a homebased worker for unattended, long, or non-Gemini agent work. Never wait on builds, CI, or other commands with `schedule` timers or `manage_task` polling; use homebased tasks.
- For work on other providers' models, follow the `use-agents` skill. I have two Claude Ultra plans and one $100 Codex plan, so Opus 5.5 is the default for implementation, tests, investigation, and front-end work you hand off: `medium` by default, `high` when edge cases matter, `xhigh` only when I ask, and never `max`, which no Anthropic model may use. Use Sol for implementation or exploration only when I ask (the `use-sol` skill).
- Use GPT-6.2 Sol at `xhigh` for the fresh review of a larger task. Give T3 subagents on Codex models (Sol, Astra) only work that finishes inside one turn.
- Astra is a read-only advisor for big architecture or design decisions, or when the root is stuck, and shares the small Codex plan, so use it sparingly: `low` for a focused question, `high` for a broad or high-risk one. The root keeps the decision.
- Ask me before using Fable (any version), GPT-6 Luna, or GPT-5.6 Sol; you may suggest them.

# Skills

- When a rule names a skill that is not in your skills list, read `~/.agents/skills/<name>/SKILL.md`.
- Reuse skill instructions that are already present in the active conversation context across user turns. Do not reread the same `SKILL.md` or its required references only because a new user turn started.
