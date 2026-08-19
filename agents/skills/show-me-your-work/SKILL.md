---
name: show-me-your-work
description: Keep an evidence-linked, append-only decision log for long-running, multi-phase, experimental, or unattended work. Use when the user wants a reviewable trail, asks the agent to work while away, or needs to understand why a run changed direction. Do not use for short tasks with no meaningful decisions.
---

# Show me your work

Keep one decision log that lets a reviewer reconstruct what changed, why, and what proved the result.

## Start the log

Use `_scratch/decision-logs/<task-slug>.tsv` from the repository root. Copy [decision-log-template.tsv](references/decision-log-template.tsv) or let the helper create the header.

The columns are:

- `ts`: UTC timestamp
- `phase`: the work phase or experiment
- `decision`: the selected action or conclusion
- `why`: the concrete reason
- `evidence`: a resolvable path, command result, commit, pull request, trace, or screenshot
- `result`: a verified outcome or an explicit open state

Use [log.sh](scripts/log.sh) to append a safe row:

```sh
<skill-dir>/scripts/log.sh <logfile> <phase> <decision> <why> <evidence> <result>
```

The helper removes tabs and line breaks from cells and prevents spreadsheet formula execution.

## Log decisions, not activity

Add one row when the work chooses a path, accepts or rejects an experiment, completes a verified unit, changes direction, or becomes blocked. Do not log routine file reads, commands, or commentary.

Use these result values when they fit:

- `verified`
- `not verified`
- `inconclusive`
- `reverted`
- `blocked`
- `open`

Append a correcting row when an earlier decision was wrong. Do not edit history to make the run look cleaner.

## Keep evidence real

Each evidence cell must resolve and support the row. Prefer evidence created by repeatable scripts or normal repository checks. An agent report is not proof of the artifact it describes.

Keep the log under `_scratch/` by default. If the user asks for a committed or shared audit trail, move or copy the completed log to a suitable tracked project location after removing private data. Do not publish or commit it without the required authority.

## Close the run

Compare the log with the actual diff, commands, and artifacts. Add missing pivots or failed experiments. Mark unresolved work honestly and remove rows that describe intentions without actions.

Return the log path, the important decisions, unresolved results, and the evidence that proves completion.
