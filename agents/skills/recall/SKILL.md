---
name: recall
description: Reconstruct recent work on a named topic from local Codex conversation history, repository state, and linked work records. Use for catch-up, recent-work summaries, resuming a topic across chats, or finding where the user left off. Do not use when a supplied handoff already contains current verified state.
---

# Recall

Build a current-state brief from history. A transcript records what happened. Live state determines what remains true.

## Set the boundary

Default to the active workspace and the last seven days. Use the topic, workspace, and time range that the user gives. Never search another workspace without a clear request.

Route one known session to the product's resume mechanism when available. Use `handoff` when the task is to prepare future resumption. Use this skill when context must be rebuilt across records.

## Search local Codex history

Use these sources in order:

1. `~/.codex/history.jsonl` for a cheap prompt index
2. matching files under `~/.codex/sessions/` for the full record
3. `~/.codex/archived_sessions/` only when the active history does not cover the requested period

Read session metadata before using a match. Confirm that its `cwd` or workspace roots match the requested workspace. Search terms first, then read only the relevant message regions. Exclude the current session, generated test sessions, and unrelated subagent runs when possible.

Capture the user's goal, decisions, corrections, open work, commands, branches, pull requests, and durable artifacts. Do not treat an agent's stated result as proof.

## Check current state

Verify surfaced branches, commits, pull requests, files, processes, or tickets with the relevant read-only tool. If a named feature or bug has a shared record, inspect the available source that can change the answer, such as GitHub, an issue tracker, project documents, or observability. Report unavailable sources as gaps only when they matter.

## Return a short brief

- **Capsule:** at most five bullets that state the topic and current state
- **Threads:** one line per active or completed thread, with a verified status
- **Problems:** at most five recurring failures, reversions, or unknowns
- **Next move:** one concrete action

Cite local sessions by timestamp or session ID and shared records by their stable identifier. Keep private conversation details out of public artifacts.
