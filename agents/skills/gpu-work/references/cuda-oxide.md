# cuda-oxide watch-outs

Traps that cost correctness or speed in cuda-oxide kernels, and the checks that catch them. Add to it after each run: one line per trap with the fix, plus the commit it was seen at, since some are bugs that later commits may fix. Recheck old entries when you move to a newer cuda-oxide. Evidence for each entry is in the run logs under `~/code/research/gpu-kernels/cuda-kernels/`. A slowdown you measured but couldn't explain goes under "Open slowdowns" with its suspects, until a test confirms a trap or clears it.

## Coverage

The entries come from one codebase (sam3rs, about 50 kernels: elementwise, norms, convs, flash attention on BF16 and TF32 tensor cores), one cuda-oxide commit (`2d47e43`) and one GPU (RTX 5090, sm_120). The sm_80 tier was checked in SASS only, never run on Ampere. Not yet exercised, so expect unlisted traps there: atomics, grid-wide reductions and cooperative launches, TMA and `wgmma` (sm_90+), FP8, thread-block clusters.

## Setup

- Use a project-owned `CARGO_HOME` for cargo-oxide: the backend cache in `$CARGO_HOME/cuda-oxide/` is shared, and installing another commit rebuilds it under other projects.
- Pin the commit in the kernel crate's `Cargo.lock` and install cargo-oxide at that commit, so the device library and backend match. The toolchain is whatever that commit's `rust-toolchain.toml` pins.
- `cargo oxide build` builds the backend when its cache is missing or stale, so a build script needn't call `cargo oxide setup`. Run inside a standalone crate, setup prints "Backend build succeeded, but Cargo reported no librustc_codegen_cuda.so artifact" yet leaves a working cache; outside any project it refuses.
- PTX is only written when the crate itself recompiles: `cargo clean -p <crate>` first.
- The PTX ISA `.version` sets the minimum driver; check it in the build script (llc-21: 7.0 for sm_80, 8.7 for sm_120).

## Speed traps

