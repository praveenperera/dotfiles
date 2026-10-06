---
name: why
description: Find evidence for why code, an architecture, a threshold, or a product decision has its current shape. Use when the user invokes $why or /why for design rationale, regressions, postmortems, or historical decisions. Use `how` for mechanics without motivation.
disable-model-invocation: true
---

# Why

Recover rationale from evidence. Do not turn the current code shape into a story about its intent.

## Anchor the question

Identify the exact files, symbols, behavior, change, or decision in question. If the target is vague, state your best reading from the conversation and proceed. Establish the mechanics with a focused code trace when they are unclear. Do not invoke `how` only because this skill is active.

Build the anchor before searching wider: file paths and line ranges, key symbols, the last commits that touched the target from `git blame -L` and `git log --follow`, and pull request numbers from those commits.

## Search the sources that can answer

Start with source control, because it connects rationale to the change that shipped:

- current code, tests, and comments
- `git log`, `git blame`, commits, pull requests, and review threads through `gh`

Then search the records that can change the answer. Much of this user's rationale lives in agent conversations rather than in pull requests, so check local agent history when the change was made with an agent:

- Claude Code transcripts under `~/.claude/projects/` and Codex sessions under `~/.codex/sessions/`; match by workspace and by the commit's date range, and confirm that a session's working directory matches before using it
- plans and specs under the repository `_plans/` directory, and notes in `~/code/research`
- issues and project trackers for product or operational pressure
- observability, error tracking, or analytics when the code reacts to a runtime signal or a number

A source that is unavailable is a gap. A relevant search with no result is a finding. Do not run a large sweep for a narrow question whose answer is already explicit in a commit or pull request.

For a broad question with several independent sources, send one read-only GPT-6.1 Sol `high` investigator per source through the `use-agents` skill and keep the synthesis in the root. Give each investigator the anchor, the question, and the confidence tiers below.

## Weigh the result

Classify every claim:

- **Direct:** a source states the reason; cite it next to the claim
- **Supported:** several facts point to a reason that no source states; list them
- **Inferred:** a reasonable reading with nothing explicit behind it; show the chain of reasoning and hedge the wording
- **Unknown:** the record does not support an answer; say what you searched and with which terms

Use words like "because" and "was designed to" only for direct or supported claims. Give stable citations such as commit hashes, pull request numbers, ticket IDs, session timestamps, document paths, and file symbols. Do not cite code behavior as proof of its own motivation. Do not assume the most recent commit explains the current shape; trace back to the change that introduced it.

## Return

State the answer first. Then give the direct evidence, supported inferences, competing explanations when they matter, the sources consulted with one line each (including empty and skipped ones), and material gaps.

If the user plans to change the code, finish with what the evidence says to preserve, what is safe to change, what to avoid, and the main risk.

This is a read-only skill. Do not change external records or code unless the user separately asks for that action. Keep private conversation details out of public artifacts.
