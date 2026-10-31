---
name: gpu-work
description: Rules and workflow for GPU kernel and native-runtime work. Use for writing, porting or optimizing CUDA kernels (cuda-oxide, PTX tiers, shipping CUDA crates), speed climbs on GPU pipelines, porting a model off PyTorch or ONNX Runtime to a native runtime, replacing cuDNN or cuBLAS layers, and profiling kernels with nsys or ncu, or when the user invokes $gpu-work or /gpu-work. Defaults to ambitious floor-capable designs over easy tuning, keeps a decision log, and adds new rules and gotchas back into this skill. Do not use for ML training runs or other GPU jobs that change no kernel or runtime code.
---

# GPU work

The rules for GPU kernel and native-runtime work live in this skill. The evidence behind them lives in the research repo, and runbooks live in private-workbench. Two shorthands are used below:

- `R` = `~/code/research/gpu-kernels/`: results per project, and the general lessons file `R/cuda-kernels/cuda-kernel-lessons-general-2026-10-04.md`. "Lessons §N" means its section N.
- `PW` = `~/code/private-workbench/`: the hill-climb playbook, the worker brief template, and one sheet per repo with its build, bench, accuracy gate, helper scripts and rejected ideas.

## How to run the work

1. Read "Be ambitious", "Every project" and "Kernels" below, then the track or tracks that fit in "Pick your track". Open only the notes they link.
2. Start the project's decision log before the first design choice, and keep it up as you go: [references/recording.md](references/recording.md) "Decision log".
3. Speed climb on an existing pipeline: follow `PW/playbooks/gpu-hill-climb.md` and the repo sheet `PW/ops/<repo>/hill-climb/README.md`.
4. Delegating to a worker: fill `PW/playbooks/kernel-worker-brief-template.md`, copying in only the rules that apply and naming the ambitious design to build first.
5. All GPU work, including tests, nsys and ncu, goes through the homebased GPU queue; never run it directly.
6. At the end of each phase, after a revert or a parked attempt, and after the user corrects you: log the decision, then record results and lessons as in [references/recording.md](references/recording.md).
7. Before finishing, add the new rules and gotchas from the decision log to this skill and commit them: [references/recording.md](references/recording.md) "Update this skill".

If you are a worker with a brief, the brief replaces steps 1, 3 and 4. Put your decisions and rule candidates in your report rather than the research repo or this skill; the root records them. The ambition rule still applies.

## Be ambitious

Agents drift to small, safe tuning (parameter sweeps, launch configs, small fusions) because it's easy to measure. That is the second pass, not the plan.

- **Build the gap-and-floor table on day 0.** For each case, list the hardware floor (bandwidth or FLOPs), our time and the target. Measure the floor rather than quoting it: notes have disagreed on the 5090's tensor rate by 2x.
- **Where a component is far from its floor, build the design that can reach it.** If the target is more than ~1.3x away, or ours runs below ~70% of the floor, the default is a rewrite; tune only within ~15% of the floor. Spend about 70% of the budget on one or two such rewrites and 30% on tuning and integration. In OKC the big gains came from rewrites, while a day of tuning landed inside the judge's ±2-3% noise. Tuning still pays as a second pass (scriptrs #14 gained 9.8-14.1% with no rewrite). (`R/cuda-kernels/okc-tuning-vs-rewrites-lessons-2026-10-25.md`, lessons §5a and §7)
- **Judge a rewrite by its ablation ceiling** and drop it only for a proven blocker, never for its first end-to-end number. A climb that only tunes has skipped the big rows.

## Every project

