---
name: gpu-work
description: Rules and workflow for GPU kernel and native-runtime work. Use for writing, porting or optimizing CUDA kernels (cuda-oxide, PTX tiers, shipping CUDA crates), speed climbs on GPU pipelines, porting a model off PyTorch or ONNX Runtime to a native runtime, replacing cuDNN or cuBLAS layers, profiling kernels with nsys or ncu, and Metal backends on Apple silicon (MSL kernels, Metal 4 tensor ops, Core ML or Neural Engine offload), or when the user invokes $gpu-work or /gpu-work. Defaults to ambitious floor-capable designs over easy tuning, and keeps a decision log. Do not use for ML training runs or other GPU jobs that change no kernel or runtime code.
---

# GPU work

Shorthands:

- `R` = `~/code/research/gpu-kernels/`. "Lessons §N" is section N of `R/cuda-kernels/cuda-kernel-lessons-general-2026-10-04.md`.
- `SMR` = `~/code/research/inference/specialized-model-runtimes/`: per-model baselines and decision logs.
- `PW` = `~/code/private-workbench/`: the hill-climb playbook, the worker brief template, and one sheet per repo with its build, bench, gate, scripts and rejected ideas.

## How to run the work

1. Read "Be ambitious", "Every project" and "Kernels" below, then the tracks that fit in "Pick your track".
2. Start the project's decision log before the first design choice: [references/recording.md](references/recording.md).
3. Speed work: open `PW/playbooks/gpu-hill-climb.md` and `PW/ops/<repo>/hill-climb/README.md` before the first change. nsys ranks the gaps; run ncu on each one you act on.
4. Delegating: fill `PW/playbooks/kernel-worker-brief-template.md` with the rules that apply, and name the ambitious design to build first.
5. Run all GPU work, including tests, nsys, ncu and compute-sanitizer, through the homebased GPU queue. Import every module each benchmark venv loads on the CPU before queuing. A green job only means its wrapper exited 0: read each step's output for missing packages, a failed program under nsys, a test binary built without its `cuda` feature (check `--list`), or a tool printing usage. Stop a probe inside a step with `kill $!`; a `pkill -f` pattern in the step's own command line kills the step.
6. After each phase, revert, parked attempt or user correction, log the decision and record results as in [references/recording.md](references/recording.md).

A worker with a brief skips steps 1, 3 and 4 and reports decisions and rule candidates; the root records them.

## Be ambitious

Small, safe tuning (parameter sweeps, launch configs, small fusions) is the second pass, not the plan.

- **Build the gap-and-floor table on day 0.** Per case: the measured hardware floor (bandwidth or FLOPs), our time, the target, and the achievable ceiling from ncu (the vendor's best kernel's tensor-pipe or DRAM active %).
- **Rewrite what is far from its floor.** More than ~1.3x from target or under ~70% of floor means rewrite; tune only within ~15% of floor. Spend about 70% of the budget on one or two rewrites.
- **Judge a rewrite by its ablation ceiling** (drop compute, reductions or secondary loads and re-time), not its first end-to-end number, and drop it only for a proven blocker. A native runtime is a go when its measured ceiling beats the best existing configuration by ~1.3x.

## Every project

