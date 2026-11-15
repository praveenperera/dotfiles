# Briefs

An agent knows only its brief. Use [use-agents](../../use-agents/SKILL.md) for what every brief must contain and for the model and runner. This file adds what an orchestrated run needs on top of that. A child thread's brief is its first message, sent with `homebased t3 launch --child --message-file <brief>`; homebased appends the parent's link and the `homebased thread report` command for its one report.

## Template

```markdown
<objective, in one or two sentences>

Goal it serves: <the row of the state file's Goals table this work moves, and how>
Scope you own: <files, PR, branch or worktree>
Not yours: <what other agents own, and what you must not touch>
Decided approach: <the design choices already made>
Rules: read <absolute path to the state file>, section "Rules", and follow it. It overrides anything here that conflicts with it.

Verification: <commands and the pass condition>
Progress: after each item, update <absolute path to the progress file> with what is done, what is left and the commit SHAs.

<keep the one line for the runner>
Long work (child thread): follow "Long work" in ~/.agents/skills/use-agents/SKILL.md. Submit long work yourself and sleep on it with `homebased wait --long`. Send one report with `homebased thread report` once you are done, failed or blocked; it settles this thread. A progress update, such as "downloads are running", is not a report.
Long work (T3 subagent): follow "Long work" in ~/.agents/skills/use-agents/SKILL.md. Do no long work; if the work needs it, report blocked. Wait on shorter commands the way that section says, and end a turn only with a final report. A progress update, such as "the build is running, the wait is registered", is not a report: keep working, or keep the wait alive and handle its events, until you are done, failed or blocked.

Review loop: when the change is verified, <start one fresh review with delegate_task (<provider and model>), fix the findings, and get a delta review of each fix, within <n> fix passes | skip review>. Report the final verdict, not each round, with the head SHA it covered and the reviewer model and effort.

Shared resources: do not delete files, reorder queues, rebase or merge other PRs, or take locks that are not yours. List what you need in your report.

Report only a final state, and start it with one sentence on what the result means for the goal it serves:
- done: changed files, commits, check results, the final review verdict with the head SHA it covered and the reviewer model and effort, risks
- failed: what failed, the evidence, and what you tried
- blocked: the one decision or resource you need from the root
```

## Notes

- Do not mention context sizes, limits or compaction in a brief, or in a message to an agent. The root's watches handle compaction. An agent that knows its limit stops early to hand off.
- Point to the state file instead of pasting rules into the brief, so a correction lands in one place.
- Name the agent in any message that corrects it. When a correction applies to several agents, send it to each one.
- If the brief grants nested delegation, name the model and effort it may use and how many reviews it may start.
