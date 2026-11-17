# Track D: ship CUDA code

PTX tiers, shared CUDA crates, crates.io, Python wheels. The build layout is in SKILL.md "Kernels"; the PTX `.version` check is in [cuda-oxide.md](cuda-oxide.md) "Setup". Shorthands as in SKILL.md.

- The PTX ISA version sets the driver floor (nvcc 13.0 emits ISA 9.0, which r580 drivers JIT; CUDA 13.x needs driver 580 or newer, though a CUDA 13 compiler still builds offline). Pin the toolchain that writes the PTX. On a new platform test cubin loading and forced PTX JIT separately; they fail separately.
- Ship an sm75 baseline tier; a low PTX tier JITs forward with a few percent loss. Add architecture-specific tiers only where they unlock math the portable tier can't use: sm80 over sm75 PTX gave 1.00-1.05x on Ada, and 1.29x on an A100 only for the TF32 stage that gained tensor-core `mma`.
- **Budget first-load JIT.** A 650 KB PTX module took about 1.4 s to JIT once, then came from `~/.nv/ComputeCache`. One tier's PTX was 0.81 MB (0.14 MB gzipped) against 2.31 MB for a six-arch fatbin, so embedding every tier is cheap; only the selected one JITs. Warm up before timing.
- **Cubins need one build per (PTX tier, arch at or above it)**, since the loader takes exact-cc cubins only: sm75 PTX needs six. Make selection prefer the qualified artifact, and record the loaded cubin hash, its PTX hash and the ptxas version.
- **Stale embedded PTX is easy to time by mistake.** `rsync -a` keeps old mtimes, so cargo reuses a binary whose `include_str!` holds the previous PTX. Touch copied PTX before building, and record the embedded PTX hash in every timing record.
- Make tiers open-ended before the first release, and make "driver only" a runtime profile, not a Cargo feature.
- **Algorithmic tuning knobs choose among qualified, deterministic configurations**: outputs may differ between configurations but never run to run. Pure launch knobs that keep arithmetic order (grid caps, stream priority) are free. Key a user-run tune file on GPU name, cc, SM count, driver and artifact hashes.
- Embed each artifact once, split per-tier artifact crates to fit crates.io's 10 MiB limit, and check package sizes in CI.
- GPU tests run only on request, so `cargo test` can't bypass the GPU queue. Build the test binaries in a CPU task (`cargo test --no-run`) and run only the built binary in the GPU job, chained with `--after`. Keep CUDA crates out of non-CUDA dependency trees, and test from a fresh clone on each platform.
- A CUDA Python wheel can be about 4 MB: committed PTX plus a dependency on `nvidia-cublas-cu12`.
