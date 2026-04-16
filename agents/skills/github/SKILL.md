---
name: github
description: >
  Use when the user wants to do GitHub work with the `gh` CLI or the GitHub API
  directly, especially for issues, pull requests, labels, comments, projects, and
  other operations that may not be covered by higher-level tools. Prefer this skill
  when the task is specifically about how to do something on GitHub with `gh` or
  `gh api`, or when connector tools do not expose the needed operation.
metadata:
  author: Praveen Perera
  version: "0.0.1"
---

# GitHub

This skill is a navigation file for GitHub workflows implemented with `gh` and
`gh api`. Keep this file lean and load the specific guide that matches the task.

## When to use this skill

Use this skill when the user wants to:

- perform GitHub operations with `gh`
- call the GitHub REST or GraphQL API through `gh api`
- understand when `gh` subcommands are enough versus when to drop to raw API calls
- handle GitHub features that are missing from higher-level connector tools

## Working rules

- prefer `gh` subcommands when they cleanly support the operation
- use `gh api` when GitHub only exposes the feature through the API or when `gh`
  does not have a higher-level command
- keep commands non-interactive so they are safe for agents to run
- verify auth with `gh auth status` before debugging API behavior
- when an API field must be numeric, boolean, or null, prefer `-F/--field` over
  `-f/--raw-field`

## Guides

- Link sub-issues to parent issues:
  [references/link-sub-issues.md](references/link-sub-issues.md)