- **Measure the end-to-end payoff before building anything.** Profile the whole pipeline on the target GPU and do the Amdahl arithmetic: how much of the end-to-end time can this stage move? Then confirm with a rough prototype. A proof harness built before that check cost speakrs five days and shipped one PR; the rough check (same output, measured speedup on a fixed input set) shipped more in the next eight hours. (`R/cuda-kernels/speakrs-no-cudnn-lessons-2026-10-07.md`)
- **Find out what the incumbent actually runs.** Read kernel names in nsys, not the flags you set. cuBLAS with a BF16 math mode on FP32 buffers still ran TF32, cuDNN's "TF32 mode" sometimes picked FP32-accurate algorithms, and its FP32 mode used Winograd. The library picks a different algorithm per device and batch. On GeForce, TF32 runs at the FP32 rate; on datacenter parts it's about 8x. (Lessons §2, §6, §7)
- **Judge accuracy against an audited reference.** Truth is f64 or the framework's own output, not the library. Audit the reference itself: SAM 3's "FP32" PyTorch cast its MLP to BF16, and NeMo references ran with dropout on. Gate on the end metric (WER, DER, score and IoU), and include low-confidence cases, because they move first. (Lessons §3, §7, §10)
- **Make parity gates fail loudly.** Assert output lengths, prefill outputs with NaN, never compare with `zip` or `f32::max` alone, and compare every output the user sees (boxes, kept queries, tokens, timestamps). Two ports had gates that passed on empty or NaN output. (Lessons §10)
- **Time only on an idle, dedicated GPU through the queue, by the playbook's acceptance rule.** In short: frozen binaries, paired ABBA runs in one job judged against A/A noise, every arm's output checked, winners re-confirmed. The full rule lives in `PW/playbooks/gpu-hill-climb.md` §4 step 5; link to it rather than restating it. The `gpu-ab` CLI applies it and prints ACCEPT, REJECT or INCONCLUSIVE; use it for every timed comparison rather than a hand-rolled script, and add an adapter for a new repo. Measured noise, refusals and design decisions are in `R/gpu-ab/README.md`. (Lessons §6, §7, §8)
- **Know how each profiler lies.** nsys inflates launch-bound work and needs `--cuda-graph-trace=node` for graphs. ncu runs at base clocks, flushes L2 by default and is often denied on rented boxes; removal variants (delete math, staging or epilogue and re-time) work without counters. A "pipe saturated" verdict is relative to the instruction issued. Headline numbers come from untraced runs. (Lessons §6, §7, §8; `R/cuda-kernels/speakrs-rtx5090-optimization-2026-10-22.md`)
- **Re-bench at the final commit.** Correctness and safety fixes move speed: one SAM 3 fix took prompts from 12.0 to 15.8 ms. Quote only numbers from the final commit. (Lessons §7, §10)
- **Hybrid by default.** Keep the vendor library wherever it wins, try its epilogues and options before hand-written GEMMs (a cuda-oxide GEMM with a fused epilogue lost 1.5-2x to cuBLAS), and replace only measured hot spots. (Lessons §7, §8; no-cuDNN lessons)
- **Neither correctness nor speed carries over between GPU architectures.** Run sanitizers on at least two. A change tuned on one GPU either binds to that exact device or is checked on another before it ships in a shared PTX tier, and results say which GPU they were measured on. (Lessons §6; RTX 5090 note "Gating")
- **Get a correctness review against upstream between phases**, not only at the end, and not just a security scan. Ask advisors for designs with exposed fractions from traces, not "what next". (Lessons §10; `R/cuda-kernels/speakrs-cuda-advisor-comparison-2026-10-08.md` §8)
- **Write stop rules into the first brief**: attempt caps, acceptance criteria and known risks. Run long jobs as supervised tasks through the GPU queue, and consider handing a hard numerics problem to a different model after the second failed attempt (it worked once, in the speakrs tier run). (Lessons §4, §5)

## Kernels: cuda-oxide, shipped as PTX

Write every custom CUDA kernel in Rust with cuda-oxide, not CUDA C++, Triton or NVRTC source strings. cuBLAS, cuBLASLt and other vendor libraries are the only non-Rust device code, and a climb that needs a new kernel writes it in the kernel crate, not in a `.cu` file. cuda-oxide needs its pinned Rust toolchain (with rustc-dev), LLVM and a CUDA toolkit; that cost stays with the maintainers, and the published library builds on stable Rust:

