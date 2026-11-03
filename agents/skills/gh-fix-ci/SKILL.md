---
name: "gh-fix-ci"
description: "Diagnose or fix failing GitHub Actions checks on a pull request. Use for CI failures; report external checks without attempting to repair them."
---


# GitHub Actions CI Fix

## Overview

Use this skill when the task is specifically about failing GitHub Actions checks on a pull request. This workflow is hybrid by design:

- Use the GitHub app from this plugin for PR metadata, changed files, and review context.
- Use `cmd ship ci-failures` (or `gh`) for GitHub Actions checks and logs because the connector does not expose that workflow end to end.
- A request to fix CI authorizes diagnosis, scoped local repairs, and relevant local checks. Briefly state the cause and continue with that work. A diagnosis-only or plan-only request does not authorize implementation.
- Ask only when missing information changes the repair scope or an action needs authority not already granted. Keep commits, pushes, workflow reruns, and other external writes within the user's existing authorization.

Prereq: authenticate with GitHub CLI once, then confirm with `gh auth status`. Repo and workflow scopes are typically required for Actions inspection. `cmd ship ci-failures` reads its token from `GITHUB_TOKEN`, `GH_TOKEN`, or `gh auth token`.

## Inputs

- `pr`: PR number or URL (resolve the current branch PR first when none is given)
- `repo`: `owner/name`, needed only with a PR number outside the current checkout
- `gh` authentication for the repo host

## Quick start

- `cmd ship ci-failures <number-or-url>`
- Add `--json` if you want machine-friendly output for summarization.

## Workflow

1. Verify gh authentication.
   - Run `gh auth status` in the repo.
   - If unauthenticated, ask the user to run `gh auth login` (ensuring repo + workflow scopes) before proceeding.
2. Resolve the PR.
   - If the user provides a PR number or URL, use that directly.
   - Otherwise prefer the current branch PR with `gh pr view --json number,url`.
   - When repo and PR are known, fetch PR metadata and patch context through the GitHub app from this plugin.
3. Inspect failing checks (GitHub Actions only).
   - Preferred: run `cmd ship ci-failures <number-or-url>`. It reads the head commit's checks, fetches the log of each failed Actions job, and prints a snippet around the failure.
     - Add `--json` for machine-friendly output, `--repo owner/name` for a PR number outside the current checkout, and `--max-lines`/`--context` to resize snippets.
   - Manual fallback:
     - `gh pr checks <pr> --json name,state,bucket,link,startedAt,completedAt,workflow`
       - If a field is rejected, rerun with the available fields reported by `gh`.
     - For each failing check, extract the run id from `detailsUrl` and run:
       - `gh run view <run_id> --json name,workflowName,conclusion,status,url,event,headBranch,headSha`
       - `gh run view <run_id> --log`
     - If the run log says it is still in progress, fetch job logs directly:
       - `gh api "/repos/<owner>/<repo>/actions/jobs/<job_id>/logs" > "<path>"`
4. Scope non-GitHub Actions checks.
   - If `detailsUrl` is not a GitHub Actions run, label it as external and only report the URL.
   - Do not attempt Buildkite or other providers; keep the workflow lean.
5. Summarize failures for the user.
   - Provide the failing check name, run URL (if any), and a concise log snippet.
   - Call out missing logs explicitly and do not over-claim certainty.
6. Select a focused repair from the observed root cause.
   - Keep it tied to the failing checks. For a diagnosis-only or plan-only request, report the findings or plan and stop here.
7. Implement the requested fix locally.
   - Preserve unrelated changes and repair failures caused by this change.
   - Run relevant local checks and complete repository-required checks. Repeat or broaden checks only after a change, failure, or unresolved concern.
8. Recheck status and summarize residual risk.
   - Inspect `gh pr checks` when useful. Local success does not mean hosted CI passed on the new code; report which revision each result covers.
   - Report what is still unverified, what may still be flaky, and whether any failing checks were external and therefore not actionable here.

## `cmd ship ci-failures`

Report a PR's failing checks on its head commit, pull each failed GitHub Actions job's log, and extract a failure snippet. It exits 1 when any check failed and 0 otherwise, so it can be used in automation; an error, such as an unknown PR, also exits 1 with the message on stderr. Its short alias is `cmd sh cif`.

- A failing check is a completed check run that did not succeed, was not skipped, and was not neutral, or a commit status in `failure` or `error`. Runs that a newer run of the same workflow replaced, and runs of another PR on the same commit, are left out.
- Each result has a `status`: `ok` with `logSnippet` and `logTail`, `external` for a check outside GitHub Actions (only its `detailsUrl` is reported), `log_unavailable` with an `error`, or `no_jobs` for a run that failed without a failing job, such as on an invalid workflow file.
- The snippet centers on the last `##[error]` line of the job log, else the last line with a failure marker, else the log's tail. Timestamps and color codes are stripped.
- `pending` lists checks and runs still going, which the report cannot judge yet.

Usage examples:
- `cmd ship ci-failures 123`
- `cmd ship ci-failures https://github.com/org/repo/pull/123 --json`
- `cmd ship ci-failures 123 --repo org/repo --max-lines 200 --context 40`

## Guardrails

- Do not imply that the GitHub app can replace `cmd ship ci-failures` or `gh` for Actions log retrieval.
- Treat non-GitHub Actions providers as report-only unless the user explicitly wants a separate investigation path.
- If the failure is clearly unrelated to the local diff, say so before proposing code changes.
