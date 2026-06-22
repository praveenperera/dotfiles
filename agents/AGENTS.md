# Workflow

- Scope changes precisely to the request. If needed changes exceed the planned or requested scope, ask before proceeding
- Do not leave tech debt or shortcuts behind; go back and do them right before finishing
- I am explicitly authorizing the use of the subagent tool as needed, use subagents freely and widely

# General

- Only add important comments explaining why; inline comments start lowercase, higher level doc comments start capitalized, and comments should not require conversation context or end with a period
- For commits, follow `$HOME/.agents/commit-message-guide.md`; use Praveen Perera when an author is needed, and never add Claude/Codex/AI co-authors or generated-by notes
- Minimize nesting in functions
- Don't default to leaving deprecated code in place, remove it or ask if this is a full replacement or if old code is still needed
- Put ad hoc files the user may want to inspect, such as Markdown, HTML, screenshots, and image-generation outputs, in a repo-root `_scratch/` directory and create it if needed
- When writing public-facing copy, output only reader-visible content and omit implementation notes, source labels, workflow state, or edit instructions
- When working with this user's projects: always read existing config/code before answering from general knowledge. Never assume defaults — check the actual files first
- Preserve unrelated user/agent changes. Use hunk staging for commits and never undo unrelated edits

# Rust Project Specific

- `info` and `error` logs are okay to start capitalized
- log/println! macros, prefer inline variable capture like `warn!("person id={id} ...")` instead of positional placeholders like `warn!("person id={} ...", id)`
- For unfamiliar crates or external libraries, prefer docs and `btx` over guessing; check `target/doc/`, run `cargo doc -p <crate-name>`, use `btx`, or inspect `~/.cargo/registry/src` as appropriate
- Whenever you get clippy errors first run cargo fix --allow-dirty and then fix whatever remains
- I always prefer eyre (color-eyre if cli) to anyhow
- Don't use `mod.rs` for regular modules, prefer the Rust 2018+ layout with `module_name.rs` and nested modules in `module_name/nested_module_name.rs`
- if-let chains are stable in Rust now, always collapse nested if-lets into a single statement using `&&`
- Avoid redundant closures - use `.map(func)` instead of `.map(|x| func(x))`
- Prefer tuple structs over named field structs for simple wrappers (e.g., `struct Foo(Arc<Inner>)` not `struct Foo { inner: Arc<Inner> }`)
- `#[act_zero_ext::into_actor_result]` on `fn foo()` generates: public async `foo() -> ActorResult<T>` wrapper + private `do_foo()` with original logic
- Prefer structs with methods over freestanding functions to encapsulate state and provide a cleaner API
- Use named imports (`use foo::{Bar, Baz}`) instead of wildcard imports (`use foo::*`)
- Keep test-only functions, types, and modules out of production code paths; put them under `mod tests` or a dedicated `mod test_support`, using `#[cfg(test)]` only to gate those test modules

# Build Verification

- For Rust projects: always run `just fmt` and `just clippy` after changes (most projects use a justfile; fall back to `cargo fmt` and `cargo clippy` if no justfile exists). For Android/Kotlin: verify builds compile. For iOS/Swift: verify builds compile. Never submit changes without verifying they compile

# Testing

- Add or update tests when they protect user-visible behavior, reproduce a bug, cover compatibility/migration risk, or lock down a non-obvious invariant
- Do not add tests that only restate edited literals or implementation details; before adding a test, identify the behavior or invariant it would catch
- For static config/list changes, prefer compile/lint verification unless there is selection, fallback, parsing, migration, or filtering behavior that needs coverage

# Subagents

- Prefer using subagents freely when they keep the main context focused or reduce low-signal code, logs, or search results
- Use subagents for parallel investigation, especially when exploring independent parts of a codebase, comparing multiple possible implementations, checking generated docs, or inspecting external references
- Use subagents for bounded implementation work when the write scope is clear and can be kept separate from other edits
- Use subagents for verification work that can happen alongside implementation, such as running focused tests, checking platform-specific build output, reviewing UI screenshots, or auditing likely regressions
- Give subagents concrete ownership, concise task boundaries, and instructions to summarize only the findings or changed files that matter to the main task
- Keep urgent blocking work in the main thread when waiting for a subagent would slow down the next necessary step
