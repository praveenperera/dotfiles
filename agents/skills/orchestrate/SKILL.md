---
name: orchestrate
description: "Run a long coordinating thread that drives several subagents, child threads, PRs, PR stacks, or queued jobs, with a state file that survives compaction. Use when this thread coordinates three or more agents, PRs, or queue jobs over a run that may outlast a compaction, when a brief names this skill, or when the user invokes $orchestrate or /orchestrate. Do not use for one subagent whose result the root integrates in the same turn, or for fanout hand-offs."
---

# Orchestrate

Every tool call the root makes re-reads the root's whole context. At 170K, a root that runs builds, edits and polls itself pays for a fresh 170K read on every step, while a subagent or child thread doing the same work starts small and then goes away. The root's job is to keep its own context small, to spend tokens in the agents it starts, and to keep enough state on disk that it can recover after compaction.

[use-agents](../use-agents/SKILL.md) picks the model, effort and runner for each agent, using the agent terms in your instructions. This skill covers what the root does around them.

## Start

1. Agree the scope before any fan-out: the goals, the done condition for each, what is out of scope, and who owns what. Ask any questions in one batch. Do not start research sweeps or implementers until the scope is settled.
2. Write each goal in the state file's Goals table in the user's words. Every status reports against that table, so a goal missing from it never gets reported.
3. Create a run directory named for the task, `_scratch/<short-task>/`, and write the state file in it from [state-file.md](references/state-file.md) before the first agent starts. Its own directory keeps a new run from overwriting another run's files.
4. Put the standing rules in its RULES section once. Briefs link to that section instead of restating it. An agent on another machine reads it over ssh from the root's machine (see [briefs.md](references/briefs.md)).

## Coordinate, don't implement

- The root reads its agents' final reports, the diffs it accepts, the state file, and the command output it needs for a decision.
- Its agents do the edits, builds, test loops, CI fixes, log reading and reviews. Send wide reading to `Explore` subagents.
- The root edits its state files and makes integration fixes of a few lines. If the root starts a second edit-build-test cycle on the same problem, it hands that problem to a T3 subagent or child thread.
- In-session Agent subagents are for lookups that finish this turn. One that would grow past about 150K context should be a T3 subagent or child thread instead.

## Child threads own their long work

