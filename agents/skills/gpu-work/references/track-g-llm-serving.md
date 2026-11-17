# Track G: LLM serving tuning

Making a model server faster on one machine: vLLM, SGLang, llama.cpp, ExLlama or TabbyAPI, MLX, TensorRT-LLM. Config levers come first because the engine already exists; once they plateau, a kernel far from its floor moves to tracks A and B. This track stands alone: it needs only SKILL.md's queue rule and [gpu-ab](https://github.com/praveenperera/gpu-ab).

## Day 0

- **Write down the targets and limits.** Targets: c1 decode, cN decode, prefill at several lengths, TTFT, startup. Limits: the memory cap, the context that must stay stable, and which lossy trades are allowed (int8 or FP8 prefill GEMMs, FP8 KV). Label every lossy result as lossy.
- **Inventory the machine and server.** Record the accelerator, its memory and whether that memory is unified, the driver, the engine version or commit, the model and its revision, the quantization, and the current launch command.
- **Measure, don't trust spec sheets.** Measure device-copy bandwidth, large-GEMM throughput, and clocks under load.
- **Compute the ceilings.** They are this track's gap-and-floor table:
  - **Decode** is bandwidth bound. Bytes per token are the dense weights, plus active experts (top-k ÷ experts × expert bytes), plus the LM head, plus the KV read at the working context. Divide by the measured bandwidth. Speculative decoding divides the target's bytes by the accepted length and adds the draft's bytes; the draft's LM head is often the biggest part.
  - **Prefill** is compute bound. FLOPs per token ≈ 2 × active params, plus attention at about 4 × layers × hidden × position. Divide by about 50% of the measured peak.
  - **TTFT** floor: one forward pass plus tokenizing and scheduling. **Startup** floor: reading the weights plus allocation, with JIT caches warm.
  - **KV**: bytes per token per layer × the layers that have KV (hybrid models have few). This gives how many full-length sequences fit under the cap.
  - If a target is above its ceiling, say so early and show the arithmetic.
- **Dense decode is often already at the roof.** On Apple silicon, llama.cpp, MLX and vLLM-Metal ran dense models at 90-101% of measured bandwidth, so kernels had nothing to gain. MoE decode ran at 71-84% because of a near-constant 3.6-3.8 ms per token (many small kernels, a serial routing chain, less-tuned head sizes); custom Metal kernels recovered about half of that.

## Timing with gpu-ab

