# Track A: speed climb

Making kernels or a pipeline faster: a contest (OKC) or a hill climb on our runtimes. `R` = `~/code/research/gpu-kernels/`, `PW` = `~/code/private-workbench/`.

- The ambition rule in SKILL.md "Be ambitious" applies in full: about 70% of the budget on one or two rewrites aimed at the floor, 30% on tuning and integration, rewrites judged by their ablation ceiling within an hour (no compute, no reduce, no secondary loads).
- Skip experiments whose expected gain is below the noise. Rewrite the largest share first.
- Measure real hardware floors before trusting specs: H200 cold-L2 reads reached 3.6 of 4.8 TB/s.
- Measure the instruction issue rate before trusting "pipe saturated". m16n8k16 FP16 issues at the same rate as k8, which doubled throughput.
- When kernels overlap on the GPU, end-to-end wins need less total SM occupancy, not a faster kernel in isolation.
- Size a SIMT (FP32 FMA) tile by shared-memory wavefronts, not occupancy: a 16-byte warp load costs 4 or more wavefronts even when lanes broadcast, so an R x C register piece caps at RC / 4(R + C) of the FMA rate (50% for 4x4, 67% for 8x4). In sam3rs's FP32 flash attention, a second block per SM changed nothing and 8x4 pieces took 18.0 to 16.5 ms (`~/code/research/inference/specialized-model-runtimes/sam3rs-decisions.md`).
- Run an A/A hash check before a bit-identity guard on any mode with timing-based autotune: sam3rs TF32's near-tie cuBLASLt choices gave one binary two output hashes across processes. Guard such modes with a parity-limit gate.
- Read the spill before timing a change that doubles an MMA rate in a kernel already at 255 registers. In sam3rs's head-dim-256 attention, FP16-accumulated QK^T partials took the spill from 20 to 52 B and bought 10% of the kernel (0.45 of 4.22 ms an object), not the 1.5x the rate promised, and failed the f64 gate (`~/code/research/inference/specialized-model-runtimes/sam3rs-decisions.md`)
- Count the format round trips around a GEMM before expecting its precision's rate. sam3rs's tracker linears moved from TF32 to BF16 and gained 0.2 of their 1.2 ms an object, since each GEMM still converted FP32 inputs and wrote FP32 outputs that the next kernel re-read
- When a climb lands several steps at once, build one control per step from a source copy with the later steps switched off (`if false`, a threshold set to its maximum), so each step gets its own ABBA verdict and A/B fingerprint

Read: `PW/playbooks/gpu-hill-climb.md`, `R/cuda-kernels/okc-tuning-vs-rewrites-lessons-2026-10-25.md`, `R/okc/README.md`, `R/cuda-kernels/speakrs-rtx5090-optimization-2026-10-22.md` ("Transferable lessons" and "Rejected"), lessons §5a.
