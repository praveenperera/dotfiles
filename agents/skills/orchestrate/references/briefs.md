# Delegate briefs

A delegate knows only its brief. Use [use-agents](../../use-agents/SKILL.md) for what every brief must contain and for the model and runner. This file adds what an orchestrated run needs on top of that.

## Template

```markdown
<objective, in one or two sentences>

Scope you own: <files, PR, branch or worktree>
Not yours: <what other delegates own, and what you must not touch>
Decided approach: <the design choices already made>
Rules: read <absolute path to the state file>, section "Rules", and follow it. It overrides anything here that conflicts with it.

Verification: <commands and the pass condition>
Budget: finish this phase under about 150K context. If you get close, stop and report what is done and what is left.

Long commands: follow "Long commands inside a delegate" in ~/.agents/skills/use-agents/SKILL.md. Submit them yourself, wait the way that section says for your runner, and never end a turn with only "still waiting".

Review loop: when the change is verified, <start one fresh review with delegate_task (<provider and model>), fix the findings, and get a delta review of each fix, within <n> fix passes | skip review>. Report the final verdict, not each round.

Shared resources: do not delete files, reorder queues, rebase or merge other PRs, or take locks that are not yours. List what you need in your report.

Report only a final state:
- done: changed files, commits, check results, review verdict, risks
- failed: what failed, the evidence, and what you tried
- blocked: the one decision or resource you need from the root
```

## Notes

- Point to the state file instead of pasting rules into the brief, so a correction lands in one place.
- Name the delegate in any message that corrects it. When a correction applies to several delegates, send it to each one.
- If the brief grants nested delegation, name the model and effort it may use and how many reviews it may start.
