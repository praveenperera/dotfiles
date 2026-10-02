---
name: run-auto
description: Run an explicitly requested task end to end without asking the user for input, delegating through homebased and use-agents, consulting Astra on hard calls, and keeping a decision log. Use only when the user invokes $run-auto or /run-auto.
disable-model-invocation: true
---

# Run auto

The user will be away from the computer. Carry the task to completion and leave a record they can review when they return.

## Enforce the invocation gate

Activate this workflow only when the user explicitly invokes this skill. Do not infer it from the kind of task. Keep it active until the task finishes or the user cancels it.

## Ask task-specific questions once at invocation

Before starting the autonomous run, ask the user any clarifying questions specific to this task whose answers would change how the run proceeds. Ask them in one batch while the user is still present. Skip questions that the request, the repository, or a sensible default already answers, and skip the round entirely when nothing is unclear.

Log the answers in the decision log. After this round, do not ask again.

## Do not ask the user

Make every decision yourself. Do not end a turn with a question, an options list, or a request for confirmation. When the user's input would normally be useful, choose the option you would recommend, log it, and continue.

Work outside the current folder or repository when the task needs it. These overrides do not lift the safety boundary: do not delete data the task does not own, force-push, publish, deploy, or spend money. When the best path needs one of those, take the best path that avoids it, log the blocked option with result `blocked`, and keep working on everything else.

## Notify the user with ntfy

Send a push notification with [notify.sh](scripts/notify.sh) at these points only:

- `blocked`: the run cannot finish without the user, or a blocked option leaves the result materially worse
- `important`: the user should know before they return, such as a risk to data they care about, a cost, or a result that changes the plan
- `done`: the run has stopped, whether the goal is met or every remaining path is blocked

```sh
~/.agents/skills/run-auto/scripts/notify.sh done "<repo>: <task-slug> finished" "<one-line result>. Log: <decision log path>"
```

Keep each message short. Name the repository and task, state the outcome or the decision needed, and give the decision log path. Do not include secrets, tokens, or private data. Do not notify for routine progress.

The script reads the topic from `RUN_AUTO_NTFY_TOPIC`, falling back to `~/.secrets.zsh`. Sending these notifications is an allowed exception to the publish rule. When the script fails, log the failure and keep working.

## Decide hard calls with Astra

When two options remain close after you have weighed the evidence, or you are stuck, ask Astra at `low` effort for a read-only second opinion through the [use-agents](../use-agents/SKILL.md) routing. Give it the goal, the options, the evidence so far, and the constraint that the answer must pick one. Then decide yourself and log the decision with Astra's input as evidence. Astra advises; the root decides.

Do not consult Astra for routine choices with a conventional default.

## Delegate through homebased

Use [use-agents](../use-agents/SKILL.md) to pick the model and effort for each piece of work. Submit agent work and long-running commands through [homebased](../homebased/SKILL.md) so they run unattended and report back with events. Use [fleet](../fleet/SKILL.md) to choose the machine for heavy work.

Run independent work in parallel when it does not contend for the same hardware.

The root keeps the goal, the plan, acceptance, and integration. Treat every worker report as evidence to check, not proof.

## Keep a decision log

Use [show-me-your-work](../show-me-your-work/SKILL.md) from the start. Log every chosen path, accepted or rejected attempt, Astra consultation, pivot, and blocker. Keep the progress checklist in `_scratch/<task-slug>/TASKS.md` alongside it.

## Improve a measurable result

When the task improves a measurable result, read [hill-climb.md](references/hill-climb.md) and follow it. Otherwise do not load it.

## Decide when to stop

Stop when one of these holds:

- the goal is met and verified
- every remaining path is blocked by the safety boundary

Do not stop because a single approach failed. Change direction and log the pivot.

## Report on return

Close the decision log and send the `done` notification. Then report in this order:

1. anything that needs the user's input, including blocked options
2. the final result and the evidence that verifies it
3. the important decisions and pivots
4. the decision log and checklist paths
