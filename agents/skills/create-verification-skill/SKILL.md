---
name: create-verification-skill
description: Create a durable project-local skill that drives a real application and captures evidence. Use when the user asks for a reusable verification workflow, or the same missing harness blocks repeated tasks and its creation is in scope. Do not use for one failed tool, ordinary tests, one-off verification, or libraries covered by tests.
---

# Create a verification skill

Create `.agents/skills/verify-<app>/` as the maintained way to prove real application behavior. Use `skill-creator` to build and validate it.

## Confirm the need

Do not start this workflow in the middle of an implementation only because one verification tool is temporarily unavailable. First use the repository's existing harness or report the specific live-evidence gap.

Proceed only when the user requested a reusable verification skill, or when the same missing verification path has blocked at least two separate tasks and the current task permits adding maintained project tooling. If neither condition is true, keep verification within the active task.

## Learn the application from the repository

Inspect the code and project instructions before asking the user:

- **Surface:** what users operate, including secondary surfaces that affect the main one
- **Launch:** the documented local command, required configuration, ports, data, and authentication
- **Drive:** existing UI tests, browser controls, app automation, PTY helpers, HTTP endpoints, or debug interfaces
- **Observe:** screenshots, screen recordings, terminal output, logs, responses, files, and persisted state
- **Isolate:** ports, profiles, data directories, accounts, or devices needed to avoid the user's live state

Prefer an existing harness. Use the applicable browser, computer-use, mobile, or application skill when UI control requires it. Do not invent selectors or commands that the repository does not support.

If the application cannot start from the current checkout, report or fix that problem within the user's authority before writing instructions that assume it works.

## Generate the project-local skill

The new skill must contain exact, tested instructions for:

- **Launch:** start the intended build and detect readiness
- **Doctor:** confirm that the instance is healthy, is the expected build, and is safe to drive
- **Drive:** operate real user paths through stable selectors, commands, or protocol inputs
- **Evidence:** capture the action and result under `_scratch/verification/<run>/`
- **Cleanup:** stop only the processes and temporary state created by the run while preserving evidence

Do not use test-only setters as proof of a user path. Verify visible results and important side effects. If a dry-run mode is required, observe what it actually changes.

## Seed the feature map

Create `features/README.md` and one Markdown file for each of the three to five most important user-visible features. Each feature file states:

- what the feature does for the user
- how the user reaches it
- how the harness drives it
- the observable result that proves it works
- prerequisites and known limits

Use source paths as evidence for the map. Do not turn the feature map into a file inventory.

## Prove the skill

Run the generated workflow end to end for one mapped feature. Launch, run doctor, drive the feature, capture evidence, clean up, and confirm that the evidence remains. Fix the skill when any step fails, then validate it with `skill-creator`'s validator.

Return the skill path, the verified feature, the evidence path, remaining coverage gaps, and the command or skill that maintains it.
