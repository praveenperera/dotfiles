# Track A: speed climb

Making kernels or a pipeline faster: a contest (OKC) or a hill climb on our runtimes. Shorthands as in SKILL.md.

- SKILL.md "Be ambitious" applies in full. Get a rewrite's ablation ceiling within an hour, skip experiments whose expected gain is below the noise, and rewrite the largest share first.
- **Measure floors and rates rather than trusting specs.** Cold-L2 bandwidth falls well short of spec. Every m16n8 FP16 shape issues at the same instruction rate (62.6 G/s on the 5090), so k16 does twice k8's work: pairing conv taps into k16 cut a stage 9.1%. A "pipe saturated" verdict is relative to the instructions issued.
- **Sample power and clock-event reasons during the timed run** (`nvidia-smi --query-gpu=power.draw,clocks.sm,clocks_event_reasons.active -lms 20`) before counting idle issue slots as headroom. At the power cap, only fewer joules per FLOP buys speed. ncu locks clocks and hides this.
- **When kernels overlap on the GPU, end-to-end wins need less total SM occupancy**, not a faster kernel in isolation; a faster kernel on one stream can starve another. Count registers x resident CTAs against the SM: a 2-CTA-cluster LSTM ran 1.45x faster alone and 4.1% slower end to end because it pinned 128 CTAs x 49.6K registers.
- **At ~99% GPU busy, stream priority and sibling streams only move time around.** Raising one stream's priority left total idle unchanged (3.38 to 3.41 s), and moving prep to a sibling stream cut batch gaps 0.85 to 0.12 s but added 0.43 s of busy time. Check GPU-busy % before trying overlap tricks.
- **Re-tune the dispatch recipe after every kernel change.** Thresholds tuned for the old kernels are wrong for the new ones: re-tuning after adding k16 kernels gave another 12%, and a tune run found FP16 direct kernels beating the generic TF32 recipe by 10.3%.
- **Size a SIMT FP32 tile by shared-memory wavefronts, not occupancy.** An R x C register piece caps at RC / 4(R + C) of the FMA rate (50% for 4x4, 67% for 8x4).
- **Check warps per scheduler and top stalls before raising a pipe's rate.** Where ncu shows the tensor pipe as the limit, FP16-accumulate steps of 16 terms added into FP32 can stay within 1e-3; 32-term steps didn't. This depends on scale: Winograd products at 2^18-2^20 overflow FP16 accumulation outright. Price the step width on the f64 kernel test before timing.
- **Count format round trips around a GEMM** before expecting its precision's rate.
- **Count a tile's reuse before fusing a per-element pass into a tiled consumer**: work done while staging repeats once per block that stages it. Do it once in a small pass instead.
- **When ncu blames alignment on a GEMM with an odd N or K**, first compute the tile fill of the library's chosen tile; padding waste can dwarf the alignment cost.
- **Custom small-M kernels beat the library only at small M**; measure the crossover against its picks.
- **`torch.compile` baselines:** inductor keeps decode projections as cuBLAS GEMVs, so a small-GEMV decode step stays kernel-count bound. Profile `dynamic=True` at batch 1 too.
- **Bit-identity guards:** run an A/A hash check first on any mode with timing-based autotune, and guard such modes with a parity limit. A pick that must beat the heuristic's by over 5% gave identical hashes across fresh processes; unmargined FP8 selection changed 22 of 2,620 transcripts between runs.
- **PDL (programmatic dependent launch) traps:** nvcc and ptxas can hoist `.nc` loads above `griddepcontrol.wait`; a GPU-scope fence also waits for the CTA's own in-flight TMA; and per-kernel profile rows of PDL kernels include co-resident wait time.
- **When a climb lands several steps at once**, build one control per step with the later steps switched off, so each gets its own ABBA verdict and output fingerprint.
- **Filter kernels by their full names.** ncu `--kernel-name regex:` matches bare function names (cuBLAS gemv is `kernel`) unless you pass `--kernel-name-base demangled`. compute-sanitizer takes `--kernel-name kns=X` or `regex=X`; `regex:X` prints usage and the step looks green.

Read: `PW/playbooks/gpu-hill-climb.md`.
