# State file

The state file is the root's memory across compactions. Keep it under about 150 lines and rewrite it in place. Move history to `JOURNAL.md` in the same directory, which the root appends to and never needs to read back.

## Run directory

Give each run its own directory, `_scratch/<short-task>/`, named for the task in two or three kebab-case words, such as `_scratch/auth-migration/`, so a new run never overwrites another run's files. If that directory already belongs to another run, pick a different name. The run's files go in it: `TASKS.md` (the state file), `JOURNAL.md`, the PR bar file and per-agent files on the root's machine. Record the directory's absolute path at the top of the state file and in every brief, so the root finds it again after a compaction and agents never guess it.

These files are temporary. `_scratch/` is wiped weekly, so a run directory there needs no cleanup. If a run had to put its files somewhere else, delete them when the run ends. Either way, copy anything worth keeping, such as findings and decisions, to `~/code/research` before the run ends.

## Rules

- Rewrite sections; do not append to them. When an item is done, move it to the journal instead of leaving a ticked box in the file.
- One line per settled decision. When a decision replaces an earlier one, delete the earlier line and record the change in the journal, so the file never holds two answers to the same question.
- Record every id the moment it exists: T3 subagent task ids, child thread ids with their machine and parent, homebased task and job ids, wait task ids, scheduled task ids, and PR numbers.
- Record each command in the exact form that worked, so nobody has to rediscover it with `--help` or a transcript search.
- Write the RULES section once. Briefs link to it.
- If the file grows past 150 lines, move detail into the journal or into a per-agent file and link to it.

## Template

```markdown
# <task>: root state

Re-read this after any context compaction, before anything else. Times are US Central.

Run directory: <absolute path, such as /home/me/code/app/_scratch/auth-migration>

## Goals
<!-- One row per goal the user set, in their words. Every status reports against this table; update Standing each time -->
| Goal (user's words, time set) | Done looks like | Standing | Next decisive check |
|---|---|---|---|
| <goal> | <measurable done condition> | on track / at risk / off track: <one plain-language reason> | <the check that settles it, and when> |

- Deadline: <if any>
- Out of scope: <...>

## Owners
| Item | Owner (T3 subagent task id, or child thread id and machine) | Model | State |
|---|---|---|---|

## Rules
<!-- Standing rules from the user and the run: safety limits, resources and who owns them, review and CI bar, commit rules. Briefs link here -->

## Corrections already made
<!-- One line each: what was wrong, what is right, which agent it applies to -->

## Settled decisions
<!-- One line each: time, decision, reason. Replace instead of appending overrides -->

## Operations
- Live T3 subagents: <task id, thread id, scope>
- Child threads: <thread id, machine, link, parent (the root or another child thread), scope, reported or not>
- Live waits and jobs: <homebased task and job ids, wait task id or sleep, what each outcome means>
- Watches and schedules: <scheduled task id, context watch ids from `homebased thread watches`>
- Commands: <exact forms that worked>

## Progress
- Formula: <how % done is computed, such as items met over items planned>
- Latest (<time>): <the last status's bottom line and goal lines, replaced each time>

## Next
- [ ] <open item, with owner and next step>
```
