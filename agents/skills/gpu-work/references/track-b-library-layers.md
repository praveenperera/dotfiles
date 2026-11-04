# Track B: replace library layers

Writing custom kernels for layers cuDNN or cuBLAS already runs. Shorthands as in SKILL.md.

- Reverse-engineer the library first: its algorithm query, kernel names and bitwise forensics on small cases. Match or beat that structure; tuning a different structure plateaus.
- Peak fraction isn't the target: near-peak FP32 convs still lose to Winograd. Beating a library on datacenter parts needs algorithmic changes (Winograd, split precision, INT8 tensor cores).
- Keep the harness rough: f64 or CPU reference, timed A/B on representative shapes, end-to-end check on a fixed file set. Add proof machinery only for risks that check can't see, time-boxed, with one external audit at a freeze point.
- Declare coverage per (layer, batch class, math mode, device), and leave undeclared cases on the library.
- Scope speed claims to measured hardware. Never promise "every GPU".
- Call FP32-accurate schemes such as Ozaki by their real contract, not "exact FP32".

Read: `R/cuda-kernels/speakrs-no-cudnn-lessons-2026-10-07.md` (first), lessons §2, §3 and §6. `R/cuda-kernels/cuda-qualification-harness-2026-10-02.md` is for shipping code only.
