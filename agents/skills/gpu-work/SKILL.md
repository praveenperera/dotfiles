---
name: gpu-work
description: Rules and workflow for GPU kernel and native-runtime work. Use for writing, porting or optimizing CUDA kernels (cuda-oxide, PTX tiers, shipping CUDA crates), speed climbs on GPU pipelines, porting a model off PyTorch or ONNX Runtime to a native runtime, replacing cuDNN or cuBLAS layers, and profiling kernels with nsys or ncu, or when the user invokes $gpu-work or /gpu-work. Defaults to ambitious floor-capable designs over easy tuning, and keeps a decision log. Do not use for ML training runs or other GPU jobs that change no kernel or runtime code.
---

# GPU work

This skill holds the rules. The evidence lives in the research repo, and runbooks live in private-workbench. Shorthands used across the skill:

- `R` = `~/code/research/gpu-kernels/`. "Lessons §N" is section N of `R/cuda-kernels/cuda-kernel-lessons-general-2026-10-04.md`.
- `SMR` = `~/code/research/inference/specialized-model-runtimes/`: per-model baselines and decision logs (sam3rs, docling, liveportrait, kokoro, whisperdrz).
- `PW` = `~/code/private-workbench/`: the hill-climb playbook, the worker brief template, and one sheet per repo with its build, bench, gate, scripts and rejected ideas.

## How to run the work

1. Read "Be ambitious", "Every project" and "Kernels" below, then the tracks that fit in "Pick your track". Open only the notes they link.
2. Start the project's decision log before the first design choice: [references/recording.md](references/recording.md).
3. Speed climb, or a port's speed phase once its parity gate passes: open `PW/playbooks/gpu-hill-climb.md` and the repo sheet `PW/ops/<repo>/hill-climb/README.md` before the first speed change, and follow its loop: nsys to rank the gaps, then ncu on each one you act on.
4. Delegating: fill `PW/playbooks/kernel-worker-brief-template.md` with only the rules that apply, and name the ambitious design to build first.
5. All GPU work, including tests, nsys, ncu and compute-sanitizer, goes through the homebased GPU queue. Before queuing, import every module each benchmark venv loads, on the CPU. A green job only means its wrapper exited 0, so read each step's own output. Green jobs have hidden missing Python packages (three LivePortrait jobs), an nsys step whose program failed, a test binary built without its `cuda` feature that ran no tests (check `--list`), and a compute-sanitizer that printed usage for a flag it doesn't take. To stop a probe inside a step, `kill $!`; a `pkill -f` pattern that appears in the step's own command line kills the step.
6. At the end of each phase, after a revert or a parked attempt, and after the user corrects you: log the decision, then record results and lessons as in [references/recording.md](references/recording.md).

A worker with a brief skips steps 1, 3 and 4, and puts decisions and rule candidates in its report; the root records them. The ambition rule still applies.

## Be ambitious

Agents drift to small, safe tuning (parameter sweeps, launch configs, small fusions) because it's easy to measure. That is the second pass, not the plan.

