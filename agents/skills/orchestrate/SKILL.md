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
- When a delegate grows large, the root may queue a compact for it, or steer it. To queue a compact, run `homebased --json thread watch <ref> --context-above <current size> --action compact --once`, which compacts it the next time it is idle or waiting. To steer it, tell it what to do, such as "finish the current item, update your progress file, and report". Neither one tells the delegate about its context.
- Give each delegate one coherent phase. Do not split a phase only to make it smaller. Hand the next phase to a fresh delegate along with the state file and the previous report, and not to the old delegate with all its history. A follow-up of a few turns on the same change can stay with the same delegate.
- The implementer owns its review loop. Its brief lets it start one fresh review, fix the findings, run delta reviews within the fix budget, and report the final verdict. The root does not pass findings back and forth between reviewer and implementer.
- Replace a delegate that has compacted twice and still has much left to do: ask it for its report, then start a fresh one from that report. Its progress file (see the brief template) holds what it finished.

## Stalled or dead delegates

- When a delegate reports `blocked`, act on the reason it gives. If the block is a decision, answer it with `t3_thread_send`. If not, replace the delegate.
- A delegate has stalled when it has no final report and no live work. The stall watch (below) wakes the root with `THREAD_STALLED` when one sits idle with nothing due to wake it, so do not write a polling script for this. On that event, or when you suspect a stall, check it once with `homebased --json thread context --children --live`, or with `t3_thread_read` using a small `limit` and `runLimit` 1. If it is dead or looping, `task_cancel` it and start a fresh delegate from the state file and its last report. Do not nurse it with repeated messages.

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

  | Thread | Default compact watch | Range |
  |---|---|---|
  | Root | 225K | adjust it from what you see, up to 400K |
  | Delegates | 300K | lower it, or raise it up to 400K, per delegate |

  400K is the ceiling. Claude Code's own auto-compact window is set to 400K, and it compacts a Claude thread mid-turn when the thread reaches that size. A homebased watch compacts a thread only while it is idle or waiting, so it compacts at a turn boundary before the ceiling forces a compaction mid-turn. Codex delegates manage their own context, and these limits do not apply to them.

- Set these watches from the root's own thread at the start, and run the second again after starting new delegates. `--children` also covers the calling thread, but the root's 225K watch fires first. `--children` covers only the subagents that are live when the command runs, and running the same watch again reuses it instead of adding a second one:

  ```bash
  homebased --json thread watch --context-above 225k --action compact
  homebased --json thread watch --children --context-above 300k --action compact
  ```

  `--children` covers only T3 subagents. Threads the root started as top-level threads, such as with `t3_thread_launch` or `homebased t3 launch`, need their references passed explicitly: `homebased --json thread watch <ref>... --context-above 300k --action compact`.

  To change one thread's limit, run `homebased thread unwatch <watch-id>` on its current context watch, then run `homebased --json thread watch <ref> --context-above <limit> --action compact`. A watch with another limit on the same thread is a second watch, not a replacement, and the lower limit still fires first. A delegate still covered by the `--children` watch keeps its 300K limit too, so to raise one delegate above 300K, end the `--children` context watch and give each live delegate its own watch. Record each change and its reason in the state file's settled decisions.

  Watch the same threads for stalls, again after starting new delegates:

  ```bash
  homebased --json thread watch --children --stalled-after 20m
  homebased --json thread watch <ref>... --stalled-after 20m   # top-level threads
  ```

  It sends `THREAD_STALLED` once a thread has sat idle for 20 minutes with no turn running, no homebased task or job owing it events, and no delegated task due to report to it. A delegate's stall watch ends on its own when its task finishes. A top-level thread looks the same after its final report as when it stalls, so run `homebased thread unwatch <watch-id>` when that report arrives.

  The context watches compact a thread once it sits idle or waiting above its limit, without waking the root. The root hears about it only when the compaction cannot start. Do not add a notify-only watch (no `--action`) by default: each one wakes the root for a full turn, and that turn ends up running the same compaction by hand. Add one only for a child whose work you would rather hand to a fresh thread than compact. Record the watch ids in the state file. `homebased thread watches` lists them, and `homebased thread unwatch <watch-id>` ends one.
- Rewrite the state file at every phase boundary, so a compaction at any time loses nothing. A root compaction wakes the root only when no delegate, task or job is still due to report; otherwise the next report resumes it. Do not run `homebased thread compact` on the root's own thread: T3 refuses to compact a thread while its turn is running. Use a `--once` watch instead, as described in "Keep delegates short-lived".
- `homebased --json thread context --children --live` lists the live delegates with their state, model and context size. Use it to rebuild the delegate list in the state file and to spot a delegate that is due for replacement.
- Compact, cancel, message or replace only threads in the root's own tree: the root, the threads it started, and their descendants. Before you act on a thread, confirm that it is in that tree with `homebased --json thread context --children --live` or the owners table in the state file. Do not act on a thread only because it is waiting or large. A compaction that started cannot be undone. Do not compact a thread again soon after it compacted, because that loses detail and saves little.
- After compaction, re-read the state file before anything else. Take ids and commands from its operations block instead of grepping the transcript or rerunning `--help`. If something you needed was missing, add it to the block.

## PR stacks

When the run coordinates stacked PRs, read [stacks.md](references/stacks.md).
