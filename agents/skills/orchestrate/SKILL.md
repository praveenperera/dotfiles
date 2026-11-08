---
name: orchestrate
description: Run a long coordinating thread that drives several delegates, PRs, PR stacks, or queued jobs. Keeps the root's own context small, keeps one state file that survives compaction, briefs delegates so they own their waits and review loops, and reports status in a fixed format. Use when this thread coordinates three or more delegates, PRs, or queue jobs over a run that may outlast a context compaction, when a brief names this skill, or when the user invokes $orchestrate or /orchestrate. Do not use for one delegate whose result the root integrates in the same turn, or for fanout hand-offs.
---

# Orchestrate

Every tool call the root makes re-reads the root's whole context. At 170K, a root that runs builds, edits and polls itself pays for a fresh 170K read on every step, while a delegate doing the same work starts small and then goes away. The root's job is to keep its own context small, to spend tokens in short-lived delegates, and to keep enough state on disk that it can recover after compaction.

[use-agents](../use-agents/SKILL.md) picks the model, effort and runner for each delegate. This skill covers what the root does around them.

## Start

1. Agree the scope before any fan-out: the goal, the done condition, what is out of scope, and who owns what. Ask any questions in one batch. Do not start research sweeps or implementers until the scope is settled.
2. Write the state file from [state-file.md](references/state-file.md) before the first delegate starts.
3. Put the standing rules in its RULES section once. Briefs link to that section instead of restating it.

## Coordinate, don't implement

- The root reads delegate final reports, the diffs it accepts, the state file, and the command output it needs for a decision.
- Delegates do the edits, builds, test loops, CI fixes, log reading and reviews. Send wide reading to `Explore` subagents.
- The root edits its state files and makes integration fixes of a few lines. If the root starts a second edit-build-test cycle on the same problem, it hands that problem to a delegate.
- In-session Agent subagents are for lookups that finish this turn. One that would grow past about 150K context should be a delegate instead.

## Delegates own their waits

- Write each brief from [briefs.md](references/briefs.md). A delegate submits its own long commands and waits on them itself, following [Long commands inside a delegate](../use-agents/SKILL.md#long-commands-inside-a-delegate).
- The root never waits on, polls or relays a child's jobs. Do not call `task_status` on a running child or read its thread in a loop. Its completion wakes the root.
- The root waits only on its own work: delegate completions, which arrive by themselves, and the homebased tasks it submitted.
- A delegate notification that says nothing changed is a defect in the brief. Tell that delegate once how to keep its wait alive, and fix the template for the next delegate.

## Keep delegates short-lived

- Give each delegate one phase that finishes under about 150K context. Hand the next phase to a fresh delegate along with the state file and the previous report, and not to the old delegate with all its history. A follow-up of a few turns on the same change can stay with the same delegate.
- The implementer owns its review loop. Its brief lets it start one fresh review, fix the findings, run delta reviews within the fix budget, and report the final verdict. The root does not pass findings back and forth between reviewer and implementer.
- Replace a delegate that has compacted twice or is past about 250K context: ask it for its report, then start a fresh one from that report.

## Stalled or dead delegates

- When a delegate reports `blocked`, act on the reason it gives. If the block is a decision, answer it with `t3_thread_send`. If not, replace the delegate.
- A delegate has stalled when it has no final report and no live work. Check it once with `homebased --json thread context --children --live`, or with `t3_thread_read` using a small `limit` and `runLimit` 1. If it is dead or looping, `task_cancel` it and start a fresh delegate from the state file and its last report. Do not nurse it with repeated messages.

## Shared resources have one owner

- The root owns disk cleanup, queue order and priority, shared locks, handoffs between delegates, and rebases or merges across PRs. Delegates put candidates and requests in their reports and do not act on them.
- Run independent work now. Hold work back only for a reason, and write that reason in the state file.
- Run the cheapest upstream check before the downstream work that depends on it, and chain the downstream work on its result (homebased `after` for commands).
- Do not hold a scarce resource, such as a GPU lease or a device lock, while doing work that does not need it, such as a build. For GPU work, build in a CPU `homebased task run` and submit the job with `homebased resource job submit --after <build-task-id>`, which keeps the job out of the queue until the build succeeds.

## Decide and log

- Decide reversible calls yourself, and add one line with the time and the reason to the state file's settled decisions. Ask the user only about irreversible or outward-facing calls, or calls the user has kept for themselves, and batch those questions.
- When the user corrects something, record the correction in the state file. Send it only to the delegates it applies to, and name each of them, so no delegate applies a correction meant for another.

## Status

Use this format every time:

- **Done:** a percentage, computed with the progress formula in the state file
- **Left:** each open item, with its owner and next step
- **Time left:** the finish estimate for the last item, and the slack against the deadline when there is one
- **Needs you:** each decision with its options and a recommendation, or "nothing"
- **Issues:** blockers, failures and stalls

Send a status at phase boundaries, on any schedule the user sets, and when asked, but not for every event. Each time, replace the Latest block in the state file.

## Context watches and compaction

- Set these two watches from the root's own thread at the start, and again after starting new delegates. `--children` covers only the subagents that are live when the command runs, and running the same watch again reuses it instead of adding a second one:

  ```bash
  homebased --json thread watch --children --context-above 200k
  homebased --json thread watch --children --context-above 250k --action compact
  ```

  The first sends `THREAD_CONTEXT_OVER` when this thread or a child sits idle above 200K, and the root decides what to do. The second compacts any of them that sit idle above 250K. Record both watch ids in the state file. `homebased thread watches` lists them, and `homebased thread unwatch <watch-id>` ends one.
- Rewrite the state file at every phase boundary. Then check the context with `homebased thread context`, and compact the root with `homebased thread compact` once it is past about 150K.
- `homebased --json thread context --children --live` lists the live delegates with their state, model and context size. Use it to rebuild the delegate list in the state file and to spot a delegate that is due for replacement.
- After compaction, re-read the state file before anything else. Take ids and commands from its operations block instead of grepping the transcript or rerunning `--help`. If something you needed was missing, add it to the block.

## PR stacks

When the run coordinates stacked PRs, read [stacks.md](references/stacks.md).
