---
name: correct
description: Find the mistakes agents keep repeating in the current repository and make each one impossible, preferring architecture, then types, then a lint or check whose error names the fix, then a behavior test, and agent instructions last. Use when the user invokes $correct or /correct. Use `reflect` for lessons about skills and workflow instead of the repository.
disable-model-invocation: true
---

# Correct

The user keeps correcting agents in this repository for the same mistakes. Change the repository so the next agent cannot make them.

Assume every contributor is an agent that sees only the files it opened, copies the nearest example, and takes the shortest path that compiles. Make a change that looks right from one file be right for the whole repository.

## Find the mistake classes

Read the evidence before choosing fixes:

- the user's corrections in local agent history for this workspace: Claude Code transcripts under `~/.claude/projects/` and Codex sessions under `~/.codex/sessions/`; confirm each session's working directory matches before using it
- recent commits, reverts, and review comments
- project `AGENTS.md`, `CLAUDE.md`, and skill rules, especially rules the user restated after they were already written down
- comments that explain workarounds

Group the mistakes into classes. A class counts once it has happened twice. Rules that already exist and are still violated are the strongest signal: written instructions are not enforcing them.

## Fix each class at the highest level that works

1. **Remove it with architecture.** Give each piece of state one owner and each task one supported way. Hide internals so the wrong import fails. Replace hand-synced lists with one source of truth. Delete old paths and dead code an agent would copy.
2. **Enforce it with types** so the bad state cannot be written. If bad code still compiles, add a lint, clippy rule, or CI check whose error names the file, type, or function to use instead. If the pattern is already common, fail only when a change adds more.
3. **Test the behavior.** Fix or delete any test that would still pass if every function it calls returned nothing.
4. **Write agent instructions last,** only for judgment calls. Nothing fails when an agent skips them.

## Fix and prove

Fix the most frequent classes now, one logical change each. Prove each new check fails on a real past mistake by running it against that mistake. Run the same command locally that CI runs. An exception goes on the offending line with a reason, an expiry date, and the user's approval.

Follow the repository's formatter, linter, and test commands. Commit only when the user asked for commits, one commit per class.

## Keep the rule table

Keep a table in the repository's agent instruction file that pairs each rule with what enforces it. When the user corrects an agent later, fix the mistake and add the rule. If the rule was already there with nothing enforcing it, that is a repeat: fix it at the highest level in the same change. Drop a rule once its mistake can no longer happen.

## Reply

List each class with its evidence, the level you chose, why a higher level did not work, and the proof that the new check fails on the past mistake.
