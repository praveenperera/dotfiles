---
name: run-auto
description: Run an explicitly requested task end to end without asking the user for input, delegating through homebased and use-agents, consulting Astra on hard calls, and keeping a decision log. Use only when the user invokes $run-auto or /run-auto. Intended for performance optimization, hill climbing, auto research, and experiments such as GPU kernel tuning.
disable-model-invocation: true
---

# Run auto

The user is away from the computer. Carry the task to completion and leave a record they can review when they return.

## Enforce the invocation gate

Activate this workflow only when the user explicitly invokes this skill. Do not infer it from a task that looks like optimization or research. Keep it active until the task finishes or the user cancels it.

## Do not ask the user

Make every decision yourself. Do not end a turn with a question, an options list, or a request for confirmation. When the user's input would normally be useful, choose the option you would recommend, log it, and continue.

These overrides do not lift the safety boundary. Do not delete data, force-push, publish, deploy, spend money, or change anything outside the repository or approved machines. When the best path needs one of those, take the best path that avoids it, log the blocked option with result `blocked`, and keep working on everything else.

## Decide hard calls with Astra

When two options remain close after you have weighed the evidence, or you are stuck, ask Astra at `low` effort for a read-only second opinion through the [use-agents](../use-agents/SKILL.md) routing. Give it the goal, the options, the evidence so far, and the constraint that the answer must pick one. Then decide yourself and log the decision with Astra's input as evidence. Astra advises; the root decides.

Do not consult Astra for routine choices with a conventional default.

## Delegate through homebased

Use [use-agents](../use-agents/SKILL.md) to pick the model and effort for each piece of work. Submit agent work, long builds, benchmarks, and training runs through [homebased](../homebased/SKILL.md) so they run unattended and report back with events. Use [fleet](../fleet/SKILL.md) to place GPU or heavy work on the right machine.

Run independent experiments in parallel when they do not contend for the same hardware. Never run two timing-sensitive benchmarks on the same GPU or CPU at once.

The root keeps the goal, the experiment plan, acceptance, and integration. Treat every worker report as evidence to check, not proof.

## Keep a decision log

Use [show-me-your-work](../show-me-your-work/SKILL.md) from the start. Log every chosen path, accepted or rejected experiment, Astra consultation, pivot, and blocker. Keep the progress checklist in `_scratch/<task-slug>/TASKS.md` alongside it.

## Run experiments as a hill climb

For optimization and experimental work:

1. Define the metric, the correctness check, and the target before changing anything. Pick these yourself when the user did not.
2. Measure a baseline with enough repeated runs to know the noise floor. Record the command, machine, and commit.
3. Form one hypothesis per experiment, change one thing, and measure it with the same harness.
4. Accept a change only when it passes the correctness check and beats the current best by more than the noise. Otherwise revert it and log why.
5. Commit or tag each accepted state locally so the best-known-good version is always recoverable.
6. Rank remaining ideas by expected gain and cost, and run the next most promising one.

For research, treat each source or approach as an experiment: state what it should answer, record what it found, and link the evidence.

## Decide when to stop

Stop when one of these holds:

- the target is met and verified
- several consecutive experiments fail to beat the best result, and an Astra review of the remaining ideas finds none worth the cost
- every remaining path is blocked by the safety boundary

Do not stop because a single approach failed. Change direction and log the pivot.

## Report on return

Close the decision log. Then report in this order:

1. anything that needs the user's input, including blocked options
2. the final result against the baseline, with the metric, correctness evidence, and best commit
3. the important decisions and pivots
4. the decision log and checklist paths