- **Build the gap-and-floor table on day 0.** Per case: the measured hardware floor (bandwidth or FLOPs; notes have disagreed on the 5090's tensor rate by 2x), our time, the target, and the achievable ceiling from ncu (the vendor's best kernel's tensor-pipe or DRAM active %). In sam3rs a peak-FLOPs floor showed an 8.4 ms GEMM gap that cuBLASLt's 83-87% tensor-active mostly ruled out.
- **Where a component is far from its floor, build the design that can reach it.** More than ~1.3x from target or under ~70% of floor means rewrite; tune only within ~15% of floor. Spend about 70% of the budget on one or two rewrites and 30% on tuning and integration. (`R/cuda-kernels/okc-tuning-vs-rewrites-lessons-2026-10-25.md`, lessons §5a, §7)
- **Judge a rewrite by its ablation ceiling** (drop compute, reductions or secondary loads and re-time) and drop it only for a proven blocker, never for its first end-to-end number. The same bar decides whole native runtimes: a measured ceiling past ~1.3x over the best existing configuration is a go. (`SMR/docling-baselines-2026-11-01.md`)

## Every project

- **First cut the work needed for the same result.** Remove, precompute, reuse or combine work within the accuracy gate, then speed up what remains. An algorithm change that removes work is often the ambitious rewrite. (`~/code/research/inference/gpu-economics/deepinfer-lessons/gpu-optimization-review-2026-10-31.md`)
- **Measure the end-to-end payoff before building anything.** Profile the whole pipeline on the target GPU, do the Amdahl arithmetic, and confirm with a rough prototype; a proof harness built first cost speakrs five days. Baseline against the best existing deployment, not defaults: worker processes under CUDA MPS and open upstream patches took Docling from 12.1 to 70.4 pages/s with no new kernels. (`R/cuda-kernels/speakrs-no-cudnn-lessons-2026-10-07.md`, `SMR/docling-baselines-2026-11-01.md`)
- **Diagnose before you pick a design.** nsys says where the time goes; ncu (`--set full`, playbook §3) says why. Cite pipe-active %, top stalls, occupancy and where loads hit before writing a hypothesis; a step chosen without that reading doesn't count. Time competing designs in one harness before choosing a crossover. (`SMR/sam3rs-decisions.md`)
- **Find out what the incumbent actually runs.** Read kernel names in nsys, not the flags you set: cuBLAS and cuDNN pick algorithms and precisions per device and batch that the math mode doesn't promise. Judge a path against its own peak: TF32 tensor GEMMs reach the GeForce FP32 peak while SIMT FP32 doesn't (1.65x on a 5090), and TensorRT fp16 engines run FP16-accumulate convs. (Lessons §2, §6, §7; `SMR/liveportrait-baselines-2026-11-01.md`)
- **Judge accuracy against an audited reference.** Truth is f64 or the framework's own output, not the library. Audit the reference: SAM 3's "FP32" PyTorch cast its MLP to BF16, NeMo ran with dropout on, and upstream whisperdrz was nondeterministic. Run it twice in one process and once in another. Gate on the end metric (WER, DER, IoU) including low-confidence cases. Calibrate the noise band before the climb by rerunning the reference with one exact-in-real-arithmetic perturbation per stage the port will change. In an autoregressive pipeline that flips words (whisperdrz: 10-17 of 24 files per stage), so don't promise byte identity before that test. (Lessons §3, §7, §10; `SMR/whisperdrz-decisions.md`)
- **Make parity gates fail loudly.** Assert output lengths, prefill outputs with NaN, never compare with `zip` or `f32::max` alone, and make sure some gate metric reads every field the user sees. Text metrics that strip markup and DER that reads only segments both missed whisperdrz's inline word timings. (Lessons §10)
- **Time only on an idle GPU through the queue, by the playbook's acceptance rule** (`PW/playbooks/gpu-hill-climb.md` §4 step 5: frozen binaries, paired ABBA runs against A/A noise, every arm's output checked). Use the `gpu-ab` CLI for every timed comparison and add an adapter for a new repo (`R/gpu-ab/README.md`). When a change alters output, split the win into same-output and changed-output files, so different decoding can't pass as kernel speed. Re-bench at the final commit: correctness fixes move speed. (Lessons §6-§8, §10)
- **Know how each profiler lies.** nsys inflates launch-bound work and needs `--cuda-graph-trace=node` for graphs; a profile tail after the last GPU range is host work, so name it before calling a run GPU-bound. ncu locks base clocks and flushes L2: pass `--clock-control none` and pick `--cache-control` by whether the working set stays in L2 (a 32-layer decode step streams 1.6 GB). Where counters are denied, removal variants bound a kernel's parts. A single-process GPU-busy union doesn't bound throughput under MPS. Libraries push NVTX ranges inside yours, so attribute GPU time to the outermost harness range. Headline numbers come from untraced runs. (Lessons §6-§8)
- **Hybrid by default.** Keep the vendor library wherever it wins and try its epilogues and options before hand-written GEMMs (a cuda-oxide GEMM with a fused epilogue lost 1.5-2x to cuBLAS). Price a change that depends on the library's kernel choice over the real shape distribution, not one shape. (Lessons §7, §8)
- **Neither correctness nor speed carries over between GPU architectures.** Run sanitizers on at least two. A change tuned on one GPU binds to that device or is checked on another before it ships in a shared PTX tier, and results name the GPU. (Lessons §6)
- **Get a correctness review against upstream between phases**, not only at the end. Ask advisors for designs with exposed fractions from traces, not "what next". (Lessons §10)
- **Write stop rules into the first brief**: attempt caps, acceptance criteria, known risks. Consider handing a hard numerics problem to a different model after the second failed attempt. (Lessons §4, §5)

