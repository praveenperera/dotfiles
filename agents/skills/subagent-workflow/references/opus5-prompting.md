# Opus 5 prompts

Use the short task contract in [SKILL.md](../SKILL.md). Provide the complete objective and relevant code, tests, or visual references. Keep scope and completion conditions explicit; let Opus choose routine steps.

Current provider guidance supports complex coding and bug finding. It also warns about scope growth, excessive verification, frequent delegation, and long responses. Apply only the correction needed by the task:

- Keep requested behavior complete and leave unrelated improvements for follow-up
- Run required checks without adding generic repeated verification steps
- Delegate only substantial independent work; keep nested delegates disabled in this workflow
- Ask for concise progress and final reports when narration becomes excessive
- Review actual behavior and coverage, not a claim of completion

Keep code style in repository instructions and local examples. Do not copy the full workflow into a delegate prompt or require a separate document for each stage. Use `high` by default; change effort only under the workflow's user-choice and tuning rules.

For a bug review, ask for evidence-backed actionable findings. An instruction to report only the most severe issues can hide relevant defects. Set the actual review scope and severity requirements from the user's request.

Use [frontier-prompting.md](frontier-prompting.md) for Fable 5.1 instead of inheriting Opus-specific behavior assumptions.

Source: [Anthropic Opus 5 prompting](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5).
