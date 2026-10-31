# Track A: speed climb

Making kernels or a pipeline faster: a contest (OKC) or a hill climb on our runtimes. `R` = `~/code/research/gpu-kernels/`, `PW` = `~/code/private-workbench/`.

- The ambition rule in SKILL.md "Be ambitious" applies in full: about 70% of the budget on one or two rewrites aimed at the floor, 30% on tuning and integration, rewrites judged by their ablation ceiling within an hour (no compute, no reduce, no secondary loads).
- Skip experiments whose expected gain is below the noise. Rewrite the largest share first.
- Measure real hardware floors before trusting specs: H200 cold-L2 reads reached 3.6 of 4.8 TB/s.
- Measure the instruction issue rate before trusting "pipe saturated". m16n8k16 FP16 issues at the same rate as k8, which doubled throughput.
- When kernels overlap on the GPU, end-to-end wins need less total SM occupancy, not a faster kernel in isolation.

Read: `PW/playbooks/gpu-hill-climb.md`, `R/cuda-kernels/okc-tuning-vs-rewrites-lessons-2026-10-25.md`, `R/okc/README.md`, `R/cuda-kernels/speakrs-rtx5090-optimization-2026-10-22.md` ("Transferable lessons" and "Rejected"), lessons §5a.
