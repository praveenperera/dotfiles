# Track C: native model runtime

Porting a whole model off PyTorch or ONNX Runtime (speakrs, scriptrs, sam3rs). `R` = `~/code/research/gpu-kernels/`, `PW` = `~/code/private-workbench/`.

- **Know which kind of model it is.** Step loops, many small ops and multi-stage pipelines gain 20-30x over a generic runtime (speakrs). A GEMM-bound encoder gains about 1.05-1.1x on kernels, and the win is in the glue: SAM 3 prompts gained 1.3-15x. Do the floor arithmetic first. (`~/code/research/inference/specialized-model-runtimes/generic-vs-specialized-kernels.md`)
- **Where the time usually is:** host round trips and re-uploads, CPU fallback ops, one launch per op, host-driven decode loops, glue on the CPU, and stages run one after another.
- **What paid across the ports** (not ranked): keeping tensors on the GPU, a CUDA graph per shape where the GPU idles (check GPU-busy first), the decode loop as a graph of small fused kernels, host prep overlapped with the GPU, batching, and fusing glue into neighboring kernels.
- **Choose precision per layer and test each contract.** BF16 tensor cores need BF16 storage, format conversions are a hidden cost, and a global math mode leaked into layers promised FP32. (Lessons §7)
- **Check parity per layer, not just at the output.** Deep networks amplify rounding (about 3x per layer in the last four Parakeet layers). Compare each kernel with f64 on its exact rounded inputs, and check that the f64 reference matches the framework. (Lessons §8)
- **Fail closed on inputs:** check weights against a committed schema, have the loader reject config options the engine doesn't implement, golden-test the tokenizer against upstream including special tokens, and feed the reference's decoded image or audio to parity tests. (Lessons §7, §10)
- **Bound shape caches by bytes** and share scratch across shapes, then count cache misses before blaming kernels. Serialize model load, capture and teardown with one process-wide lock, and run concurrency tests under full CPU load. (Lessons §8)
- **Build the safe Rust layer early:** `Pod`-only transfers, host buffers kept alive through async copies, buffers that keep their stream alive, loader errors instead of panics, and geometry validated at the API boundary. (Lessons §7, §9)
- **Tiny recurrences may belong on the CPU.** A Silero VAD step was faster on the CPU than as a graph launch.

Read: lessons §7 (SAM 3), §8 (scriptrs) and §10 (patterns across ports), `PW/playbooks/gpu-hill-climb.md` once the port works, `~/code/research/inference/specialized-model-runtimes/README.md`, `R/scriptrs-cuda/README.md`, `~/code/research/inference/specialized-model-runtimes/sam3-native-2026-10-30.md`, `~/code/research/speakrs/README.md`.