- **Generic `#[device]` fns are never inlined** (`2d47e43`, upstream #1471): they stay a `.func` and their pointers arrive generic (generic `LD` instead of `LDS`/`LDG`, flash attention 25-72% slower). Write generic bodies as plain `#[inline(always)]` fns, and unroll their loops with `cuda_device::thread::__unroll_config::<0>()` as the loop body's first statement (0 = fully, N = by N). That marker is the undocumented call `#[unroll]` expands to, so recheck it after every cuda-oxide upgrade (the build's no-`call` check and a SASS look catch a break).
- **64-bit index math** (`i64` indices, as ported from `long long`) compiles `/` and `%` to slow 64-bit helper sequences. 32-bit coordinates (`thread::index_1d_u32`, `element_thread32`, `#[launch_contract(coordinates = u32)]`) divide inline: 20-70% fewer instructions in elementwise and conv kernels.
- **`f32::max`/`min` become compare + select** (`2d47e43`, upstream #1594), not one `max.f32`: use libdevice `__nv_fmaxf`/`__nv_fminf`.
- **No automatic partial unrolling of runtime-bound loops**, which nvcc does (a `blockDim.x`-step loop gets 4x from nvcc and stays rolled here, one load round trip per iteration: flash tile staging 27% slower, `2d47e43`). The full-unroll marker is checked before inlining, so a bound that is constant only after inlining is refused, and LLVM ignored a partial count. Loop to a fixed upper bound fully unrolled and skip the extra iterations with a test that folds after inlining.
- **`ptx_asm!` loads and stores keep their source order**: asm is opaque and a store with `clobber("memory")` blocks moving any later load above it, so neither LLVM nor ptxas overlaps the loads of an unrolled load-store loop. Write all the loads into registers first, then all the stores.
- **Iterating a local array with `for x in arr`** puts it on the stack: write the operations out.
- **`core::iter::Zip` over a slice iterator and a view iterator is not inlined** (`2d47e43`): a `call` and a 72-byte stack. Use cuda-device's own `zip_exact`/`ZipView32` or a counted loop over views.
- **Selecting between a real view and `RowView32::empty()`** (a null pointer) turns the loads into generic `LD`, and `Option<RowView32>` puts it on the stack: keep an optional input as an empty slice and test `is_empty()`.
- **Bounds-checked `slice[i]` in a loop can stop LLVM unrolling it** (14 loads issued together became 2); a view's `get(i)` checks against the loop bound once and unrolls. Read rows through `MatrixView32`/`RowView32`. Checks also cost registers (`roi_align` 40 to 45 for four checked loads), and the cuda-oxide book measures a naive GEMM 2.4x slower with checked indexing.
- **Shared addresses for `ldmatrix`: 32-bit math, lane term first, constant last** (upstream #1235). Computed in 64 bits and truncated, or with the constant added first, the offset becomes an `or` ptxas can't fold into the immediate (about 150 more instructions in a flash kernel, `2d47e43`).
- **A bounds check whose trap depends on the thread marks the warp divergent**: ptxas then wraps every later shuffle and `mma` in WARPSYNC/ENDCOLLECTIVE. On sm_80 the same shows as 5-6 SASS `CALL`s, ptxas's divergent-shuffle fallbacks. In warp-cooperative kernels, check once up front with a block-uniform condition and index unchecked after.
- **A pointer merged with null (through `if`/`select` or an empty view) becomes a generic `LD`**: load inside the branch instead.
- **Deriving a row length from a slice length in a block-per-row kernel** (`x.len() / gridDim.x`) puts an integer division before the first load of every block. Pass the row length as a param and let the view check it (layernorm_bf16 3% faster, `2d47e43`). One-thread-per-element kernels only compare against the length, so they are fine.
- **An inline helper returning `Option<(struct, views...)>`** puts the tuple on the stack (`2d47e43`, likely upstream #1590: a pointer-niche enum payload lowers to a local depot): a 304-byte frame with `LDL`/`STL` in every flash kernel that called it. Return a plain tuple and `unreachable_unchecked()` past the one up-front check.
- **Moving a kernel's code into shared helpers can change its SASS** with no stack or spills (`2d47e43`: +8 instructions in the BF16 flash kernels from a different block layout). When a kernel must stay unchanged, give the new kernel its own copies and diff the old kernel's PTX to byte-identical.
- **`cvt.rna.tf32.f32` costs 3 instructions per value on sm_120** (ptxas: an `FSETP` NaN/Inf guard, a predicated add of `0x1000`, a mask), which shows up as many `FSETP` in TF32 kernels. A bit-trick rounding saves only the guard and turns the canonical NaN `0x7fffffff` into -0.
- **`warp::reduce_max_f32` uses `f32::max`** (`2d47e43`, upstream #1594), so it carries the compare + select; write the butterfly with `fmaxf`. `reduce_sum_f32` is fine.

## Open slowdowns

Measured, cause not confirmed. Test the suspects before trusting a fix.

- **LayerNorm about 1 ms slower than raw-pointer code** (`2d47e43`, sam3rs: `layernorm` plus `layernorm_inplace` +7%, `layernorm_bf16` +8% before the row-length fix, which recovered 0.43 of about 1.5 ms). Untested suspects: `&[T]` loads lower to `ld.global.nc` (`LDG.E.CONSTANT`) where raw pointers gave plain `LDG.E`, and the select from `get(c).unwrap_or(0.0)` on each load.
- **`flash_attn_rpb_d32` behind nvcc** (`2d47e43`): +28% against nvcc after the port, then 9% faster in the rewrite, so about +16% if the two profiles compose; never measured head to head. It stages FP32 tiles and loads relative-position-bias rows, and the port used 152 registers against nvcc's 128 (kv16_d32 109 against 80, bf16_d64 168 against 149). Compare SASS against the nvcc build before guessing.
- **`transpose` +0.22 ms after the one-thread-per-element rewrite** (`2d47e43`): each thread gathers one output element, so reads are strided. Probably the layout choice rather than cuda-oxide; a tiled shared-memory transpose would tell.

## Correctness traps

- `convert::f32_pair_to_packed_bf16` **truncates**; `cvt_bf16x2_f32` rounds to nearest even.
- A closure capturing shared-memory pointers fails MIR verification (`2d47e43`): use a `macro_rules!`.
- **An unchecked read evaluated before the check that proves it.** `x.unwrap_or(unsafe { *w.get_unchecked(c) })`, or a `let` above an `if w.is_empty()`, reads eagerly: UB on an empty or short slice. Keep each unchecked read inside the branch that proves it in bounds.
- **The launch contract is only checked by cuda-oxide's own launcher.** Another host (cudarc, the driver API) must keep `grid x block <= 2^32` for 32-bit coordinates and push a length after every slice pointer itself.

## API gaps and limits

- `#[unroll]` works only directly in `#[kernel]`/`#[device]` bodies, not inside `macro_rules!` bodies, and not on loops with a non-constant step (warns, stays rolled).
- `#[unroll(N)]` on a loop with a bounds check warns about multiple exits (the trap branch counts) and is ignored; LLVM often unrolls it unaided.
- `ThreadIndex32` is not `Copy`, so each output `DisjointSlice` needs its own witness: call `thread::index_1d_u32(lc)` once per slice.
- `DisjointSlice` params are declared `.align 1` whatever `T` is (upstream #1098), so wide stores can't be inferred from them.
- No block-indexed output space: a block that owns a row writes through `DisjointSlice::get_unchecked_mut` with a SAFETY comment. A kernel that reads and writes one buffer can't take it as both `&[T]` and `DisjointSlice<T>` (that aliasing is UB): split an in-place kernel that takes only the `DisjointSlice` (it reads through the slice's raw pointer, since two closures can't share `get_unchecked_mut`).
- A `DisjointSlice` is written only at the thread's own index: a scatter kernel becomes a gather over the output (so its reads may stride), and a thread with two outputs either takes a second witness or writes one combined buffer.
- No bf16 type in cuda-device: `u16` bits plus the `convert` functions, behind a newtype.
- No `static mut SharedArray` in generic fns: declare tiles per kernel and pass pointers in.
- No `[T; D / 16]` in generics without `generic_const_exprs`: pass the derived length as another const param.
- No `f32::mul_add` in `no_std` (use `float::fma_rn_f32`); libdevice math through `unsafe extern "C" { fn __nv_erff(x: f32) -> f32; }`, which matches nvcc's results. `__nvvm_reflect("__CUDA_FTZ")` resolves to the non-FTZ branch, nvcc's default.
- Wide shared stores (`st.shared.v4`) and 16-byte global loads need `ptx_asm!`. Wide global stores don't: a `repr(C, align(16))` element type in a `DisjointSlice` lowers to `st.global.v4`.
- No safe empty `DisjointSlice` (`from_mut_slice(&mut [])` is unsafe); `launch_contract(block = ...)` emits `.reqntid` and drops `.maxntid`.
- Tensor-core wrappers are split across files: `cuda_device::wmma` has `mma_m16n8k16_f32_bf16`, `_f32_f16`, `_f16_f16` (FP16 accumulate, from `generated/register_mma.rs`), `mma_m16n8k8_f32_tf32` and `mma_m16n8k4_f32_tf32` (no k16 TF32 shape; `convert::cvt_rna_tf32_f32` rounds), `ldmatrix_*` and the `mma_frag` accumulator layout helpers (`2d47e43`). Search the crate source, including `generated/`, before writing `ptx_asm!`.
- `ldmatrix_x4_shared_u32` over FP32/TF32 rows (row stride D + 4 words keeps it conflict free) gives lane (g, t) word t of row g, which is exactly the m16n8k8 TF32 A and B fragment layout; the S accumulator then serves as the P @ V A fragment as `{s0, s2, s1, s3}` with V read at rows 2t and 2t + 1.
- A host that launches slice-ABI kernels should check its arguments against each `.entry` param list read from the PTX text it loaded (`.u64 .ptr` address, `.u64` length, `.u32`, `.f32`; `i32` params are declared `.u32` too), once per kernel. Probing `cuFuncGetParamInfo` past the last param returns `CUDA_ERROR_INVALID_VALUE`, which compute-sanitizer reports as an API error for every kernel, and `cuFuncGetParamCount` needs CUDA 13.2.
- `#[kernel] pub unsafe fn` with raw pointers and plain scalars exports a C-like entry, so hosts written for CUDA C++ kernels need no launch changes.

## Checks on every build

Scripts in this skill's `scripts/` folder:

- `sass_mix.sh REF.ptx NEW.ptx [ARCH] [KERNEL...]`: calls left in each PTX, then per kernel the registers, stack, spills, SASS instruction count and trap opcodes (generic `LD`, `FSETP`+`FSEL`, `LDL`/`STL`). With no kernels, every kernel whose registers or spills changed. CPU only.
- `nsys_kernel_diff.py CTL.sqlite[,...] EXP.sqlite[,...] [N]`: per-kernel GPU time of two builds from nsys SQLite exports (`--export=sqlite`, both builds profiled through the GPU queue with the same command, interleaved), the N biggest wins and losses.

- No `call` in the PTX (libdevice is inlined, so any call is a body that wasn't); fail the build on it.
- ptxas `-v`: no unexpected spills or stack frames. A 32-byte stack in a kernel calling `sinf`/`cosf` is libdevice's slow path, the same as nvcc's.
- A refactor with byte-identical PTX is safe; diff it.
- When porting or changing codegen, compare `cuobjdump -sass` opcode counts per kernel with the reference build, and run an nsys per-kernel diff plus an A/B in every precision mode: parity and sanitizers miss speed regressions.
