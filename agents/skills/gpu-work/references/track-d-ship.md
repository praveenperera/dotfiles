# Track D: ship CUDA code

PTX tiers, shared CUDA crates, crates.io, Python wheels (cudakit). The build layout itself is in SKILL.md "Kernels". `R` = `~/code/research/gpu-kernels/`.

- The PTX ISA version sets the driver floor: nvcc 13.0 emits ISA 9.0, which r580 drivers can JIT, and cuda-oxide's llc-21 path emitted 7.0 for sm_80 and 8.7 for sm_120. Pin the toolchain that writes the PTX and check the `.version` line in the build script.
- A low PTX tier JITs forward well. On SAM 3, compute_80 PTX cost 2.6-3.0% against compute_120 PTX. Architecture-specific PTX pays only where it unlocks math the portable tier can't use.
- Make tiers open-ended before the first release, and make "driver only" a runtime profile, not a Cargo feature.
- Embed each artifact once. Split per-tier artifact crates to fit crates.io's 10 MiB limit, and check package sizes in CI.
- GPU tests run only on request, so `cargo test` can't bypass the GPU queue. Keep CUDA crates out of non-CUDA dependency trees, and test from a fresh clone on each platform.
- A CUDA Python wheel can be about 4 MB: committed PTX plus a dependency on `nvidia-cublas-cu12`.

Read: lessons §1 (cuda-oxide field report), §9 (cudakit), and §8 "Build and release"; `~/code/cudakit/docs/design.md`; `R/cuda-kernels/rust-cuda-ecosystem.md`; `R/cuda-oxide-intrinsic-floors/README.md`; [cuda-oxide.md](cuda-oxide.md).
