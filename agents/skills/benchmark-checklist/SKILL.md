---
name: benchmark-checklist
description: Vet a performance or evaluation number before reporting or acting on it by checking the limiter, tuning, physical limits, errors, repeatability, end-to-end relevance, comparability, and whether the work happened. Use when the user invokes $benchmark-checklist or /benchmark-checklist.
disable-model-invocation: true
---

# Benchmark Checklist

A measured number is a claim about a system. A run that went wrong still prints a plausible number: failed requests, a cache that skipped the work, code that never ran, a side left on defaults, a different dataset, and noise all produce results that look fine. If you cannot say why the number is not twice as good, you do not know what you measured.

Answer each question with evidence from a run, not a guess from reading the code. For a quick ballpark the user asked for, one run is enough; still check questions 4 and 7 and say it is one run. A choice between options is never a ballpark.

## Before running

- Write the claim you expect to make in the words you would ship, such as "diarization is 30% faster at p50 on the 60-file VoxConverse test set". The questions test that sentence.
- Read the measurement script. Note what it times, what it counts, and what it ignores.
- Check the load with `uptime` and the core count. If the machine is busy and you cannot stop the other work, interleave the sides so both see the same noise, and say so. For remote or GPU machines, check that no other job holds the GPU.

## The questions

1. **Why not double?** Name the limiter: a core, a lock, memory bandwidth, the GPU, the disk, the network, or the load generator. Get it from a profile or system counters in a run you do not report, since profilers slow the work. Map the hot spot to source. If a change did not move the number, the limiter explains why; find it before calling the change useless.
2. **Was it tuned?** Run every side the way production runs it: release builds, production flags, batch sizes, thread counts, warm or cold caches as production sees them, and the same versions and data. A limiter that is a setting, such as a debug build or a missing index, means that side is untuned. Tune it and measure again before picking a winner.
3. **Did it break limits?** Do the arithmetic. Compare bytes per second with disk and network bandwidth, and operations per second with the cores or GPU throughput you have. Removing a piece that takes 10% of the run can make it at most about 11% faster. A result past a limit measured something other than the work.
4. **Did it error?** Count failures, skipped files, and non-success responses, and check that outputs are correct, not just present. Errors are often fast and timeouts slow. If the script does not count errors, add the count.
5. **Does it reproduce?** Run each side at least five times, alternating A and B so warmup and drift do not favor one side. Report the median and the range. Treat a gap smaller than run-to-run variation as no measurable difference.
6. **Does it matter?** Next to a micro result, measure the end-to-end path a user waits on with realistic sizes. Report the micro result as a share of the whole.
7. **Did it even happen?** Confirm the work ran inside the timed region: the model ran on every file, the rows were written, the result was used. Lazy evaluation, unawaited work, and timeouts all produce numbers for work that never happened.

## Evaluation metrics

For accuracy metrics such as DER, WER, or eval scores, also check comparability:

- Both sides use the same dataset, split, files, and reference labels. A number on a different dataset is a different claim.
- Scoring settings match, such as collar, overlap handling, and normalization.
- Nothing was tuned on the test split. Thresholds and hyperparameters chosen on test data make the result optimistic.
- The gap holds across files or trials, not only in the aggregate.

## Report

- Lead with the verdict: faster, slower, better, worse, no measurable difference, or inconclusive.
- Give the number with its unit, the run count, the range, the limiter, and the dataset. For example, "p50 41 ms to 33 ms, median of 7 runs per side, range 32 to 35 ms after, bound by JSON parsing on one core."
- Call the verdict inconclusive when a side ran untuned, the sides are not comparable, or you could not check questions 4 and 7. For a speed or throughput number, also call it inconclusive when you cannot name the limiter. Name the gap.
- Keep a pull request body to one primary number. Put the runs, range, and limiter evidence in a linked artifact or notes file.