## Kernels: cuda-oxide, shipped as PTX

Write every custom CUDA kernel in Rust with cuda-oxide, not CUDA C++, Triton or NVRTC strings; vendor libraries are the only non-Rust device code. cuda-oxide's toolchain cost stays with the maintainers, and the published library builds on stable Rust:

- Take cuda-oxide from `praveenperera/cuda-rust` `master` at `08ac925a8` (never crates.io) with the toolchain that commit's `rust-toolchain.toml` pins (stable 1.99.1). Record the rev in the kernel crate's `Cargo.toml` and `Cargo.lock`, and install cargo-oxide at the same commit under a project-owned `CARGO_HOME`. Move the pin in its own change, never inside a climb whose guards diff PTX.
- Kernels live in a standalone crate outside the workspace, with its own `rust-toolchain.toml`. Only a build script run on a GPU box through the queue compiles it to PTX (and cubins where they pay).
- Commit the PTX and embed it with `include_str!` (`include_bytes!` for cubins); split per-tier crates if artifacts pass the crates.io limit.
- The host loads the PTX through the driver at run time (cudarc with dynamic loading), so users need no toolkit or GPU to build.
- CI checks the committed PTX against hashes of every input (kernel sources, cuda-oxide rev, toolchain, toolkit), so stale PTX fails without a GPU.

Before writing or porting a kernel, read [references/cuda-oxide.md](references/cuda-oxide.md): setup, known traps and per-build checks. Don't edit it during a task; log each new trap, and each slowdown you measured but couldn't explain, in the decision log with a `gotcha` Skill line. Compare builds with [scripts/sass_mix.sh](scripts/sass_mix.sh) and [scripts/nsys_kernel_diff.py](scripts/nsys_kernel_diff.py). Track D has the shipping details.

## Pick your track

| Track | You are | Read |
|---|---|---|
| A. Speed climb | Making kernels or a pipeline faster: a contest (OKC) or a hill climb on our runtimes | [references/track-a-speed-climb.md](references/track-a-speed-climb.md) |
| B. Replace library layers | Writing custom kernels for layers cuDNN or cuBLAS already runs (speakrs) | [references/track-b-library-layers.md](references/track-b-library-layers.md) |
| C. Native model runtime | Porting a whole model off PyTorch or ONNX Runtime (speakrs, scriptrs, sam3rs, whisperdrz) | [references/track-c-native-runtime.md](references/track-c-native-runtime.md) |
| D. Ship CUDA code | PTX tiers, shared CUDA crates, crates.io, Python wheels (cudakit) | [references/track-d-ship.md](references/track-d-ship.md) |
| E. Many GPUs or rented boxes | Several architectures, or vast.ai and similar boxes | [references/track-e-many-gpus.md](references/track-e-many-gpus.md) |
| F. Agent-run kernel work | Running workers, advisors or reviewers on kernel tasks | [references/track-f-agents.md](references/track-f-agents.md) |

New to directing kernel work? `R/cuda-optimization-primer/README.md` and its `director-card.md` give the vocabulary.
