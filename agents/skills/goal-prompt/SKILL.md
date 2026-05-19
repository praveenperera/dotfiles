---
name: goal-prompt
description: Use when the user wants to create, improve, or sanity-check an arbitrary Codex goal without creating a full goal-ready spec. Helps draft compact goals with measurable outcomes, evidence surfaces, constraints, boundaries, iteration policy, and blocked stop conditions.
---

# Goal Prompt

## Purpose

Create compact, paste-ready Codex goal prompts without creating a file-backed `_plans/` spec.

Use this skill when the user wants an arbitrary goal, goal wording, goal review, or a lightweight goal prompt. Do not use it when the user asks for a durable implementation spec, explicitly invokes `$goal-ready-spec`, or needs a repo-local plan folder.

## When To Use A Goal

Use a goal when the task has:

- a durable objective that may take multiple turns
- an evidence-based finish line
- an uncertain path where Codex should keep iterating

Prefer a normal prompt for one-line edits, simple explanations, short reviews, or vague requests without a measurable completion condition.

## Goal Checklist

A strong goal defines:

- Outcome: what should be true when done
- Evidence Surface: tests, commands, reports, artifacts, source material, or audit evidence that proves completion
- Constraints: requirements, style rules, architecture, scope limits, and repo instructions that must be preserved
- Boundaries: files, tools, repositories, data, or resources Codex may use or must avoid
- Iteration Policy: how Codex chooses the next action after failed checks, partial progress, or compaction
- Blocked Stop Condition: when Codex should stop and what evidence, attempts, blocker, and requested input it should report

## Workflow

1. Restate the intended outcome in one sentence.
2. Identify the evidence surface that proves completion.
3. Capture constraints and boundaries from the user's request and local context when relevant.
4. Add an iteration policy that tells Codex to inspect evidence, make the smallest useful next move, rerun relevant checks, and continue.
5. Add a blocked stop condition that prevents false completion.
6. Produce a paste-ready prompt that starts with "Set your own goal to..."

Ask a focused question only if the missing detail would make completion unverifiable or risk changing the user's intended scope.

## Prompt Template

```text
Set your own goal to <desired end state>, verified by <specific evidence>, while preserving <constraints>. Use <allowed inputs, tools, files, repos, or resources> and stay within <boundaries>. Between iterations, inspect the latest evidence, make the smallest useful next move, rerun the relevant checks, and continue until the evidence proves completion. If blocked or no valid paths remain, stop and report attempted paths, evidence gathered, the blocker, and the next input needed.
```

## Research Prompt Template

Use this shape for research, comparison, audit, or investigation goals where the output is a report rather than an implementation:

```text
Set your own goal to produce the strongest evidence-backed answer to <research question> using <available sources or resources>. Build a claim inventory, map important claims to evidence, and end with a report that separates confirmed findings, approximate or proxy-supported findings, blocked claims, and remaining uncertainty. Between iterations, follow the strongest unresolved claim or weakest evidence gap. If exact proof is unavailable, label the limitation instead of overclaiming. If blocked, stop and report attempted sources, evidence gathered, the blocker, and the next input needed.
```

## Output Format

Return:

- Goal Prompt: paste-ready text starting with "Set your own goal to..."
- Why This Works: one short paragraph explaining the outcome, evidence, constraints, boundaries, iteration policy, and blocked stop condition
- Missing Details: only include this section when the prompt depends on unresolved information