- Launch a child thread with `homebased t3 launch --child` for each phase, experiment or long run, and use T3 subagents (`delegate_task`) for short work. Never launch a child thread with `t3_thread_launch`, which records no link to the root.
- Write each brief from [briefs.md](references/briefs.md). A child thread submits its own long work and sleeps on it, and a T3 subagent does no long work, following [Long work](../use-agents/SKILL.md#long-work).
- The root never waits on, polls or relays an agent's tasks or jobs. Do not call `task_status` on a running T3 subagent or read an agent's thread in a loop. A T3 subagent's completion and a child thread's report wake the root.
- The root waits only on its own work: T3 subagent completions and child-thread reports, which arrive by themselves, and the homebased tasks and jobs it submitted.
- Track child threads through their link to the root, not from memory: `homebased --json thread context --children` lists them next to T3 subagents, each with a `child_thread` record, and the root shows `waiting` until every child thread has reported.
- A notification that says nothing changed is a defect in the brief. Tell that agent once how to keep its wait alive or sleep, and fix the template for the next brief.

## Keep agents short-lived

- Context is the root's job. Never put context sizes, limits or budgets in a brief, and never tell an agent to stop or hand off because of its context. An agent that knows its limit stops early to hand off, and every handoff re-reads the same setup. The root's watches compact agents (see "Context watches and compaction"), and the root decides when to replace one.
- When an agent grows large, the root may queue a compact for it, or steer it. To queue a compact, run `homebased --json thread compact <ref>`. It compacts an idle or waiting agent at once, and one running a turn answers `queued` and compacts when that turn ends. To steer it, tell it what to do, such as "finish the current item, update your progress file, and report". Neither one tells the agent about its context.
- Give each child thread one coherent phase. Do not split a phase only to make it smaller. Hand the next phase to a fresh child thread along with the state file and the previous report, and not to the old one with all its history. A follow-up of a few turns on the same change can stay with the same agent.
- The implementer owns its review loop. Its brief lets it start one fresh review, fix the findings, run delta reviews within the fix budget, and report the final verdict. The root does not pass findings back and forth between reviewer and implementer.
- How often an agent has compacted is never a reason to replace it. Replace an agent only when it is confirmed dead (see "Stalled or dead agents"), or when it repeats the same disproved action, or breaks the same stated invariant, after one correction and with no new evidence. Before a replacement for repetition, record both occurrences and the correction in the state file. A failed test or a valid wait is not a reason. To replace one, stop the old one first, with `task_cancel` for a T3 subagent's run, or by interrupting a child thread and settling it with `t3_thread_organize`, then start a fresh agent from the state file, its last report and its progress file (see the brief template), which holds what it finished.

## Stalled or dead agents

- When an agent reports `blocked`, act on the reason it gives. If the block is a decision, answer it with `t3_thread_send`, or with `homebased message send` for a child thread on another machine. If it is blocked on credentials, a resource or a dependency, resolve the block, then tell it to continue. Do not replace it.
- An agent has stalled when it has no final report and no live work. The stall watch (below) wakes the root with `THREAD_STALLED` when one sits idle with nothing due to wake it, so do not write a polling script for this. On that event, or when you suspect a stall, check it once with `homebased --json thread context --children --live`, or with `t3_thread_read` using a small `limit` and `runLimit` 1. If it is confirmed dead, or meets the replacement rule in "Keep agents short-lived", replace it as that rule says. An agent that waits on credentials, a resource or a dependency has not stalled; resolve the block instead. A child thread that sleeps on long work has not stalled either. Do not nurse an agent with repeated messages.

## Shared resources have one owner

- The root owns disk cleanup, queue order and priority, shared locks, handoffs between agents, and rebases or merges across PRs. Agents put candidates and requests in their reports and do not act on them.
- Run independent work now. Hold work back only for a reason, and write that reason in the state file.
- Run the cheapest upstream check before the downstream work that depends on it, and chain the downstream work on its result (homebased `after` for commands).
- Do not hold a scarce resource, such as a GPU lease or a device lock, while doing work that does not need it, such as a build. For GPU work, build in a CPU `homebased task run` and submit the job with `homebased resource job submit --after <build-task-id>`, which keeps the job out of the queue until the build succeeds.

## Decide and log

- Decide reversible calls yourself, and add one line with the time and the reason to the state file's settled decisions. Ask the user only about irreversible or outward-facing calls, or calls the user has kept for themselves, and batch those questions.
- When the user corrects something, record the correction in the state file. Send it only to the agents it applies to, and name each of them, so no agent applies a correction meant for another.

## Status

The user asks for a status to learn where the run stands against the goals they set, not to follow the work. They do not track the details between updates. Lead with what the run means for their goals, and put the details after it.

Use this format every time, in this order:

1. **Bottom line:** one or two sentences that say whether we are getting what we set out to do. When the honest answer is "unlikely" or "no", say that first.
2. **Goals:** one line per row of the state file's Goals table: on track, at risk or off track; what changed since the last status; and the next check that decides it.
3. **Needs you:** each decision with its options and a recommendation, or "nothing".
4. **Details**, for the user to dig into:
   - **Done:** a percentage, computed with the progress formula in the state file
   - **Left:** each open item, with its owner and next step
   - **Time left:** the finish estimate for the last item, and the slack against the deadline when there is one
   - **Issues:** blockers, failures and stalls
   - Workstream notes, each one naming the goal it moves. Work that changes no goal's standing goes in the state file or the journal, not here.

The first three parts have to stand alone, because the user may read nothing else. Keep them to about ten lines, and write them for someone who has not followed the run: no job ids, thread ids, SHAs, internal code names or parameter values unless the user uses them. Do not write "what this means" as a closing section. That meaning is the bottom line. A request for a "quick update" or a "TLDR" gets the first three parts and one line offering the details.

The same high-level bar applies to **every** update, milestone ping, and companion report page (not just formal status checks):
- **Always open with what the update means high level for us** in plain English — what decision just changed or locked in, what we now do differently (keep, cut, speed up, avoid spending money on), and how it affects our odds of winning or timeline. Put supporting numbers *after* that takeaway, never as a bare scorecard without translating why it matters.
- **Keep report pages strictly high-level and current-state, never a running log or artifact dump.** Overwrite the current verdict as results land rather than appending chronological logs, and never put per-item drill-downs (like individual question PNG galleries or trace tables) in the main flow; keep them out of the page or collapsed behind an opt-in `<details>` toggle unless explicitly requested there.

Send a status at phase boundaries, on any schedule the user sets, and when asked, but not for every event. Each time, update the standing in the Goals table and replace the Latest block in the state file with the bottom line and goal lines.

## Context watches and compaction

- Context limits:

  | Thread | Default compact watch | Root may adjust within |
  |---|---|---|
  | Root | 200K | 150–250K |
  | Claude T3 subagents and child threads: implementation, research, review | 250K | 200–300K; up to the model's ceiling only to let an agent finish an item that is nearly done |
  | Codex T3 subagents and child threads | none | they compact themselves at about 210–270K |
  | Agent-tool subagents, such as Explore | none, since they are not T3 threads | scope them to finish well under 150K, and under 100K on Haiku, which costs 5x above 100K |

  Why these numbers (from six weeks of usage data in `~/code/research/agents/context-size-vs-token-cost-2026-11.md`): cache reads are almost all input tokens, so each extra 100K adds about $0.02 to every Opus call. A compaction costs about $0.50 and pays for itself within 10 to 15 calls. A fresh agent costs about $1.27 before its first edit. Compacting at 250K is cheaper than both handing off at 150K and running to 400K. The data shows little loss of quality at these sizes.

  Each model's ceiling is its Claude Code auto-compact window in `~/.claude/settings.json`: 325K for Opus 5.5, and 256K for Sonnet 5.5 and Haiku 5.5. Claude Code compacts a thread mid-turn when it reaches that size, so a watch at or above it never fires. Keep every watch below the ceiling of the thread's model. A homebased watch compacts a thread only while it is idle or waiting, so it compacts at a turn boundary before the ceiling forces a compaction mid-turn.

  Reviews and other subagents that a T3 subagent or child thread starts itself get no context watch, because the root does not learn their thread ids. That is deliberate. They are usually Codex, and Claude reviews rarely grow large enough to compact.

- Set the root's watch on its own thread at the start:

  ```bash
  homebased --json thread watch --context-above 200k --action compact
  ```

  Give each Claude T3 subagent and child thread its own watch when you start it, using the thread id that `delegate_task` returns, or the `thread.thread_id` that `homebased t3 launch --child` returns, on any machine:

  ```bash
  homebased --json thread watch <thread-id> --context-above 250k --action compact
  ```

  Do not use `--children` for context watches. It covers the calling thread and every live T3 subagent and child thread, Codex ones included, with no model filter. Running it again after a new agent also adds a 250K watch on top of any limit you have already raised. Per-thread watches keep each limit where you set it, and leave Codex agents alone. Running the same watch again reuses it instead of adding a second one.

  To change one thread's limit, run `homebased thread unwatch <watch-id>` on its current context watch, then run `homebased --json thread watch <ref> --context-above <limit> --action compact`. A watch with another limit on the same thread is a second watch, not a replacement, and the lower limit still fires first. Keep the root within 150–250K. Record each change and its reason in the state file's settled decisions, and each watch id in its operations block.

  Watch the same threads for stalls, again after starting new agents:

  ```bash
  homebased --json thread watch --children --stalled-after 20m
  homebased --json thread watch <ref>... --stalled-after 20m   # top-level threads launched without --child
  ```

  It sends `THREAD_STALLED` once a thread has sat idle for 20 minutes with no turn running, no homebased task or job owing it events, no T3 subagent due to report to it, and no child thread of its own that has not reported. A T3 subagent's stall watch ends on its own when its task finishes, and a child thread's once it has reported and is settled. A top-level thread launched without `--child` looks the same after its final report as when it stalls, so run `homebased thread unwatch <watch-id>` when that report arrives.

  The context watches compact a thread once it sits idle or waiting above its limit, without waking the root. The root hears about it only when the compaction cannot start. Do not add a notify-only watch (no `--action`): each one wakes the root for a full turn, and that turn ends up running the same compaction by hand. Context size is never a reason to replace an agent (see "Keep agents short-lived"). Record the watch ids in the state file. `homebased thread watches` lists them, and `homebased thread unwatch <watch-id>` ends one.
- Rewrite the state file at every phase boundary, so a compaction at any time loses nothing. A root compaction wakes the root only when no T3 subagent, child thread, task or job is still due to report; otherwise the next report resumes it. To compact the root itself, run `homebased --json thread compact` with no reference: mid-turn it answers `queued`, and T3 compacts the root once the turn ends. Nothing wakes the root after a queued compaction, so use it only when a T3 subagent, child thread, task or job is still due to report. Otherwise run `homebased --json thread watch --context-above <limit> --action compact --once` with a limit a few thousand tokens below the root's current size and end the turn; the watch compacts the root and then wakes it.
- `homebased --json thread context --children --live` lists the live T3 subagents and child threads with their state, model and context size. Use it to rebuild the Owners table in the state file and to check whether an agent you suspect has stalled is still live.
- Compact, cancel, message or replace only threads in the root's own tree: the root, the threads it started, and their descendants. Before you act on a thread, confirm that it is in that tree with `homebased --json thread context --children --live` or the owners table in the state file. Do not act on a thread only because it is waiting or large. A compaction that started cannot be undone. Do not compact a thread again soon after it compacted, because that loses detail and saves little.
- After compaction, re-read the state file before anything else. Take ids and commands from its operations block instead of grepping the transcript or rerunning `--help`. If something you needed was missing, add it to the block.

## PR stacks

When the run coordinates stacked PRs, read [stacks.md](references/stacks.md).
