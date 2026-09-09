# Workflow

- Ship production-quality changes. For changes to domain state, public interfaces, or ownership, model the domain first and use typed models to exclude invalid states. Prefer the proper owner or abstraction over caller-specific conditionals. Small local changes should follow established patterns without a separate architecture exercise. Repeated fixes in one area signal that the model may be wrong; revisit it and remove resulting shortcuts before finishing.
- Encode recurring corrections as types, tests, lints, scripts, or runtime checks instead of repeating instructions.

# General

- The code explains what; comments explain why. Comment non-obvious decisions, constraints, and tradeoffs. Start inline comments lowercase and higher-level doc comments with a capital letter; do not end comments with periods or make them depend on conversation context. Document every public API in libraries.
- Report to the user only in ASD-STE100 Simplified Technical English.
- For commits, follow `$HOME/.agents/commit-message-guide.md`; use Praveen Perera when an author is needed, and never add Claude/Codex/AI co-authors or generated-by notes.
- Minimize nesting in functions.
- Do not leave deprecated code in place by default. Remove it, or ask whether the change must preserve the old path.
- Put ad hoc files the user may want to inspect, such as Markdown, HTML, screenshots, and image-generation outputs, in a repo-root `_scratch/` directory and create it if needed.
- In public-facing copy, include only reader-visible content. Omit implementation notes, source labels, workflow state, reasoning, conversation context, and edit instructions.
- Preserve unrelated user or agent changes. Use hunk staging for commits and never undo unrelated edits.

# Codex Specific

- NEVER USE THE RESET USAGE TOOL. NEVER RESET USAGE.
- Use `fork_turns="none"` for Codex subagents by default, and give each subagent a self-contained prompt. Use `fork_turns="all"` only when the user explicitly requests a full-history fork.
- Astra is the smartest and best for high-level thinking, Sol is best for normal root work and review, and Luna `max` is best for bounded implementation after the design is settled.
- Prefer Sol `high` as root, Luna `max` for bounded implementation. The actual root owns scope, integration, and acceptance. For a larger task, use one fresh read-only Sol review of the related change set; add another only when a second risk area would bloat that prompt. Do not start one reviewer per package or file, and do not repeat completed checks.
- Reserve Astra for complex planning, architecture, and difficult review: `medium` by default, `low` for focused work, and `high` or above only on explicit user request or when stuggling with a solution.
- Use Astra or Opus 5 for overall front-end design; Sol can implement and extend established designs. Use Astra to simplification and code removal from the root's complexity-check list. Fable 5.1 is opt-in only; the root may suggest it. Opus defaults to `medium`.
- Infer intent and scope from the request, complete the work that is already authorized before asking a clarifying question, and do not add unsolicited warnings, disclaimers, or approval flows.
- The user's instructions take precedence over a skill's guidelines. When a skill rule blocks progress, cite the exact `SKILL.md` file and rule instead of stopping.
- Spawn a subagent when a self-contained `fork_turns="none"` prompt drops unused parent context, or when non-overlapping owned scopes can run at the same time. Extra agents are useful when each prompt stays small. Do not spawn when the child would reload the same large context, the scopes overlap, the current thread already has the needed files, or a nested agent would review or edit the same work. Only the root may spawn unless the user or the root's contract authorizes nested delegation.

# Rust Project Specific

- Unless asked, do not set an MSRV for new Rust projects; default to stable.
- `info` and `error` logs may start with uppercase letters.
- In log and `println!` macros, prefer inline variable capture such as `warn!("person id={id} ...")` over positional placeholders.
- For unfamiliar crates or external libraries, inspect documentation or source instead of guessing. Check `target/doc/`, run `cargo doc -p <crate-name>`, inspect `~/.cargo/registry/src`, or use `btx` to look at the code directly.
- When clippy reports autofixable issues, run `cargo fix --allow-dirty` only when the working tree and command scope make it safe from unrelated changes; otherwise apply the fixes manually. Fix remaining lints directly instead of silencing them with `allow` or `warn` unless there is a specific reason.
- Prefer `eyre`, or `color-eyre` for CLIs, over `anyhow`.
- Use the Rust 2018+ module layout instead of `mod.rs` for regular modules.
- Avoid redundant closures; use `.map(func)` instead of `.map(|value| func(value))`.
- Prefer tuple structs over named-field structs for simple wrappers, such as `struct Foo(Arc<Inner>)`.
- Prefer structs with methods over freestanding functions when they encapsulate shared state.
- Use named imports instead of wildcard imports.
- Add a blank line after a multi-line construct before the next statement or block, regardless of its closing syntax. Also use blank lines to separate distinct logical phases in a function. A related single-line statement can stay with the block that follows it when separation would add noise. Keep a short, single-phase body together, and do not add a blank line only before the final expression.
- Keep test-only functions, types, and modules out of production code paths. Put them under `mod tests` or a dedicated `mod test_support`, and use `#[cfg(test)]` only to gate those modules.
- Prefer turso + toasty orm with compile-time typed checked queries over raw sqlite

# Docker image builds

- Use the global `rb` skill before choosing a container image build command
- Publish public images to Docker Hub

# Verification

- After implementation changes, run the repository's formatter and linter. For Rust, run `just fmt` and `just clippy`; fall back to `cargo fmt` and `cargo clippy` when no justfile exists.
- Run the checks appropriate to the change. Reuse reliable results for unchanged code; repeat or broaden checks only for new changes, failures, missing evidence, or unresolved concerns.

# Testing

- Add or update tests when they protect user-visible behavior, reproduce a bug, cover compatibility or migration risk, or lock down a non-obvious invariant.
- Do not write tests for reversible, low-impact changes that only restate edited literals or mirror the implementation.
- For static configuration or list changes, prefer compile or lint verification unless selection, fallback, parsing, migration, or filtering behavior needs coverage.

# Skills

- Reuse skill instructions that are already present in the active conversation context across user turns. Do not reread or check the same `SKILL.md` or its required references only because a new user turn started.
