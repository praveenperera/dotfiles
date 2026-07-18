# Workflow

- Ship production-quality changes. Model the domain first, make impossible states impossible with typed domain models, and prefer the proper owner or abstraction over caller-specific conditionals. Repeated fixes in one area signal that the model may be wrong; revisit it and remove shortcuts or resulting tech debt before finishing.

# General

- Only add important comments that explain why. Start inline comments lowercase and start higher-level doc comments with a capital letter; do not end comments with periods or make them depend on conversation context. Document every public API in libraries.
- For commits, follow `$HOME/.agents/commit-message-guide.md`; use Praveen Perera when an author is needed, and never add Claude/Codex/AI co-authors or generated-by notes.
- Minimize nesting in functions.
- Do not leave deprecated code in place by default. Remove it, or ask whether the change must preserve the old path.
- Put ad hoc files the user may want to inspect, such as Markdown, HTML, screenshots, and image-generation outputs, in a repo-root `_scratch/` directory and create it if needed.
- In public-facing copy, include only reader-visible content. Omit implementation notes, source labels, workflow state, reasoning, conversation context, and edit instructions.
- Preserve unrelated user or agent changes. Use hunk staging for commits and never undo unrelated edits.

# Rust Project Specific

- `info` and `error` logs may start with uppercase letters.
- In log and `println!` macros, prefer inline variable capture such as `warn!("person id={id} ...")` over positional placeholders.
- For unfamiliar crates or external libraries, inspect documentation or source instead of guessing. Check `target/doc/`, run `cargo doc -p <crate-name>`, inspect `~/.cargo/registry/src`, or use `btx` to look at the code directly.
- When clippy reports autofixable issues, run `cargo fix --allow-dirty` only when the working tree and command scope make it safe from unrelated changes; otherwise apply the fixes manually. Fix remaining lints directly instead of silencing them with `allow` or `warn` unless there is a specific reason.
- Prefer `eyre`, or `color-eyre` for CLIs, over `anyhow`.
- Use the Rust 2018+ module layout instead of `mod.rs` for regular modules.
- Use if-let chains with `&&` when they preserve semantics and reduce nesting.
- Avoid redundant closures; use `.map(func)` instead of `.map(|value| func(value))`.
- Prefer tuple structs over named-field structs for simple wrappers, such as `struct Foo(Arc<Inner>)`.
- Prefer structs with methods over freestanding functions when they encapsulate shared state.
- Use named imports instead of wildcard imports.
- Keep test-only functions, types, and modules out of production code paths. Put them under `mod tests` or a dedicated `mod test_support`, and use `#[cfg(test)]` only to gate those modules.

# Build Verification

- After implementation changes, run the repository's formatter and linter. For Rust, run `just fmt` and `just clippy`; fall back to `cargo fmt` and `cargo clippy` when no justfile exists.

# Long-Running Commands

- Never keep an agent or subagent active solely to poll a process. Do not repeatedly invoke `exec`, `write_stdin`, or `wait` just to check liveness. For commands expected to exceed two minutes, prefer a detached process or runtime-managed background job that records durable completion state, then end the agent turn until there is new information.
- Treat completion notifications as wake-up hints, not the source of truth. Give each background job a durable job ID, atomic status or result file, start time, expected duration, and hard deadline. The process or a local supervisor must record a terminal `succeeded`, `failed`, or `timed_out` state even if notification delivery fails.
- When the thread next wakes, reconcile durable state before acting: read the status file, verify the recorded process identity if the state is nonterminal, and report or recover stale jobs whose deadline passed. Never infer that a missing notification means the process is still running.
- If the runtime supports event-driven completion, register it together with one bounded deadline fallback. If it does not, tell the user that automatic resumption is unavailable and provide the job ID, status path, and a command or time for one later check. Do not simulate notifications with model-driven polling.
- If live model-driven monitoring is explicitly required, first warn that every poll can consume quota and obtain user approval for the cadence and maximum monitoring budget.

# Testing

- Add or update tests when they protect user-visible behavior, reproduce a bug, cover compatibility or migration risk, or lock down a non-obvious invariant.
- Do not add tests that only restate edited literals or implementation details. Identify the behavior or invariant the test would protect before adding it.
- For static configuration or list changes, prefer compile or lint verification unless selection, fallback, parsing, migration, or filtering behavior needs coverage.

# Subagents

- Delegate only bounded, independent work when doing so is likely to reduce total context, cost, or elapsed time; prefer phase-sized, non-overlapping slices.
- Give each worker a self-contained objective, evidence surface, relevant files or commands, constraints, ownership boundary, expected concise result, and an effort and runtime budget. Use `agents.spawn_agent` with `fork_turns="none"` by default, and fork only the smallest context that cannot be supplied explicitly.
- Where supported, use `low` effort for mechanical work, `medium` by default, `high` for complex work and reviews, and `xhigh` only for exceptional architectural, safety-critical, or unusually ambiguous work. Bound reviews by scope, evidence, runtime, and rounds; prefer one broad review and at most one targeted follow-up.
- Keep architecture, integration, known-finding repair, and final verification with the primary agent. Avoid duplicating active work, inspect delegated results, and require reports of changed files, verification, risks, and integration notes. After the review cap, fix actionable findings directly and record genuine blockers or residual risks.
- In codex never use `service_tier: priority` unless the user explicitly requests it, always default to omiting it.