- Take cuda-oxide from the tip of its git `main`, never a crates.io release. When you start kernel work or rebuild the PTX, move to the current tip and to the toolchain that tip pins in its own `rust-toolchain.toml` (stable 1.99.1 in October 2026, a nightly before that). Record the commit (a `rev` or the lockfile) so the PTX check knows which cuda-oxide built the committed files. Install cargo-oxide under the project's own `CARGO_HOME`, so other projects' backend caches stay at their revs.
- Kernels live in a standalone crate outside the workspace, with its own `rust-toolchain.toml` matching cuda-oxide's (the speakrs and sam3rs layout). Only a build command (an xtask or script run on a GPU box) compiles it, once per area and tier, to PTX and, where it pays, cubins.
- Commit the generated PTX and embed it with `include_str!` (`include_bytes!` for cubins). If the artifacts would pass the crates.io upload limit, split them into one crate per PTX tier: an sm75 baseline, plus higher tiers only where they unlock math the baseline can't use.
- The host library loads the PTX through the driver at run time (for example cudarc with dynamic loading), so users need no special toolchain, no CUDA toolkit and no GPU to build it, and the installed driver JIT-compiles the PTX.
- CI checks the committed PTX against hashes of every input that can change it (kernel sources, cuda-oxide rev, toolchain, toolkit), so stale PTX fails without a GPU.

Track D has the shipping details. Before writing or porting a kernel, read [references/cuda-oxide.md](references/cuda-oxide.md), the known traps and per-build checks. Add each new trap you hit to it, and each slowdown you measured but couldn't explain to its open list. To compare a kernel build against a reference build, use this skill's scripts: [scripts/sass_mix.sh](scripts/sass_mix.sh) `REF.ptx NEW.ptx [ARCH] [KERNEL...]` (calls, registers, spills and trap opcodes per kernel; CPU only) and [scripts/nsys_kernel_diff.py](scripts/nsys_kernel_diff.py) `CTL.sqlite[,...] EXP.sqlite[,...] [N]` (per-kernel GPU time from nsys exports profiled through the queue). (`R/cuda-kernels/sam3rs-cuda-oxide-port-2026-10-30.md`; lessons §1)

## Pick your track

Most projects fit one or two tracks. Each file holds that track's rules and the notes to open.

| Track | You are | Read |
|---|---|---|
| A. Speed climb | Making kernels or a pipeline faster: a contest (OKC) or a hill climb on our runtimes | [references/track-a-speed-climb.md](references/track-a-speed-climb.md) |
| B. Replace library layers | Writing custom kernels for layers cuDNN or cuBLAS already runs (speakrs) | [references/track-b-library-layers.md](references/track-b-library-layers.md) |
| C. Native model runtime | Porting a whole model off PyTorch or ONNX Runtime (speakrs, scriptrs, sam3rs) | [references/track-c-native-runtime.md](references/track-c-native-runtime.md) |
| D. Ship CUDA code | PTX tiers, shared CUDA crates, crates.io, Python wheels (cudakit) | [references/track-d-ship.md](references/track-d-ship.md) |
| E. Many GPUs or rented boxes | Supporting several architectures, or working on vast.ai and similar boxes | [references/track-e-many-gpus.md](references/track-e-many-gpus.md) |
| F. Agent-run kernel work | Running workers, advisors or reviewers on kernel tasks | [references/track-f-agents.md](references/track-f-agents.md) |

New to directing kernel work? The CUDA optimization primer (`R/cuda-optimization-primer/README.md`) and its director card (`R/cuda-optimization-primer/director-card.md`) give the vocabulary: which unit does the work, which memory each step uses, and how to classify a bottleneck.
