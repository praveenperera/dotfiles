# cuda-oxide watch-outs

Traps that cost correctness or speed in cuda-oxide kernels, with their fixes and the checks that catch them. Recheck entries when the pin moves.

**Coverage:** elementwise, norm, 2D and 3D conv, flash-attention, small-M `mma`, DTW, relaxed block and device `u32` atomics (histograms) and side kernels, written on an RTX 5090 (sm_120). The sm_80 tier has run on A100, A10, L4 and 4090, and an sm_75 tier on a T4 (inkrs). Not yet exercised, so expect unlisted traps: grid-wide reductions, cooperative launches, TMA, `wgmma`, FP8, clusters.

## Setup

The pin and crate layout are in SKILL.md "Kernels".

- **Moving the pin is a codegen change, even with no source change.** Rebuild at both pins, run `sass_mix.sh` over every kernel, and time any kernel whose registers moved. Explain any speed move over a few percent before accepting it, and diff outputs per layer when they drift: the fork-to-upstream move changed inkrs maps by up to 1/255 with every gate passing, and moved tla kernels by −41% to +35%, all unexplained. Tag every results file with the pin it ran at, and keep the old pin reachable (a tag) until every consumer has moved. Budget about half a day per project plus queue time.
- The backend cache in `$CARGO_HOME/cuda-oxide/` has one slot, rebuilt whenever a project's `cuda-device` resolves to another commit, hence the project-owned `CARGO_HOME`.
- `cargo oxide build` rebuilds a missing or stale backend, so build scripts needn't call `setup`. `setup` always rebuilds, and refuses when `CUDA_OXIDE_BACKEND` or `.cargo/cuda-oxide.toml` pins the backend. `build` rewrites the PTX every run only when the crate root is `src/lib.rs` or `src/main.rs`; otherwise check the PTX timestamp.
- **Use the toolchain's own llc, and fail when it's missing.** Set `CUDA_OXIDE_LLC=$(rustc +T --print sysroot)/lib/rustlib/<host>/bin/llc`; otherwise cuda-oxide silently falls back to the sysroot llc, then `llc-23`…`llc` on PATH, and another llc changes codegen and speed. Don't inherit an env file's `llc-21`. Keep the pinned toolchain out of rustup cleanup; reinstall it with `rustup component add rust-src rustc-dev llvm-tools --toolchain T`.
- Host `cargo check` of the kernel crate needs `RUSTC_BOOTSTRAP=1`, the project's `CARGO_HOME` and `+<toolchain>`.
- The PTX ISA `.version` sets the minimum driver; check it in the build script (7.0 for sm_80, 8.7 for sm_120).

## Speed traps

