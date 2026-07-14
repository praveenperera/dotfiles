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
- For unfamiliar crates or external libraries, inspect documentation or source instead of guessing. Check `target/doc/`, run `cargo doc -p <crate-name>`, use `btx`, or inspect `~/.cargo/registry/src` as appropriate.
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

- After implementation changes, run the repository's formatter and compile or lint checks. For Rust, run `just fmt` and `just clippy`; fall back to `cargo fmt` and `cargo clippy` when no justfile exists. For Android/Kotlin and iOS/Swift, run the repository's documented compile check. The only exception is a request to only commit already-verified changes.
- Report verification commands and their results. If a required check cannot run or fails, report the exact command, failure, and remaining risk; do not claim successful verification.

# Testing

- Add or update tests when they protect user-visible behavior, reproduce a bug, cover compatibility or migration risk, or lock down a non-obvious invariant.
- Do not add tests that only restate edited literals or implementation details. Identify the behavior or invariant the test would protect before adding it.
- For static configuration or list changes, prefer compile or lint verification unless selection, fallback, parsing, migration, or filtering behavior needs coverage.

# Subagents

- Delegate bounded, context-isolated work only when it is likely to reduce total context, cost, or elapsed time.
- Give each worker a self-contained objective, evidence surface, relevant files or commands, constraints, ownership boundary, expected concise result, and an appropriate effort and runtime budget. Prefer phase-sized, non-overlapping slices.
- Launch Codex subagents with `agents.spawn_agent` and `fork_turns="none"` by default. Fork only the smallest recent context that cannot be supplied explicitly.
- Where supported, use `low` effort for simple mechanical work, `medium` by default, `high` for complex work and reviews, and `xhigh` only for exceptional architectural, safety-critical, or unusually ambiguous work.
- Bound reviews by scope, effort, evidence, runtime, and maximum rounds. Prefer one broad review and at most one targeted follow-up; do not repeat broad reviews until one reports no findings.
- Keep architecture, integration, known-finding repair, and final verification with the primary agent. Inspect delegated results before relying on them, avoid duplicating active work, and proceed once the results needed for the next step are available.
- Require each worker to report changed files, verification results, risks, and integration notes. After the review cap, integrate and fix actionable findings directly, then record any genuine blocker or residual risk.
