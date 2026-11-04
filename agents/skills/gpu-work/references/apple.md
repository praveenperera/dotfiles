# Apple silicon

Metal backends and native runtimes on Apple GPUs, and Core ML or Neural Engine offload. SKILL.md "Be ambitious" and "Every project" apply, as do Tracks A and C; this file replaces the CUDA-only parts (cuda-oxide, PTX, nsys, ncu, compute-sanitizer). Shorthands as in SKILL.md.

Exercised on an M4 Pro only. Expect unlisted traps on M5 matrix units, other M generations, iOS, Metal's API and shader validation layers, atomics and indirect command buffers.

## Pick the stack

- **Default to native Metal:** an objc2-metal host and MSL kernels behind the same public API as the CUDA backend, selected at compile time on macOS. MLX's lazy graph and per-op dispatch give up fusion and a single command stream, and its GEMM is no faster. Whole-model Core ML means opaque compilation, FP16 and fixed shapes.
- **Core ML can't host a custom kernel.** An ML Program has no custom-op hook, so Metal and Core ML mix only as whole-graph handoffs, with outputs written into Metal buffers through output backings.
- **Price the Neural Engine as a second engine, not a faster one.** Its GEMMs match the GPU, but activation-by-activation matmuls and long global attention run several times slower, the first compile takes minutes, and its FP16 error is about double the GPU's BF16. It keeps most of its rate while the GPU runs, so the gain is from splitting work. Read `MLComputePlan` placement and time each block type before planning a split; a compute-plan preference isn't runtime placement.

## Floors

- **Measure the roofs on each chip.** Apple publishes no FP32 rate, and measured bandwidth sits about 10% under spec.
- **Choose precision per stage with the Apple ratio.** FP32 `matmul2d` runs within about 18% of BF16, so stages outside the big GEMM-bound trunk can stay FP32 in both modes.
- **Unified memory removes copies, not waits.** Each host wait drains the serial encoder: move picks and argmaxes onto the GPU and wait once a pass.

## Kernels: MSL on Metal 4 tensor ops

- Embed the `.metal` sources with `include_str!` and compile them at load with `newLibraryWithSource` (language 4.0, `MTLMathMode::Safe`), so users need no Metal Toolchain. Build every pipeline on a background thread during the weight load; Metal's shader cache makes warm loads cheap. `xcrun metal` is an offline compile check only, and needs a separately downloaded Xcode component.
- Check each dispatch's bindings against the pipeline's reflected argument list, once per kernel, and check every command buffer's status before its results update host state, so a GPU fault or validation error fails the call instead of returning stale data.
- **GEMMs: MPP `matmul2d` with cooperative-tensor epilogues** before a hand simdgroup_matrix kernel; it reaches the BF16 roof where hand kernels reach about 85%.
- **Attention: write the flash loop on `matmul2d` cooperative tensors** (`reduce_rows`, `map_iterator`, `get_left_input_cooperative_tensor`) before hand-tiling simdgroup_matrix. When each SIMD group computes its own skinny QK tile, have the threadgroup's SIMD groups compute one 64 x 64 tile into threadgroup memory instead, and split keys until there are about 16 SIMD groups a core. Always time the unfused score GEMM, softmax and P@V GEMM too: at head dim 256 with 256-wide values it beat every fused variant by 2x.
- **Fill the GPU.** When a tile grid falls under about 8 threadgroups a core with long K or many keys, use split-K or split-KV plus a combine kernel.
- Convs: `convolution2d` has no padding, so run `matmul2d` over the taps of a padded input. Permute transposed-conv weights tap-major so each tile stores contiguous channels.
- No grid-wide sync: persistent kernels lose when too few threadgroups stay resident, and a long recurrence may belong on the CPU.

## Correctness traps

- **Run Metal's validation layers as the sanitizer.** At the end of each phase, run the repo's kernel and stage tests through the queue under [scripts/metal_validate.sh](../scripts/metal_validate.sh), which sets `MTL_DEBUG_LAYER=1 MTL_SHADER_VALIDATION=1` and fails on validation errors even when the tests exit 0. Out-of-bounds reads rarely fault on Apple GPUs and can land in another pooled buffer, so ragged tiles and split ranges pass parity until a shape or pool layout changes. Shader validation is roughly memcheck: there is no race, init or sync check. Never time under either layer. Confirm on the first run that the `matmul2d` kernels are covered.
- `simdgroup_matrix::thread_elements()` returns a reference: `auto e = ...` copies the fragment and silently drops writes. Use `thread auto& e`.
- **A cooperative-tensor shape that compiles can return garbage** (a 256-column `matmul2d` destination on one SIMD group; 32 or 64 query rows a SIMD group). Split P@V into chunks of at most 64 columns, each with its own named accumulator; arrays of cooperative tensors don't compile. Give every new tile shape an f64 kernel test before timing it.
- Gate precision changes on the slowest-to-diverge end metric: FP16 attention operands passed the kernel test and image parity, then collapsed video tracking.
- The dispatch argument check compares element types: take `device const int*` and cast rather than binding an I32 buffer to a struct pointer.
- MPSGraph: a recurrent descriptor made through `NSObject` init sets every activation to none and returns NaN. Set them explicitly.

## Profiling and timing

- **Time dispatches with counter-sample timestamps.** Apple GPUs sample only at encoder boundaries, so in profile mode give each dispatch its own compute pass and charge it `end - max(start, previous end)`. Waiting after each dispatch lets the GPU clock down and inflates small kernels; summing back-to-back command buffers double-counts their overlap. Check the profile's sum against an unprofiled run, and quote headline numbers from unprofiled runs.
- Instruments' Metal System Trace times dispatches in a normal run but needs Developer mode (`DevToolsSecurity -status`).
- MPSGraph's `commitAndContinue` splits command buffers: time the graph by wall clock, not the last buffer.
- **The queue doesn't see every GPU user** (iOS simulators, browsers, other projects' Core ML jobs). `gpu-ab` runs on the Mac and drops arms where other Metal clients used the GPU; give each repo its own Mac adapter, named apart from its CUDA one (the gpu-ab README has an example). Process medians drift 2-7% on the Mac, so take A/A noise from the Mac, never from a CUDA run.

## Runs and memory

- Mac GPU jobs, including validation runs, go through homebased `gpu0` with `preempt: wait`.
- Core ML caches compiled models per executable name, so a harness that rebuilds in its own tree can run different code from the one you meant.
- Core ML keeps a GPU buffer set for each concurrent prediction that never shrinks while the model lives: cap predictions in flight.

Read: `SMR/sam3rs-decisions.md` (Metal entries from 2026-11-03), `R/cuda-kernels/speakrs-metal-kernels-2026-10-04.md`, `R/mac-llm-decode/roof/README.md` and `findings-2026-10.md`, `~/code/research/speakrs/coreml-memory-cap-2026-11-02.md`, `R/gpu-ab/README.md`.
