---
name: goal-prompt
description: Draft a compact, paste-ready Codex goal prompt with a measurable outcome, evidence surface, constraints, boundaries, iteration policy, and honest blocker handling. Use only when the user explicitly invokes $goal-prompt or /goal-prompt.
---

# Goal Prompt

Draft one paste-ready prompt that starts with `Set your own goal to`.

Include:

- the concrete outcome
- the tests, commands, artifacts, sources, or audit evidence that prove completion
- binding constraints and scope boundaries
- the allowed files, tools, repositories, data, or resources when relevant
- an iteration policy that inspects current evidence, takes the smallest useful next action, and reruns focused checks
- a blocker condition that reports attempts, evidence, the blocker, and the input needed without claiming success

Ask one focused question only when missing information would make completion unverifiable or materially change scope. Otherwise infer a compact prompt from the request.

For research goals, require claims to map to evidence and distinguish confirmed findings, proxies, blocked claims, and uncertainty.

Return only the paste-ready prompt. Do not add a heading, rationale, checklist, or missing-details section.
