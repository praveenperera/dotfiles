---
name: gpu-work
description: Start or run GPU kernel, CUDA, cuda-oxide or native-runtime work (custom kernels, speed climbs, porting a model off PyTorch or ONNX Runtime, profiling with nsys or ncu) from the research repo's GPU start page, defaulting to ambitious floor-capable designs over easy tuning. Use when the user invokes $gpu-work or /gpu-work.
disable-model-invocation: true
---

# GPU work

The rules live in the research repo. This skill says where to look and what to aim for; don't restate the rules elsewhere.

## Be ambitious

Agents drift to small, safe tuning (parameter sweeps, launch configs, small fusions) because it's easy to measure. That is the second pass, not the plan. Build the gap-and-floor table, and where a component is far from its hardware floor, build the design that can reach the floor: spend about 70% of the budget on one or two such rewrites and 30% on tuning and integration. Judge a rewrite by its ablation ceiling and drop it only for a proven blocker, never for its first end-to-end number. A climb that only tunes has skipped the big rows.

## Kernels: cuda-oxide, shipped as PTX

Write every custom CUDA kernel in Rust with cuda-oxide, not CUDA C++, Triton or NVRTC source strings. cuda-oxide needs a pinned nightly and a CUDA toolkit; that cost stays with the maintainers, and the published library builds on stable Rust:

- Take cuda-oxide from the tip of its git `main`, never a crates.io release. When you start kernel work or rebuild the PTX, move to the current tip and to the nightly that tip pins in its own `rust-toolchain.toml`. Record the commit (a `rev` or the lockfile) so the PTX check knows which cuda-oxide built the committed files.
- Kernels live in a standalone crate outside the workspace, with its own `rust-toolchain.toml` matching cuda-oxide's nightly. Only a build command (an xtask or script run on a GPU box) compiles it, once per area and tier, to PTX and, where it pays, cubins.
- Commit the generated PTX and embed it with `include_str!` (`include_bytes!` for cubins). If the artifacts would pass the crates.io upload limit, split them into one crate per PTX tier: an sm75 baseline, plus higher tiers only where they unlock math the baseline can't use.
- The host library loads the PTX through the driver at run time (for example cudarc with dynamic loading), so users need no nightly, no CUDA toolkit and no GPU to build it, and the installed driver JIT-compiles the PTX.
- CI checks the committed PTX against hashes of every input that can change it (kernel sources, cuda-oxide rev, toolchain, toolkit), so stale PTX fails without a GPU.

Track D on the start page has the details.

## Where to look

1. Read `~/code/research/gpu-kernels/README.md`: "Every project", then the track or tracks that fit. Open only the notes your track links.
2. Speed climb on an existing pipeline: follow `~/code/private-workbench/playbooks/gpu-hill-climb.md` and the repo sheet in `~/code/private-workbench/ops/<repo>/hill-climb/README.md`.
3. Delegating to a worker: fill `~/code/private-workbench/playbooks/kernel-worker-brief-template.md`, copying in only the rules that apply and naming the ambitious design to build first.
4. All GPU work, including tests, nsys and ncu, goes through the homebased queue on `gpu0`.
5. At the end of each phase and before finishing, record results and lessons as in "Recording lessons" on the start page.

If you are a worker with a brief, the brief replaces steps 1-3, but the ambition rule still applies.
