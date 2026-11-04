# Track A: speed climb

Making kernels or a pipeline faster: a contest (OKC) or a hill climb on our runtimes. Shorthands as in SKILL.md.

- SKILL.md "Be ambitious" applies in full. Get a rewrite's ablation ceiling within an hour, and skip experiments whose expected gain is below the noise. Rewrite the largest share first.
- **Measure floors and rates rather than trusting specs.** H200 cold-L2 reads reached 3.6 of 4.8 TB/s; m16n8k16 FP16 issues at k8's rate, doubling throughput. A "pipe saturated" verdict is relative to the instructions issued.
- **Sample power and clock-event reasons during the timed run** (`nvidia-smi --query-gpu=power.draw,clocks.sm,clocks_event_reasons.active -lms 20`) before counting idle issue slots as headroom. sam3rs's FP32 encode sat at the 5090's 575 W cap, where only fewer joules per FLOP buys speed. ncu locks clocks and hides this.
- **When kernels overlap on the GPU, end-to-end wins need less total SM occupancy**, not a faster kernel in isolation. A faster kernel on one stream can lengthen the run by starving another (whisperdrz e21, -0.3%).
- **Size a SIMT FP32 tile by shared-memory wavefronts, not occupancy.** An R x C register piece caps at RC / 4(R + C) of the FMA rate (50% for 4x4, 67% for 8x4); in sam3rs flash attention 8x4 took 18.0 to 16.5 ms where a second block per SM did nothing.
- **Check warps per scheduler and top stalls before raising a pipe's rate.** sam3rs's head-dim-256 attention had one warp per scheduler, so FP16-accumulate QK^T bought 10% instead of 1.5x and failed the f64 gate. Where ncu does show the tensor pipe as the limit, FP16-accumulate steps of 16 terms added into FP32 gave 592 to 447 µs within 1e-3; 32-term steps crossed it. Price the step width on the f64 kernel test before timing. (`SMR/sam3rs-decisions.md`)
- **Count format round trips around a GEMM before expecting its precision's rate.** sam3rs linears moved TF32 to BF16 and gained 0.2 of 1.2 ms, since each still converted FP32 in and out.
- **Count a tile's reuse before fusing a per-element pass into a tiled consumer.** Work done while staging runs once per block that stages it: RoPE fused into key staging ran 9-81 times per key and cost 1.67 ms against 1.2 saved; rotating keys once in a small pass, with the query rotation kept fused, gained 2.3-3.2%.
- **When ncu blames alignment on a GEMM with an odd N or K, first compute the tile fill of the library's chosen tile**; padding waste can dwarf the alignment cost (whisperdrz ECAPA2 convs).
- **Custom small-M kernels can beat the library only at small M**: whisperdrz's won at 2-16 rows and lost to cuBLASLt's picks at 42-54 (0.63-0.85x).
- **`torch.compile` baselines:** inductor keeps each decode projection as a cuBLAS GEMV, so a small-GEMV decode step stays kernel-count bound (Docling TableFormer ran 8% slower). Profile `dynamic=True` at batch 1 too: Kokoro's instance norms ran at 1% SM throughput there while batch 8 gained 2.4x. (`SMR/docling-baselines-2026-11-01.md`, `SMR/kokoro-baselines-2026-11-01.md`)
- **Bit-identity guards:** run an A/A hash check first on any mode with timing-based autotune (sam3rs TF32's near-tie cuBLASLt picks gave one binary two hashes); guard such modes with a parity limit.
- **When a climb lands several steps at once**, build one control per step from a source copy with the later steps switched off, so each gets its own ABBA verdict and output fingerprint.
- **Filter kernels by their full names.** ncu `--kernel-name regex:` matches bare function names (cuBLAS gemv is `kernel`) unless you pass `--kernel-name-base demangled`. compute-sanitizer takes `--kernel-name kns=X` or `regex=X`; `regex:X` prints usage and the step looks green.

Read: `PW/playbooks/gpu-hill-climb.md`, `R/cuda-kernels/okc-tuning-vs-rewrites-lessons-2026-10-25.md`, `R/okc/README.md`, `R/cuda-kernels/speakrs-rtx5090-optimization-2026-10-22.md` ("Transferable lessons" and "Rejected"), lessons §5a.
