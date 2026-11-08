# State file

The state file is the root's memory across compactions. Keep it at `_scratch/<task>/TASKS.md`, under about 150 lines, and rewrite it in place. Move history to `_scratch/<task>/JOURNAL.md`, which the root appends to and never needs to read back.

## Rules

- Rewrite sections; do not append to them. When an item is done, move it to the journal instead of leaving a ticked box in the file.
- One line per settled decision. When a decision replaces an earlier one, delete the earlier line and record the change in the journal, so the file never holds two answers to the same question.
- Record every id the moment it exists: delegate task ids, child thread ids, homebased task and job ids, wait task ids, scheduled task ids, and PR numbers.
- Record each command in the exact form that worked, so nobody has to rediscover it with `--help` or a transcript search.
- Write the RULES section once. Briefs link to it.
- If the file grows past 150 lines, move detail into the journal or into a per-delegate file and link to it.

## Template

```markdown
# <task>: root state

Re-read this after any context compaction, before anything else. Times are US Central.

## Goal
- <goal and done condition, with the user's words and time>
- <deadline, if any>
- Out of scope: <...>

## Owners
| Item | Owner (delegate task id / thread id) | Model | State |
|---|---|---|---|

## Rules
<!-- Standing rules from the user and the run: safety limits, resources and who owns them, review and CI bar, commit rules. Briefs link here -->

## Corrections already made
<!-- One line each: what was wrong, what is right, which delegate it applies to -->

## Settled decisions
<!-- One line each: time, decision, reason. Replace instead of appending overrides -->

## Operations
- Live delegates: <task id, child thread id, scope>
- Live waits and jobs: <homebased task and job ids, wait task id, what each outcome means>
- Watches and schedules: <scheduled task id, context watch ids from `homebased thread watches`>
- Commands: <exact forms that worked>

## Progress
- Formula: <how % done is computed, such as items met over items planned>
- Latest (<time>): <the last status, replaced each time>

## Next
- [ ] <open item, with owner and next step>
```
