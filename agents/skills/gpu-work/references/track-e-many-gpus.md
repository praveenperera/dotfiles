# Track E: many GPUs or rented boxes

Supporting several architectures, or working on vast.ai and similar boxes. `R` = `~/code/research/gpu-kernels/`.

- Rent one GPU per target generation early. Ada, sm_120 and the A100 each found a bug or a loss the others couldn't.
- Profile the whole pipeline on each new GPU before optimizing; the A100 loss was scalar FP32 convs where cuDNN used TF32.
- Cooperative grids start differently by architecture and can deadlock under MPS. Mixed-size loads and stores are a data race. Always use `cvta_generic_to_shared_u32`.
- Billed boxes must never idle: smoke-test the package on a cheap box, bootstrap only the toolchain, cap uploads, and pull evidence on a schedule.
- Never put secrets or agents on a rented box. Wrap GPU work in a shared flock, kill only PIDs you started, and wipe the box when done.

Read: lessons §2, §4 and §6, `R/cuda-kernels/gpu-platforms-modal-beam-vast-2026-10-07.md`, `R/cuda-kernels/speakrs-cuda-findings-log.md`.
