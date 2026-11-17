---
name: gpu-work
description: "Rules and workflow for GPU kernel and native-runtime work: writing or optimizing CUDA or Metal kernels, speed climbs on GPU pipelines, porting a model off PyTorch or ONNX Runtime, replacing cuDNN or cuBLAS layers, Core ML offload, profiling with nsys or ncu, and tuning an LLM server (vLLM, SGLang, llama.cpp, MLX) for tok/s, TTFT, KV cache or speculative decoding. Use for that work or on $gpu-work or /gpu-work. Not for ML training runs or GPU jobs that change no kernel, runtime or server."
---

# GPU work

Shorthand: `PW` = `~/code/private-workbench/`, which holds the hill-climb playbook, the worker brief template, and one sheet per repo with its build, bench, gate, scripts and rejected ideas.

## How to run the work

1. Read "Be ambitious", "Every project" and "Kernels" below, then the tracks that fit in "Pick your track".
2. Start the project's decision log before the first design choice: [references/recording.md](references/recording.md).
3. Speed work: open `PW/playbooks/gpu-hill-climb.md` and `PW/ops/<repo>/hill-climb/README.md` before the first change. nsys ranks the gaps; run ncu on each one you act on.
4. Delegating: fill `PW/playbooks/kernel-worker-brief-template.md` with the rules that apply, and name the ambitious design to build first.
5. Run all GPU work, including tests, nsys, ncu and compute-sanitizer, through the homebased GPU queue, and nothing else: hold the GPU only while it is actually computing. Build, compile, stage data, score and report outside the lease. Build first in a CPU `homebased task run` task (PTX, cubins, `cargo build`, test binaries with `cargo test --no-run`), then submit the GPU job with `resource job submit --after <task-id>` so its steps only run prebuilt binaries. A build inside a GPU job holds the GPU idle while other jobs wait; the queue refuses steps that are plainly builds and cancels or yields a job whose processes hold no CUDA context for 5 minutes. Label a job `preempt: yield` only when its step polls `HOMEBASED_YIELD_FILE`; otherwise use `wait`. Import every module each benchmark venv loads on the CPU before queuing. A green job only means its wrapper exited 0: read each step's output for missing packages, a failed program under nsys, a test binary built without its `cuda` feature (check `--list`), or a tool printing usage. Stop a probe inside a step with `kill $!`; a `pkill -f` pattern in the step's own command line kills the step.
6. After each phase, revert, parked attempt or user correction, log the decision and record results as in [references/recording.md](references/recording.md).

A worker with a brief skips steps 1, 3 and 4 and reports decisions and rule candidates; the root records them.

## Be ambitious

Small, safe tuning (parameter sweeps, launch configs, small fusions) is the second pass, not the plan.

- **Build the gap-and-floor table on day 0.** Per case: the measured hardware floor (bandwidth or FLOPs), our time, the target, and the achievable ceiling from ncu (the vendor's best kernel's tensor-pipe or DRAM active %). Build floors from dense peaks and sustained rates: the RTX 5090's BF16 dense peak is about 209.5 TFLOPS (419 is the sparse figure), and an H200 sustained 3.6-3.9 of its 4.8 TB/s spec. A floor built from spec sheets makes every case look further from the floor than it is.
- **In a compute-bound stage, fusion saves traffic that is already hidden.** Compare the memory and compute floors before fusing: with a 13 ms memory floor under a 63 ms compute floor, a perfect direct conv could gain only 13%, and the open headroom was algorithmic (Winograd).
- **Rewrite what is far from its floor.** More than ~1.3x from target or under ~70% of floor means rewrite; tune only within ~15% of floor. Spend about 70% of the budget on one or two rewrites.
- **Judge a rewrite by its ablation ceiling** (drop compute, reductions or secondary loads and re-time), not its first end-to-end number, and drop it only for a proven blocker. A native runtime is a go when its measured ceiling beats the best existing configuration by ~1.3x.
- **Discount small ceilings.** What gets built lands well under the ablation: ceilings of x1.012-1.029 built to x0.96-1.00, so a sub-step's ceiling needs 2-3x the acceptance gate before a build is worth starting. Large ceilings (a 1.3-1.6x main loop) hold up better.
- **Don't stop a climb because the remaining ideas take hours, and don't pick the cheapest experiment that can clear the gate.** That selects exactly the changes the noise hides. One climb stopped twice on "multi-hour, poor priors" when only those rewrites could matter. Give a rewrite at least a day of runway, and rank next steps by whether they are on the rewrite path.

## Every project

