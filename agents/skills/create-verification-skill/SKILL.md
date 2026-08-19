---
name: create-verification-skill
description: Create a project-local skill that launches and drives a real application, captures evidence, and maps its user-visible features. Use when a repository lacks a repeatable way for agents to verify UI, CLI, desktop, mobile, or service behavior. Do not use for a library whose existing tests already exercise its public behavior.
---

# Create a verification skill

Create `.agents/skills/verify-<app>/` as the maintained way to prove real application behavior. Use `skill-creator` to build and validate it.

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