- **A generic `#[device]` fn ignores its own `#[inline(always)]` and `#[inline(never)]`**: LLVM decides by size, so a large body stays a `.func` and its pointers arrive generic (generic `LD` instead of `LDS`/`LDG`). Write hot generic helpers as plain `#[inline(always)]` fns, not `#[device]`, and unroll their loops with `__unroll_config::<N>()` (see "API gaps"). Non-generic `#[device]` fns are unaffected.
- **A borrow (`mir.ref`) of a local inside a loop allocas at the use site**, which LLVM treats as dynamic: no inlining, and SROA leaves it on the stack. An unexplained `.func` or frame: look for an `alloca` outside `entry:`.
- **Use a counted loop over views, not `Zip` over a slice and a view iterator**, which has two exits and stays rolled. `zip_exact` is fine.
- **Don't return `Option<(struct, views...)>` from an inline helper**; the payload goes to the stack. Return a plain tuple and `unreachable_unchecked()` past one up-front check.
- **Register arrays indexed by a rolled loop's counter go to `.local`.** Indices must be constants, so the loop must fully unroll; a body calling `expf` can stay rolled, and filling through `iter_mut().enumerate()` also lands on the stack. Put `#[unroll]` on every loop that indexes a register array, or build vectors with array constructors. `opt` runs `default<O2>`, whose full-unroll threshold is half of clang `-O3`'s, so loops nvcc unrolls can stay rolled. Only `cargo oxide build --verbose` warns ("could not be promoted to registers"), so check `ptxas -v` for a frame.
- **LLVM can hoist loop-invariant loads, or merge a later pass's reloads into an earlier pass, until a kernel hits 255 registers and spills** (inkrs `stem1_stats`: 1088 B of spill; `depth_pool`: 440 B frame). Load with `read_volatile` where the value is cheap to reload, or hold it packed (fp16 pairs); in ptxas `-v`, 255 registers is the tell. Prefetching `mma` fragments can do the same: it roughly doubled registers in inkrs's convs and halved occupancy.
- **Avoid 64-bit `/` and `%`** (`i64` and every runtime `usize`); they compile to a SASS `CALL`. Use 32-bit coordinates (`thread::index_1d_u32`, `element_thread32`, `#[launch_contract(coordinates = u32)]`), shifts or constant divisors. In block-per-row kernels pass the row length as a param, not `x.len() / gridDim.x`.
- **Panic checks compile to traps.** `clamp` asserts min ≤ max and `/` checks for zero, even with bounds known safe. Use `max` then `min`, and divide by `x.max(1)`. A thread-dependent trap marks the warp divergent, so ptxas wraps every later shuffle and `mma` in WARPSYNC; in warp-cooperative kernels check once with a block-uniform condition and index unchecked after.
- **Bounds-checked `slice[i]` in a loop can stop unrolling** and costs registers. Read through `MatrixView32`/`RowView32`, whose `get(i)` checks once.
- **Fold block-uniform checks before the first load into one non-short-circuit condition** (`a & b & c`, one `return`) and read through a row pointer it covers. `ld.global.nc` and `get(c).unwrap_or(0.0)` cost nothing.
- **`f64::max`/`min`, `libm::fmax`/`fmin`, and `f32` on the libNVVM route compile to compare + select.** On the default route `f32::max`/`min` and `warp::reduce_max_f32` are one instruction.
- **`#[unroll(N)]` partially unrolls block-stride loops**: a counted-up `<`/`<=` loop with a runtime start, a positive constant or loop-invariant step (`i += blockDim_x()`) and a bound recomputed from fixed values. A `match` in the body is fine; a `break` isn't. Full `#[unroll]` needs a constant start and step.
- **`DisjointSlice` turns a scatter into a gather that can read strided.** For a transpose, keep the scatter through `as_mut_ptr()` with a SAFETY comment, or use a shared-memory tiled transpose.
- **With llc-21 only:** a pointer merged with null through `if`/`select` turns loads generic (keep optional inputs as empty slices and test `is_empty()`), and `for x in arr` by value keeps the array on the stack.
- **PTX can change without a code change.** Moving code into shared helpers can shift SASS. Shared statics and device globals are named from a hash of the symbol, and `const` globals from their contents, but labels, `// @<name>` comments and keyless `__shared_mem_<n>` still move: normalize them (`PW/ops/sam3rs/hill-climb/ptx_same.py`) before calling a kernel changed. When a kernel must stay unchanged, give the new kernel its own copies and diff the old kernel's PTX to byte-identical. Only a static whose top-level type is `SharedArray` or `Barrier` goes to shared memory; one that nests them in a tuple, struct or array is refused, so give each its own static.
- **`cvt.rna.tf32.f32` costs 3 instructions per value on sm_120**, the same as nvcc.
- **For `ldmatrix`, use `ldmatrix_*_shared_u32` with a per-lane u32 address computed once**; the runtime-stride usize form costs extra setup instructions and registers.
- **Special-register reads must stay `convergent`.** Without it LLVM merges `tid.x` reads and hoists dependent addresses, costing registers and speed in flash kernels. If a pin drops blanket `convergent`, time the flash kernels against the previous build.
- **Under one wave, ptxas can save registers for occupancy the grid never uses** and serialize loads to fit. When a latency-bound grid is under a wave at every deployed shape, try `#[launch_bounds(128, 1)]` (`.minnctapersm 1`) or `.maxnreg`; above a wave, time it. Diagnose from SASS scoreboard waits, not load counts.

