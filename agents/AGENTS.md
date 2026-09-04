# Workflow

- Ship production-quality changes. Model the domain first, make impossible states impossible with typed domain models, and prefer the proper owner or abstraction over caller-specific conditionals. Repeated fixes in one area signal that the model may be wrong; revisit it and remove shortcuts or resulting tech debt before finishing.
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

- Default to the global `rb` skill for container image builds. Prefer `rb build --project <name> -- [buildx args…]` over local `docker build` and `docker buildx build`.
- Use local Docker only when the user asks for a local build, or when `rb` is unavailable and the user accepts that fallback.
- Load `$rb` / the `rb` skill before inventing a build command. Do not print control-plane tokens or project SSH private keys.

# Skills

- Reuse skill instructions that are already present in the active conversation context across user turns. Do not reread or check the same `SKILL.md` or its required references only because a new user turn started.
