# Briefs

An agent knows only its brief. Use [use-agents](../../use-agents/SKILL.md) for what every brief must contain and for the model and runner. This file adds what an orchestrated run needs on top of that. A child thread's brief is its first message, sent with `homebased t3 launch --child --message-file <brief>`; homebased appends the parent's link and the `homebased thread report` command for its one report.

## Template

```markdown
<objective, in one or two sentences>

Goal it serves: <the row of the state file's Goals table this work moves, and how>
Scope you own: <files, PR, branch or worktree>
Not yours: <what other agents own, and what you must not touch>
Decided approach: <the design choices already made>
<keep the one line for where the agent runs>
Rules (same machine as the root): read <absolute path to the state file>, section "Rules", and follow it. It overrides anything here that conflicts with it.
Rules (another machine): the state file is on <root's ssh host, such as main>, not on this machine. Read it with `ssh <host> cat <absolute path to the state file>`, section "Rules", and follow it; reread it the same way whenever the root says the rules changed. It overrides anything here that conflicts with it. Do not edit it.

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
- The state file, the PR bar file and any other file a brief links to live on the root's machine. For an agent on another machine, such as `code.local`, use the "another machine" Rules line and give every other root-side file the same `ssh <host> cat <path>` form, or `scp <host>:<path> .` for a file it needs locally. Check once that the host can read the file over ssh before the first brief to that machine. Do not paste the rules into the brief or a message instead; a pasted copy goes stale at the next correction.
- Put an agent's progress file on the machine it runs on, and record that machine and path in the state file. The root reads it over ssh when it replaces the agent.
- Name the agent in any message that corrects it. When a correction applies to several agents, send it to each one.
- If the brief grants nested delegation, name the model and effort it may use and how many reviews it may start.