## Correctness traps

- `convert::f32_pair_to_packed_bf16` **truncates**; `cvt_bf16x2_f32` rounds to nearest even.
- **Keep each unchecked read inside the branch that proves it.** `x.unwrap_or(unsafe { *w.get_unchecked(c) })`, or a `let` above `if w.is_empty()`, is UB on a short slice.
- **Take the index before the output witness, and the read after it**: `let i = t.get() as usize`, then the witness, then the read. Reading first runs on tail threads and traps.
- **Only cuda-oxide's launcher checks the launch contract.** Another host (cudarc, the driver API) must keep `grid x block <= 2^32` for 32-bit coordinates and pass a length after every slice pointer.
- A `const` reference or promoted borrow is a read-only global: a device fn may return it, but only read through a `*mut` cast from it.
- A const, table or static holding a pointer to shared memory (`SharedArray`, `Barrier`) is refused; pass shared pointers as arguments or build the value at run time. A shared pointer stored through a generic place is widened to generic.
- **A kernel that appends to a cache it reads in the same launch** must read the new keys from the source rows, never from the cache.

## API gaps and limits

- **At `8d8254869` every `scalar_arithmetic` intrinsic is refused below sm_80** (`add_rn_f32`, `mul_rn_f32`, `fma_rn_f32`, `div_*` with explicit rounding), though PTX has had them since sm_20: a catalog mis-tag ([#1462](https://github.com/NVIDIA/cuda-rust/issues/1462), fixed by #1725, open as of 2026-11-13). In the sm_75 tier write them with `ptx_asm!(... options(register_only))` behind a tier feature in one shim module, and drop the shim once the pin includes #1725. `scalar_math` (`sqrt_rn`, `ex2_approx`, ...) was fixed in #1670 and works on sm_75 at this pin. `cvt_f16x2_f32` and `mma_m16n8k16_f32_f16` really are sm_80+: use two `cvt.rn.f16.f32` and two `mma_m16n8k8_f32_f16`.
- `#[unroll]` only works inside `#[kernel]`/`#[device]` bodies; in a `macro_rules!` body or plain helper, start the loop body with `__unroll_config::<N>()` (0 = fully). A helper shared with a host crate that lacks `cuda_device` can't call it; use a per-crate `unroll_full!()` macro that expands to nothing on the host.
- `cp_async_wait_group` needs a literal count; `match` over the const param.
- Generic kernels export `_TID_` hash names; wrap const-generic kernels in macro-generated non-generic `#[kernel]`s so hosts can find them.
- A `&mut dyn FnMut` helper compiles to an indirect call; use a macro. An inner `__unroll_config::<0>()` loop starting at an outer loop's variable stays rolled: loop `i` over `0..N` and use `kk = c0 + i`.
- An unroll clones at most 65,536 operations, counted after inner loops expand; an outer loop around a few fully unrolled inner loops goes over, stays rolled with a warning, and moves any local array it indexes to `.local`. There is no per-loop override: split the loop or unroll less.
- `ThreadIndex32` is not `Copy`: each output `DisjointSlice` needs its own witness from `thread::index_1d_u32(lc)`.
- No block-indexed output space: a block that owns a row writes through `DisjointSlice::get_unchecked_mut` with a SAFETY comment. Taking one buffer as both `&[T]` and `DisjointSlice<T>` is UB, so an in-place kernel takes only the `DisjointSlice` and reads through its raw pointer.
- A `DisjointSlice` is written only at the thread's own index; a thread with two outputs takes a second witness or writes one combined buffer.
- Wide loads and stores come from alignment, not asm: `vector::F32x4` (or a `repr(C, align(16))` type) in a slice, `DisjointSlice` or `SharedArray`, or `vector::as_vectors`. Reading only some lanes narrows the load.
- No `f16` or `bf16` value type on stable: the `f16`, `f16x2`, `bf16` and `bf16x2` intrinsics carry `u16`/`u32` bits. Wrap them in a newtype using `convert` (`cvt_f32_f16x2_lo`, `cvt_f16x2_f32`).
- No `static mut SharedArray` in generic fns (E0401): declare tiles per kernel and pass pointers, or use `DynamicSharedArray`. No `[T; D / 16]` in generics: pass the derived length as another const param.
- Math: use `float::fma_rn_f32` or `a * b + c` (contracts to `fma.rn`), not `f32::mul_add`. `libm::erff`, `sqrtf` and 120 other libm exports lower to libdevice. libm has no rsqrt: use `float::rsqrt_approx_f32`, or declare `extern "C" fn __nv_rsqrtf` to match nvcc's `rsqrtf`. `__CUDA_FTZ` reflects to non-FTZ, nvcc's default.
- Optional output: a `#[device]` helper takes `Option<DisjointSlice<T>>` (or `Option<&mut DisjointSlice<T>>`) and gets `None`. There is no safe empty `DisjointSlice`; `unsafe { DisjointSlice::from_mut_slice(&mut []) }` with a SAFETY note is sound. An associated `const` struct holding `&[T]` fails codegen; build it in an `#[inline(always)] fn` with inline `&[]`.
- `launch_contract(block = ...)` emits `.reqntid` only, since ptxas forbids it with `.maxntid`.
- Tensor-core wrappers: `cuda_device::wmma` has `mma_m16n8k16_f32_bf16`, `_f32_f16`, `_f16_f16`, `mma_m16n8k8_f32_tf32`, `mma_m16n8k4_f32_tf32`, `ldmatrix_*` and the `mma_frag` layout helpers. Search the crate, including `generated/`, before writing `ptx_asm!`. `ldmatrix_x4_shared_u32` over FP32 rows (stride D + 4 words) gives exactly the m16n8k8 TF32 A/B fragment, and the S accumulator serves as the P @ V A fragment as `{s0, s2, s1, s3}`.
- A host launching slice-ABI kernels should check its arguments against each `.entry` param list parsed from the loaded PTX, once per kernel. Don't probe `cuFuncGetParamInfo` past the last param (compute-sanitizer flags it); `cuFuncGetParamCount` needs CUDA 13.2.
- `#[kernel] pub unsafe fn` with raw pointers and scalars exports a C-like entry, so hosts written for CUDA C++ kernels need no launch changes.

## Checks on every build

- `~/.claude/skills/gpu-work/scripts/sass_mix.sh REF.ptx NEW.ptx [ARCH] [KERNEL...]` (CPU only): calls left in each PTX, then per kernel the registers, stack, spills, SASS instruction count and trap opcodes (generic `LD`, `FSETP`+`FSEL`, `LDL`/`STL`). With no kernels, every kernel whose registers or spills changed.
- `~/.claude/skills/gpu-work/scripts/nsys_kernel_diff.py CTL.sqlite[,...] EXP.sqlite[,...] [N]`: per-kernel GPU time of two builds from nsys SQLite exports, both profiled through the queue with the same command, interleaved.
- No `call` in the PTX; fail the build on one. Check PTX, not SASS: ptxas adds SASS `CALL`s for the IEEE f32 division slow path (`1/x`, `x/n`) even when everything is inlined. Other SASS `CALL`s are real: integer divides or divergent-shuffle fallbacks.
- ptxas `-v`: fail the build on any frame or spill not on a committed allowlist that gives its reason (a 32-byte frame with `sinf`/`cosf` is libdevice's slow path, as with nvcc). Reading the log isn't enough: inkrs shipped a 192 B frame and an 8 B spill nobody explained.
- Build every tier on every branch, not only after merge, and fail on tier-gated intrinsics (above) used outside the tier shim; inkrs's sm_75 tier broke three times this way.
- Generate the compute-sanitizer coverage list from each tier's `.entry` list, so a new kernel can't skip it.
- A refactor with byte-identical PTX is safe; diff it. When porting or changing codegen, compare SASS opcode counts per kernel, and run an nsys per-kernel diff plus an A/B in every precision mode: parity and sanitizers miss speed regressions.
