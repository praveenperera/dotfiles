---
name: gpu-work
description: Start or run GPU kernel, CUDA, cuda-oxide or native-runtime work (custom kernels, speed climbs, porting a model off PyTorch or ONNX Runtime, profiling with nsys or ncu) from the research repo's GPU start page, defaulting to ambitious floor-capable designs over easy tuning. Use when the user invokes $gpu-work or /gpu-work.
disable-model-invocation: true
---

# GPU work

The rules live in the research repo. This skill says where to look and what to aim for; don't restate the rules elsewhere.

## Be ambitious

Agents drift to small, safe tuning (parameter sweeps, launch configs, small fusions) because it's easy to measure. That is the second pass, not the plan. Build the gap-and-floor table, and where a component is far from its hardware floor, build the design that can reach the floor: spend about 70% of the budget on one or two such rewrites and 30% on tuning and integration. Judge a rewrite by its ablation ceiling and drop it only for a proven blocker, never for its first end-to-end number. A climb that only tunes has skipped the big rows.

## Where to look

1. Read `~/code/research/gpu-kernels/README.md`: "Every project", then the track or tracks that fit. Open only the notes your track links.
2. Speed climb on an existing pipeline: follow `~/code/private-workbench/playbooks/gpu-hill-climb.md` and the repo sheet in `~/code/private-workbench/ops/<repo>/hill-climb/README.md`.
3. Delegating to a worker: fill `~/code/private-workbench/playbooks/kernel-worker-brief-template.md`, copying in only the rules that apply and naming the ambitious design to build first.
4. All GPU work, including tests, nsys and ncu, goes through the homebased queue on `gpu0`.
5. At the end of each phase and before finishing, record results and lessons as in "Recording lessons" on the start page.

If you are a worker with a brief, the brief replaces steps 1-3, but the ambition rule still applies.
