# cuda-oxide watch-outs

Traps that cost correctness or speed in cuda-oxide kernels, and the checks that catch them. Add to it after each run: one line per trap with the fix, plus the commit it was seen at, since some are bugs that later commits may fix. Recheck old entries when you move to a newer cuda-oxide. Evidence for each entry is in the run logs under `~/code/research/gpu-kernels/cuda-kernels/`.

## Setup

- Use a project-owned `CARGO_HOME` for cargo-oxide: the backend cache in `$CARGO_HOME/cuda-oxide/` is shared, and installing another commit rebuilds it under other projects.
- Pin the commit in the kernel crate's `Cargo.lock` and install cargo-oxide at that commit, so the device library and backend match. The toolchain is whatever that commit's `rust-toolchain.toml` pins.
- PTX is only written when the crate itself recompiles: `cargo clean -p <crate>` first.
- The PTX ISA `.version` sets the minimum driver; check it in the build script (llc-21: 7.0 for sm_80, 8.7 for sm_120).

## Speed traps

- **Generic `#[device]` fns are never inlined** (`2d47e43`): they stay a `.func` and their pointers arrive generic (generic `LD` instead of `LDS`/`LDG`, flash attention 25-72% slower). Write generic bodies as plain `#[inline(always)]` fns, and unroll their loops with `cuda_device::thread::__unroll_config::<0>()` as the loop body's first statement (0 = fully, N = by N). That marker is the undocumented call `#[unroll]` expands to, so recheck it after every cuda-oxide upgrade (the build's no-`call` check and a SASS look catch a break).
- **`f32::max`/`min` become compare + select** (`2d47e43`), not one `max.f32`: use libdevice `__nv_fmaxf`/`__nv_fminf`.
- **No automatic partial unrolling of runtime-bound loops**, which nvcc does: ask for it where loads should overlap.
- **Iterating a local array with `for x in arr`** puts it on the stack: write the operations out.

## Correctness traps

- `convert::f32_pair_to_packed_bf16` **truncates**; `cvt_bf16x2_f32` rounds to nearest even.
- A closure capturing shared-memory pointers fails MIR verification (`2d47e43`): use a `macro_rules!`.

## API gaps and limits

- `#[unroll]` works only directly in `#[kernel]`/`#[device]` bodies, not inside `macro_rules!` bodies, and not on loops with a non-constant step (warns, stays rolled).
- No `static mut SharedArray` in generic fns: declare tiles per kernel and pass pointers in.
- No `[T; D / 16]` in generics without `generic_const_exprs`: pass the derived length as another const param.
- No `f32::mul_add` in `no_std` (use `float::fma_rn_f32`); libdevice math through `unsafe extern "C" { fn __nv_erff(x: f32) -> f32; }`, which matches nvcc's results.
- Wide loads and stores (`float4`, `st.shared.v4`) need `ptx_asm!`.
- Not every tensor-core instruction is wrapped: the FP16-accumulate `mma.sync ...f16.f16.f16.f16` isn't (`2d47e43`), so it is `ptx_asm!`. Check `cuda_device::wmma` before writing one.
- `#[kernel] pub unsafe fn` with raw pointers and plain scalars exports a C-like entry, so hosts written for CUDA C++ kernels need no launch changes.

## Checks on every build

Scripts in this skill's `scripts/` folder:

- `sass_mix.sh REF.ptx NEW.ptx [ARCH] [KERNEL...]`: calls left in each PTX, then per kernel the registers, stack, spills, SASS instruction count and trap opcodes (generic `LD`, `FSETP`+`FSEL`, `LDL`/`STL`). With no kernels, every kernel whose registers or spills changed. CPU only.
- `nsys_kernel_diff.py CTL.sqlite[,...] EXP.sqlite[,...] [N]`: per-kernel GPU time of two builds from nsys SQLite exports (`--export=sqlite`, both builds profiled through the GPU queue with the same command, interleaved), the N biggest wins and losses.

- No `call` in the PTX (libdevice is inlined, so any call is a body that wasn't); fail the build on it.
- ptxas `-v`: no unexpected spills or stack frames.
- A refactor with byte-identical PTX is safe; diff it.
- When porting or changing codegen, compare `cuobjdump -sass` opcode counts per kernel with the reference build, and run an nsys per-kernel diff plus an A/B in every precision mode: parity and sanitizers miss speed regressions.
