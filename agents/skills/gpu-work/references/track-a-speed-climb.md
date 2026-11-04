# Track A: speed climb

Making kernels or a pipeline faster: a contest (OKC) or a hill climb on our runtimes. Shorthands as in SKILL.md.

- SKILL.md "Be ambitious" applies in full. Get a rewrite's ablation ceiling within an hour, skip experiments whose expected gain is below the noise, and rewrite the largest share first.
- **Measure floors and rates rather than trusting specs.** Cold-L2 bandwidth falls well short of spec, and m16n8k16 FP16 issues at k8's rate. A "pipe saturated" verdict is relative to the instructions issued.
- **Sample power and clock-event reasons during the timed run** (`nvidia-smi --query-gpu=power.draw,clocks.sm,clocks_event_reasons.active -lms 20`) before counting idle issue slots as headroom. At the power cap, only fewer joules per FLOP buys speed. ncu locks clocks and hides this.
- **When kernels overlap on the GPU, end-to-end wins need less total SM occupancy**, not a faster kernel in isolation; a faster kernel on one stream can starve another.
- **Size a SIMT FP32 tile by shared-memory wavefronts, not occupancy.** An R x C register piece caps at RC / 4(R + C) of the FMA rate (50% for 4x4, 67% for 8x4).
- **Check warps per scheduler and top stalls before raising a pipe's rate.** Where ncu shows the tensor pipe as the limit, FP16-accumulate steps of 16 terms added into FP32 can stay within 1e-3; 32-term steps didn't. Price the step width on the f64 kernel test before timing.
- **Count format round trips around a GEMM** before expecting its precision's rate.
- **Count a tile's reuse before fusing a per-element pass into a tiled consumer**: work done while staging repeats once per block that stages it. Do it once in a small pass instead.
- **When ncu blames alignment on a GEMM with an odd N or K**, first compute the tile fill of the library's chosen tile; padding waste can dwarf the alignment cost.
- **Custom small-M kernels beat the library only at small M**; measure the crossover against its picks.
- **`torch.compile` baselines:** inductor keeps decode projections as cuBLAS GEMVs, so a small-GEMV decode step stays kernel-count bound. Profile `dynamic=True` at batch 1 too.
- **Bit-identity guards:** run an A/A hash check first on any mode with timing-based autotune, and guard such modes with a parity limit.
- **When a climb lands several steps at once**, build one control per step with the later steps switched off, so each gets its own ABBA verdict and output fingerprint.
- **Filter kernels by their full names.** ncu `--kernel-name regex:` matches bare function names (cuBLAS gemv is `kernel`) unless you pass `--kernel-name-base demangled`. compute-sanitizer takes `--kernel-name kns=X` or `regex=X`; `regex:X` prints usage and the step looks green.

Read: `PW/playbooks/gpu-hill-climb.md`, `R/cuda-kernels/okc-tuning-vs-rewrites-lessons-2026-10-25.md`, `R/okc/README.md`, `R/cuda-kernels/speakrs-rtx5090-optimization-2026-10-22.md` ("Transferable lessons" and "Rejected"), lessons §5a.