- **Cut work first.** Remove, precompute, reuse or combine work within the accuracy gate, then speed up what remains.
- **Measure the end-to-end payoff before building.** Profile the whole pipeline on the target GPU, do the Amdahl arithmetic, and confirm with a rough prototype. Baseline against the best existing deployment (MPS workers, open upstream patches), not defaults.
- **Diagnose before you pick a design.** nsys says where the time goes; ncu `--set full` says why. Cite pipe-active %, top stalls, occupancy and where loads hit before writing a hypothesis. Time competing designs in one harness.
- **Read the kernels the incumbent actually runs** in nsys, not the flags you set: libraries pick algorithms and precisions per device and batch. Judge a path against its own peak: TF32 tensor GEMMs reach the GeForce FP32 peak, SIMT FP32 doesn't.
- **Gate accuracy against an audited reference**: f64 or the framework's own output, after checking its real precision, dropout and determinism. Run it twice in one process and once in another. Gate on the end metric, including low-confidence cases. Calibrate the noise band first by rerunning the reference with one exact-in-real-arithmetic perturbation per stage; autoregressive pipelines flip tokens, so don't promise byte identity before that.
- **Make parity gates fail loudly.** Assert output lengths, prefill outputs with NaN, never compare with `zip` or `f32::max` alone, and make some gate metric read every field the user sees.
- **Time only on an idle GPU through the queue** by `PW/playbooks/gpu-hill-climb.md` §4 step 5: frozen binaries, paired ABBA runs against A/A noise, every arm's output checked. Use `gpu-ab` for every timed comparison (`R/gpu-ab/README.md`). Split wins into same-output and changed-output files. Re-bench at the final commit.
- **Know how each profiler lies.** nsys inflates launch-bound work and needs `--cuda-graph-trace=node` for graphs; a tail after the last GPU range is host work. ncu locks base clocks and flushes L2: pass `--clock-control none` and pick `--cache-control` by whether the working set stays in L2. Where counters are denied, bound a kernel's parts with removal variants. Single-process GPU-busy doesn't bound throughput under MPS. Attribute GPU time to the outermost harness NVTX range. Headline numbers come from untraced runs.
- **Hybrid by default.** Keep the vendor library wherever it wins, and try its epilogues and options before hand-written GEMMs. Price a change that depends on the library's kernel choice over the real shape distribution.
- **Correctness and speed don't carry across architectures.** Run sanitizers on at least two. Bind a tuned change to its device, or check it on another, before it ships in a shared PTX tier.
- **Review correctness against upstream between phases.** Ask advisors for designs with exposed fractions from traces, not "what next".
- **Put stop rules in the first brief**: attempt caps, acceptance criteria, known risks. Hand a hard numerics problem to a different model after the second failed attempt.

## Kernels: cuda-oxide, shipped as PTX

Write every custom CUDA kernel in Rust with cuda-oxide, not CUDA C++, Triton or NVRTC strings; vendor libraries are the only non-Rust device code.

- Take cuda-oxide from `praveenperera/cuda-rust` `master` at `4e267d0ee` (never crates.io) with the toolchain its `rust-toolchain.toml` pins (stable 1.99.1). Record the rev in the kernel crate's `Cargo.toml` and `Cargo.lock`, and install cargo-oxide at the same commit under a project-owned `CARGO_HOME`. Move the pin in its own change.
- Kernels live in a standalone crate outside the workspace, with its own `rust-toolchain.toml`. Only a build script run through the GPU queue compiles it to PTX (and cubins where they pay).
- Commit the PTX and embed it with `include_str!` (`include_bytes!` for cubins); split per-tier crates past the crates.io limit.
- The host loads the PTX through the driver at run time (cudarc with dynamic loading), so users need no toolkit or GPU to build.
- CI checks the committed PTX against hashes of every input (kernel sources, cuda-oxide rev, toolchain, toolkit).

Read [references/cuda-oxide.md](references/cuda-oxide.md) before writing or porting a kernel. Don't edit it during a task; log new traps and unexplained slowdowns in the decision log with a `gotcha` Skill line. Compare builds with [scripts/sass_mix.sh](scripts/sass_mix.sh) and [scripts/nsys_kernel_diff.py](scripts/nsys_kernel_diff.py).

On Apple GPUs, read [references/apple.md](references/apple.md) instead of this section: kernels are MSL, and nsys, ncu and compute-sanitizer don't apply.

## Pick your track

| Track | You are | Read |
|---|---|---|
| A. Speed climb | Making kernels or a pipeline faster: a contest (OKC) or a hill climb on our runtimes | [references/track-a-speed-climb.md](references/track-a-speed-climb.md) |
| B. Replace library layers | Writing custom kernels for layers cuDNN or cuBLAS already runs | [references/track-b-library-layers.md](references/track-b-library-layers.md) |
| C. Native model runtime | Porting a whole model off PyTorch or ONNX Runtime | [references/track-c-native-runtime.md](references/track-c-native-runtime.md) |
| D. Ship CUDA code | PTX tiers, shared CUDA crates, crates.io, Python wheels | [references/track-d-ship.md](references/track-d-ship.md) |
| E. Many GPUs or rented boxes | Several architectures, or vast.ai and similar boxes | [references/track-e-many-gpus.md](references/track-e-many-gpus.md) |
| F. Agent-run kernel work | Running workers, advisors or reviewers on kernel tasks | [references/track-f-agents.md](references/track-f-agents.md) |

New to directing kernel work? `R/cuda-optimization-primer/README.md` and its `director-card.md` give the vocabulary.