- **An arm is a launcher script, frozen as `{bin}`.** It starts the server with the arm's config and waits for health. Then it runs the fixed client panel and writes metrics and outputs to `{out}`. Finally it aborts in-flight requests, waits for the drain, and stops the server with a timeout. Never force-kill mid-batch: a forced stop mid-prefill has caused GPU page faults. Start the server in its own process group, kill the group at the end, and fail the arm if any of its processes remain: an idle leftover server can slip under gpu-ab's contention checks and skew later arms. gpu-ab hashes only the script, so point it at an engine, weights and configs that stay unchanged for the whole run (read-only or content-addressed copies), and have it write their hashes or image digests into `{out}`; a recorded version alone doesn't stop a file from changing between arms. Start from the [adapter template](https://github.com/praveenperera/gpu-ab/blob/master/docs/adapters/example.toml), which already has a higher-is-better tok/s metric, and read [the verdict rule](https://github.com/praveenperera/gpu-ab#the-verdict) before trusting a result.
- **One change per experiment, derived from the current best.** Put the difference in the script or in `--ctl-env`/`--exp-env`. Give each metric its own bench (c1 decode, c4 decode, 32K cold prefill, time to healthy), since an adapter reads one number per arm.
- **Guard the other targets before keeping a win.** Run the remaining target metrics as guard benches on the candidate, and keep it only if none regresses by more than ~3% (or the limit the user set): a c1 decode win can cost c4 throughput or TTFT.
- **Fingerprint every arm.** Hash greedy outputs for changes that claim to be lossless. Use a gate script for lossy trades; it should also reject looping output and failed requests.
- **Keep JIT, compile and autotune caches outside the arm** and persist them. gpu-ab's warmup arms absorb the first boot after a cache miss (one vLLM boot ran 36 tok/s against 90 warm).
- **gpu-ab supports NVIDIA and Apple GPUs.** On AMD or Intel hosts, follow the same protocol by hand: paired ABBA arms in one queued run, A/A noise first, and every arm's output checked.
- **Run the server through the GPU queue** like any other GPU work. The queue cancels a job whose processes hold no GPU context for 5 minutes, so warm the page cache, or check that the engine opens its context before a long weight load. (This hasn't been tried with a server yet.)

## Counting

- **Count tokens with server usage stats or the tokenizer, never streamed chunks.** With speculative decoding, one SSE chunk carries several tokens. Use `stream_options: {include_usage: true}` or re-tokenize the output.
- **Size prompts with the real tokenizer.** "Random word" documents ran about 3.5 tokens per word, so nominal 32K and 128K cells were really 58K and 232K.
- **Build prompts before the clock starts, with a distinct deterministic prefix per request**, so prefix caching can't hit within a run, and send the same panel to every arm. Where the server reports cached tokens, check they are zero on cold measurements.
- **Definitions.** Decode: completion tokens after the first ÷ time since the first token. Prefill: prompt tokens ÷ TTFT on a cold, unique prompt; report warm (cache-hit) prefill separately and never as prefill. cN: total completion tokens ÷ wall time from first start to last finish. Confirm the server actually ran N requests at once.
- **Use a fixed, seeded prompt panel.** Random prompts moved acceptance from 2.4 to 3.7 and turned c1 comparisons into noise. Report the panel, the temperature and the thinking mode with every number.
- **Laptops and unified-memory boxes:** use AC power, a fixed performance profile and an idle host CPU. Light CPU load took one laptop RTX 4080 from 107 to 75 tok/s. Restore any profile you changed, and tell the user.

## Speculative decoding

- **Judge per-step time and accepted length separately:** tok/s = accepted length ÷ step time. Higher acceptance can still run slower; one 7-token draft accepted 3.26 against MTP's 2.85 and ran 11-14% slower.
- **Sweep the draft length per concurrency and context.** One card's best was k=4 at c1, k=3 at c4 and k=2 at c16. At 128K, 1 draft beat 3. Measure acceptance per depth, because some MTP heads are weak past depth 1.
- **Prune the draft LM head's vocabulary** to its most-used blocks. The target still verifies against the full vocabulary, so output doesn't change (+14% and +27% on two cards).
- **Keep sampling on the GPU, and send verify through decode-style attention.** Routing k-query verifies to the prefill kernel added 3.5 ms per 1K tokens of context.
- **Acceptance counters may update only when a request finishes**, so probe whole requests.
- **Speculation can lose** on bandwidth-poor chips (Apple M4: 6.35 → 3.25 tok/s with 3 MTP drafts) and on hard prompts.
- **On a sparse MoE, verify reads the union of the drafted tokens' experts.** At 8K, verifying 4 and 8 tokens cost 2.7x and 4.4x one token, and a matching draft model accepting 3.3-3.5 tokens was still 45% slower. Prompt lookup won only on copy-heavy edits.
- **Gate non-bit-exact kernels per precision, against the engine's own noise.** A decode attention kernel more accurate against f64 than the incumbent still moved near-tie tokens: it passed at 8-bit and failed at 4-bit. Hold top-1 agreement, mean KL and perplexity to the spread of the engine's own alternative paths, or task quality to at most 1 point worse at 95% confidence on paired benchmarks.
- **A greedy spec-on/spec-off mismatch can be numerics**, because M=1 and M=k rows run different kernels. Compare logits at the divergence point before calling it a bug.

## Stability gates

A config is "best" only after it passes these on the candidate itself:

- **Long-context retrieval:** 3 random codes at different depths of a ~50% document and then a ~100% one, all answered exactly. Speed alone doesn't prove the context works. Test the engine directly, because gateways often cut requests at 600 s.
- **A concurrent full-length soak**, when the KV pool is sized for N users.
- **A memory-cap watchdog that kills the run at the cap.** On unified memory, GPU allocations bypass cgroups and MemAvailable; one MemAvailable delta showed 14 GiB of a real 29.4 GiB. Sum driver residency (Linux DRM: `drm-resident-*` in `/proc/<pid>/fdinfo`, deduplicated by `drm-client-id`) plus cgroup `anon`. On Apple, use `ioreg`/`vm_stat`, and remember that `iogpu.wired_limit_mb` caps usable memory.
- **Output sanity:** flag looping output, failed requests, a GPU under ~60% busy in the window, prefill inside the decode window, and KV-bound admission. One "record" of 117 tok/s was a model looping through a numbered list.

## Levers, cheapest first

1. **Persist JIT, compile and graph caches** (Triton, Inductor, DeepGEMM, SYCL, `~/.cache`). This cut startup from 113 s to 43 s and from 22 to 7 minutes. Where mmap loading is slow, load safetensors eagerly (`--safetensors-load-strategy eager`). Use one compile worker on memory-tight boxes.
2. **Decode graphs** (CUDA or XPU): often 2× c1 on their own.
3. **Speculative decoding**, as above.
4. **Concurrency and KV sizing.** Set max running requests and the KV pool to N × context.
   - On hybrid SSM/GDN models, state slots cap concurrency, not KV tokens. SGLang silently stays at 1 request until `--max-mamba-cache-size` is raised.
   - Enable prefix caching explicitly and confirm `cached_tokens` > 0. Some engines turn it off silently for hybrid models.
5. **Prefill.** Tune the chunk size (throughput against decode stalls) and try a faster attention library. Int8/FP8 GEMMs are a labelled trade. Avoid padding cliffs in grouped MoE GEMMs. Capture prefill graphs at 256-512 tokens at most; a 2,048-token capture used 6.3 GB and crashed.
6. **Kernel thread and tile sizes.** Defaults are often tuned for a bigger card: moving 4096 threads to 1024 took one card's c1 from 48.5 to 56.9.
7. **Kernel work:** nsys, ncu and the ISA, under tracks A and B. Under graphs, fusing small kernels rarely helps; three correct fusions gave 0%, because the time is inside the big kernels.

Other trades:

- Use fewer bytes per weight in a format the hardware decodes natively. Ampere has no FP8 units, so AWQ-INT4 beat FP8 on a 3090.
- FP8 KV halves memory but halved one flash-attention kernel's throughput.
- q8_0 KV in llama.cpp lost 40% on a model with KV in only 16 of 64 layers.

When a result looks odd, suspect the measurement first: chunk counting, prompt length, cache hits, thermals, power. When higher acceptance doesn't raise tok/s, the draft costs too much; measure per-step time.

## Engine traps

- **vLLM**
  - On unified memory, `gpu_memory_utilization` is compared against free host RAM, and values near 0.985 are refused.
  - On Intel XPU, only `FULL_DECODE_ONLY` graphs worked.
- **SGLang**
  - `--num-continuous-decode-steps` does nothing while speculative decoding is on.
  - Shared-expert fusion is enabled only for some backends.
  - XPU 0.5.x verified greedily at any temperature, among other bugs.
- **llama.cpp:** MTP and draft models can fall below the no-draft baseline. Check backend work-group sizes; a work-group of 1 in one SYCL copy kernel cost 7.8×.
- **ExLlama on ROCm:** HIP graphs leaked until the process crashed. It ran stable with them off.
- **Hardware:** correlate "device lost" crashes with the kernel log before blaming code; one set turned out to be PCIe AER link drops. If an fp16 GDN/SSM state outputs `!!!`, the state dtype is wrong.

## Plateau, qualify, publish

- **Plateau:** declare one when three levers in a row land inside the noise. Write a STATUS entry in the decision log with:
  - the best numbers against the baseline and the ceiling;
  - what limits further gains, with evidence;
  - the remaining levers, ranked, with their costs.
- **Pin everything:** the base image by digest, the engine and kernels by commit, Python packages by version (fail on a `pip freeze` diff), and the weights by revision. Build from a clean checkout of the recipe commit, never a dirty tree or a local patch. Diff the launch against the benchmarked config mechanically.
- **Acceptance gates on the clean build:**
  - the server loads and `/v1/models` lists the model;
  - chat ends with a stop finish;
  - a needle at ~85% of the window is found exactly;
  - single-user decode clears a floor;
  - for each feature the model or recipe advertises: reasoning comes back separated from the content, a tool call completes the round trip, vision works, and the soak passes for N users at full context. Record the features it doesn't offer.
  Gate more than once when acceptance ramps up during warmup.
- **A/B the clean build against your best image with gpu-ab.** They must match within noise.
- **Before publishing:**
  - keep any branch that holds a pinned build commit;
  - grep the bundle for secrets, credential paths, emails and home-directory usernames;
  - say which numbers come from gates (e.g. thinking on, first 30 s) and which from the climb (medians, thinking off). The two aren't comparable.
- **Report:** a table of metric, baseline, now, ceiling and how it was measured. Then:
  - what produced the gains, and why the climb stops here;
  - the system state you changed: power profiles, running servers, uncommitted code, branches to keep;
  - what was not verified.
  To free the GPU: drain, stop, and confirm that no processes remain and the clock is idle.

Adapted from [sybil-solutions/inference-optimisation-skill](https://github.com/sybil-solutions/inference-optimisation-skill), which also has an engine-agnostic bench, a ceiling calculator and worked examples. Its license:

> MIT License
>
> Copyright (c) 2026 Sybil Solutions contributors
>
> Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
