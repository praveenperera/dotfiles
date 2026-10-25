# Hill climb

Use this loop when the task improves a measurable result.

## Set up the measurement

1. Define the metric, the correctness check, and the target before changing anything. Pick these yourself when the user did not.
2. Measure a baseline with enough repeated runs to know the noise floor. Record the command, machine, and commit.

Never run two timing-sensitive benchmarks on the same GPU or CPU at once.

## Check the gap before climbing

Hill climbing only finishes a design; it can't replace one. Before the first experiment, put the speed-of-light or floor, the current result, and the target or leader side by side. If the target is more than ~1.3x away, or the current design reaches less than ~70% of the floor, plan a rewrite toward a design that can reach it, and climb only once within ~15% of it. Judge a rewrite by its ablation ceiling, not its first end-to-end number, and give it real runway. Skip experiments whose expected gain is below the noise floor.

## Climb

1. Form one hypothesis per experiment, change one thing, and measure it with the same harness.
2. Accept a change only when it passes the correctness check and beats the current best by more than the noise. Otherwise revert it and log why.
3. Commit or tag each accepted state locally so the best-known-good version is always recoverable.
4. Rank remaining ideas by expected gain and cost, and run the next most promising one.

## Stop

Besides the stop conditions in the skill, stop when several consecutive experiments fail to beat the best result and an Astra review of the remaining ideas finds none worth the cost.

## Report

Report the final result against the baseline with the metric, the correctness evidence, and the best commit.
