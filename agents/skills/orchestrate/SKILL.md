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

- Context is the root's job. Never put context sizes, limits or budgets in a brief, and never tell a delegate to stop or hand off because of its context. A delegate that knows its limit stops early to hand off, and every handoff re-reads the same setup. The root's watches compact delegates (see "Context watches and compaction"), and the root decides when to replace one.
- When a delegate grows large, the root may queue a compact for it, or steer it. To queue a compact, run `homebased --json thread compact <ref>`. It compacts an idle or waiting delegate at once, and one running a turn answers `queued` and compacts when that turn ends. To steer it, tell it what to do, such as "finish the current item, update your progress file, and report". Neither one tells the delegate about its context.
- Give each delegate one coherent phase. Do not split a phase only to make it smaller. Hand the next phase to a fresh delegate along with the state file and the previous report, and not to the old delegate with all its history. A follow-up of a few turns on the same change can stay with the same delegate.
- The implementer owns its review loop. Its brief lets it start one fresh review, fix the findings, run delta reviews within the fix budget, and report the final verdict. The root does not pass findings back and forth between reviewer and implementer.
- How often a delegate has compacted is never a reason to replace it. Replace a delegate only when it is confirmed dead (see "Stalled or dead delegates"), or when it repeats the same disproved action, or breaks the same stated invariant, after one correction and with no new evidence. Before a replacement for repetition, record both occurrences and the correction in the state file. A failed test or a valid wait is not a reason. To replace one, `task_cancel` the old run first, then start a fresh delegate from the state file, its last report and its progress file (see the brief template), which holds what it finished.

## Stalled or dead delegates

- When a delegate reports `blocked`, act on the reason it gives. If the block is a decision, answer it with `t3_thread_send`. If it is blocked on credentials, a resource or a dependency, resolve the block, then tell it to continue. Do not replace it.
- A delegate has stalled when it has no final report and no live work. The stall watch (below) wakes the root with `THREAD_STALLED` when one sits idle with nothing due to wake it, so do not write a polling script for this. On that event, or when you suspect a stall, check it once with `homebased --json thread context --children --live`, or with `t3_thread_read` using a small `limit` and `runLimit` 1. If it is confirmed dead, or meets the replacement rule in "Keep delegates short-lived", replace it as that rule says. A delegate that waits on credentials, a resource or a dependency has not stalled; resolve the block instead. Do not nurse a delegate with repeated messages.

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

- Context limits:

  | Thread | Default compact watch | Root may adjust within |
  |---|---|---|
  | Root | 200K | 150–250K |
  | Claude delegates: implementation, research, review | 250K | 200–300K; up to the model's ceiling only to let a delegate finish an item that is nearly done |
  | Codex delegates | none | they compact themselves at about 210–270K |
  | Agent-tool subagents, such as Explore | none, since they are not T3 threads | scope them to finish well under 150K, and under 100K on Haiku, which costs 5x above 100K |

  Why these numbers (from six weeks of usage data in `~/code/research/agents/context-size-vs-token-cost-2026-11.md`): cache reads are almost all input tokens, so each extra 100K adds about $0.02 to every Opus call. A compaction costs about $0.50 and pays for itself within 10 to 15 calls. A fresh delegate costs about $1.27 before its first edit. Compacting at 250K is cheaper than both handing off at 150K and running to 400K. The data shows little loss of quality at these sizes.

  Each model's ceiling is its Claude Code auto-compact window in `~/.claude/settings.json`: 325K for Opus 5.5, and 256K for Sonnet 5.5 and Haiku 5.5. Claude Code compacts a thread mid-turn when it reaches that size, so a watch at or above it never fires. Keep every watch below the ceiling of the thread's model. A homebased watch compacts a thread only while it is idle or waiting, so it compacts at a turn boundary before the ceiling forces a compaction mid-turn.

  Reviews and other subagents that a delegate starts itself get no context watch, because the root does not learn their thread ids. That is deliberate. They are usually Codex, and Claude reviews rarely grow large enough to compact.

- Set the root's watch on its own thread at the start:

  ```bash
  homebased --json thread watch --context-above 200k --action compact
  ```

  Give each Claude delegate its own watch when you start it, using the child thread id that `delegate_task` returns, or the reference of a top-level thread you launched to coordinate:

  ```bash
  homebased --json thread watch <child-thread-id> --context-above 250k --action compact
  ```

  Do not use `--children` for context watches. It covers the calling thread and every live subagent, Codex delegates included, with no model filter. Running it again after a new delegate also adds a 250K watch on top of any limit you have already raised. Per-thread watches keep each limit where you set it, and leave Codex delegates alone. Running the same watch again reuses it instead of adding a second one.

  To change one thread's limit, run `homebased thread unwatch <watch-id>` on its current context watch, then run `homebased --json thread watch <ref> --context-above <limit> --action compact`. A watch with another limit on the same thread is a second watch, not a replacement, and the lower limit still fires first. Keep the root within 150–250K. Record each change and its reason in the state file's settled decisions, and each watch id in its operations block.

  Watch the same threads for stalls, again after starting new delegates:

  ```bash
  homebased --json thread watch --children --stalled-after 20m
  homebased --json thread watch <ref>... --stalled-after 20m   # top-level threads
  ```

  It sends `THREAD_STALLED` once a thread has sat idle for 20 minutes with no turn running, no homebased task or job owing it events, and no delegated task due to report to it. A delegate's stall watch ends on its own when its task finishes. A top-level thread looks the same after its final report as when it stalls, so run `homebased thread unwatch <watch-id>` when that report arrives.

  The context watches compact a thread once it sits idle or waiting above its limit, without waking the root. The root hears about it only when the compaction cannot start. Do not add a notify-only watch (no `--action`): each one wakes the root for a full turn, and that turn ends up running the same compaction by hand. Context size is never a reason to replace a delegate (see "Keep delegates short-lived"). Record the watch ids in the state file. `homebased thread watches` lists them, and `homebased thread unwatch <watch-id>` ends one.
- Rewrite the state file at every phase boundary, so a compaction at any time loses nothing. A root compaction wakes the root only when no delegate, task or job is still due to report; otherwise the next report resumes it. To compact the root itself, run `homebased --json thread compact` with no reference: mid-turn it answers `queued`, and T3 compacts the root once the turn ends. Nothing wakes the root after a queued compaction, so use it only when a delegate, task or job is still due to report. Otherwise run `homebased --json thread watch --context-above <limit> --action compact --once` with a limit a few thousand tokens below the root's current size and end the turn; the watch compacts the root and then wakes it.
- `homebased --json thread context --children --live` lists the live delegates with their state, model and context size. Use it to rebuild the delegate list in the state file and to check whether a delegate you suspect has stalled is still live.
- Compact, cancel, message or replace only threads in the root's own tree: the root, the threads it started, and their descendants. Before you act on a thread, confirm that it is in that tree with `homebased --json thread context --children --live` or the owners table in the state file. Do not act on a thread only because it is waiting or large. A compaction that started cannot be undone. Do not compact a thread again soon after it compacted, because that loses detail and saves little.
- After compaction, re-read the state file before anything else. Take ids and commands from its operations block instead of grepping the transcript or rerunning `--help`. If something you needed was missing, add it to the block.

## PR stacks

When the run coordinates stacked PRs, read [stacks.md](references/stacks.md).