- **Cut work first.** Remove, precompute, reuse or combine work within the accuracy gate, then speed up what remains.
- **Measure the end-to-end payoff before building.** Profile the whole pipeline on the target GPU, do the Amdahl arithmetic, and confirm with a rough prototype. Baseline against the best existing deployment (MPS workers, open upstream patches), not defaults.
- **Diagnose before you pick a design.** nsys says where the time goes; ncu `--set full` says why. Cite pipe-active %, top stalls, occupancy and where loads hit before writing a hypothesis. Time competing designs in one harness.
- **Read the kernels the incumbent actually runs** in nsys, not the flags you set: libraries pick algorithms and precisions per device and batch. Judge a path against its own peak: TF32 tensor GEMMs reach the GeForce FP32 peak, SIMT FP32 doesn't.
- **Gate each pass's structure the first time it runs.** Time it against a named reference at once, and fix the structure before anything else when it is over 2x slower. An agent wrote a correct attention backward with scalar FP32 loops and a dense `[B,H,T,T]` buffer at 17 ms against 0.27 ms for a tensor-core version. Put the structure ("tensor cores in every product, no dense T x T buffer") in the brief.
- **Gate accuracy against an audited reference**: f64 or the framework's own output, after checking its real precision, dropout and determinism. Run it twice in one process and once in another. Gate on the end metric, including low-confidence cases, and on the worst single item, not only the aggregate: FP8 trunk operands improved aggregate DER while moving single files by up to 5.9 points. Calibrate the noise band first by rerunning the reference with one exact-in-real-arithmetic perturbation per stage; autoregressive pipelines flip tokens, so don't promise byte identity before that.
- **Make parity gates fail loudly.** Assert output lengths, prefill outputs with NaN, never compare with `zip` or `f32::max` alone, and make some gate metric read every field the user sees.
- **Time only on an idle GPU through the queue** by `PW/playbooks/gpu-hill-climb.md` §4 step 5: frozen binaries, paired ABBA runs against A/A noise, every arm's output checked. Use [gpu-ab](https://github.com/praveenperera/gpu-ab) for every timed comparison. The lease doesn't reserve the CPU: host load of 17-63 pushed A/A noise to 1.3-12%, so read the host load in gpu-ab's `result.json` before trusting a verdict. Print the metric finely enough: a bench that rounded to 0.1 ms put a 1.4% step on a 7 ms case, and two decimals took A/A noise from 1.7% to 0.15%. Split wins into same-output and changed-output files. Re-bench at the final commit.
- **Know how each profiler lies.** nsys inflates launch-bound work and needs `--cuda-graph-trace=node` for graphs; a tail after the last GPU range is host work. ncu locks base clocks and flushes L2: pass `--clock-control none` and pick `--cache-control` by whether the working set stays in L2. Where counters are denied, bound a kernel's parts with removal variants. Single-process GPU-busy doesn't bound throughput under MPS. Attribute GPU time to the outermost harness NVTX range. Headline numbers come from untraced runs.
- **Hybrid by default.** Keep the vendor library wherever it wins, and try its epilogues and options before hand-written GEMMs; check that the epilogue really fused (cuBLASLt's TF32 GELU+bias ran as a separate kernel, while bias-only and FP16/BF16 epilogues fused). Price a change that depends on the library's kernel choice over the real shape distribution.
- **Correctness and speed don't carry across architectures.** Run sanitizers on at least two. Bind a tuned change to its device, or check it on another, before it ships in a shared PTX tier.
- **Review correctness against upstream between phases.** Ask advisors for designs with exposed fractions from traces, not "what next".
- **Put stop rules in the first brief**: attempt caps, acceptance criteria, known risks. Hand a hard numerics problem to a different model after the second failed attempt.

## Kernels: cuda-oxide, shipped as PTX

Write every custom CUDA kernel in Rust with cuda-oxide, not CUDA C++, Triton or NVRTC strings. The exceptions are vendor libraries and kernels contributed into a C++ project's own tree (an ONNX Runtime CUDA EP kernel). cuTile Rust needs a CUDA toolkit at run time, so it can't ship toolkit-free.

- Take cuda-oxide from `NVIDIA/cuda-rust` `main` at `8d8254869`, never crates.io (`cuda-oxide` there is an unrelated host wrapper), a release tag (v0.2.1 lacks warp tensor-core instructions) or a fork, with the toolchain its `rust-toolchain.toml` pins (stable 1.99.1). Record the rev in the kernel crate's `Cargo.toml` and `Cargo.lock`, and install cargo-oxide at the same commit under a project-owned `CARGO_HOME`. Move the pin in its own change.
- Kernels live in a standalone crate outside the workspace, with its own `rust-toolchain.toml`. Its build script compiles it to PTX (and cubins where they pay). The build needs no GPU, so run it, like every `cargo build`, `cargo oxide`, `build-ptx.sh`, `nvcc` or `ptxas` step, as a CPU `homebased task run` task, never in the GPU queue.
- Commit the PTX and embed it with `include_str!` (`include_bytes!` for cubins); split per-tier crates past the crates.io limit.
- The host loads the PTX through the driver at run time (cudarc with dynamic loading), so users need no toolkit or GPU to build.
- CI checks the committed PTX against hashes of every input (kernel sources, cuda-oxide rev, toolchain, toolkit). Hash rustfmt-normalized, comment-stripped sources so a comment edit doesn't mark the PTX stale, and have the check name the input that changed.
- Mark committed PTX, ptxas logs and the hash manifest `binary` in `.gitattributes`, so git never merges them silently: after a merge, rebuild and diff against each parent.

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
| G. LLM serving tuning | Hill-climbing a model server's tok/s, TTFT, startup or KV capacity, mostly through its config | [references/track-g-llm-serving.md](references/track-g-llm-serving.md) |
