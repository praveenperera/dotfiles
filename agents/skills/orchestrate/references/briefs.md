# Delegate briefs

A delegate knows only its brief. Use [use-agents](../../use-agents/SKILL.md) for what every brief must contain and for the model and runner. This file adds what an orchestrated run needs on top of that.

## Template

```markdown
<objective, in one or two sentences>

Goal it serves: <the row of the state file's Goals table this work moves, and how>
Scope you own: <files, PR, branch or worktree>
Not yours: <what other delegates own, and what you must not touch>
Decided approach: <the design choices already made>
Rules: read <absolute path to the state file>, section "Rules", and follow it. It overrides anything here that conflicts with it.

Verification: <commands and the pass condition>
Progress: after each item, update <absolute path to the progress file> with what is done, what is left and the commit SHAs.

Long commands: follow "Long commands inside a delegate" in ~/.agents/skills/use-agents/SKILL.md. Submit them yourself, wait the way that section says for your runner, and end a turn only with a final report. A progress update, such as "downloads are running, the wait is registered", is not a report: keep working, or keep the wait alive and handle its events, until you are done, failed or blocked.

Review loop: when the change is verified, <start one fresh review with delegate_task (<provider and model>), fix the findings, and get a delta review of each fix, within <n> fix passes | skip review>. Report the final verdict, not each round, with the head SHA it covered and the reviewer model and effort.

Shared resources: do not delete files, reorder queues, rebase or merge other PRs, or take locks that are not yours. List what you need in your report.

Report only a final state, and start it with one sentence on what the result means for the goal it serves:
- done: changed files, commits, check results, the final review verdict with the head SHA it covered and the reviewer model and effort, risks
- failed: what failed, the evidence, and what you tried
- blocked: the one decision or resource you need from the root
```

## Notes

- Do not mention context sizes, limits or compaction in a brief, or in a message to a delegate. The root's watches handle compaction. A delegate that knows its limit stops early to hand off.
- Point to the state file instead of pasting rules into the brief, so a correction lands in one place.
- Name the delegate in any message that corrects it. When a correction applies to several delegates, send it to each one.
- If the brief grants nested delegation, name the model and effort it may use and how many reviews it may start.
