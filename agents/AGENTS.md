# Workflow

- Do not leave tech debt or shortcuts behind; go back and do them right before finishing
- Make impossible states impossible; prefer typed domain models over caller-specific conditionals, always figure out the proper data model first, avoid hacks and shortcuts, repeated bug fixes in the same area of the code might point to not having the right data model
- I authorize subagent use according to the guidance below

# General

- Only add important comments explaining why; inline comments start lowercase, higher level doc comments start capitalized, and comments should not require conversation context or end with a period; libraries should have all public APIs documented
- For commits, follow `$HOME/.agents/commit-message-guide.md`; use Praveen Perera when an author is needed, and never add Claude/Codex/AI co-authors or generated-by notes
- Minimize nesting in functions
- Don't default to leaving deprecated code in place, remove it or ask if this is a full replacement or if old code is still needed
- Put ad hoc files the user may want to inspect, such as Markdown, HTML, screenshots, and image-generation outputs, in a repo-root `_scratch/` directory and create it if needed
- When writing public-facing copy, output only reader-visible content and omit implementation notes, source labels, workflow state, thinking/reasoning behind the copy, or edit instructions; dont explain yourself in public facing copy
- Preserve unrelated user/agent changes. Use hunk staging for commits and never undo unrelated edits

# Rust Project Specific

- `info` and `error` logs are okay to start capitalized
- log/println! macros, prefer inline variable capture like `warn!("person id={id} ...")` instead of positional placeholders like `warn!("person id={} ...", id)`
- For unfamiliar crates or external libraries, prefer docs and `btx` over guessing; check `target/doc/`, run `cargo doc -p <crate-name>`, use `btx`, or inspect `~/.cargo/registry/src` as appropriate
- Whenever you get clippy errors first run cargo fix --allow-dirty and then fix whatever remains
- When there is lints caught by clippy, avoid using `allow` or `warn` to silence them; instead, fix the lints, unless there is a good reason not to
- I always prefer eyre (color-eyre if cli) to anyhow
- Don't use `mod.rs` for regular modules, prefer the Rust 2018+ layout with `module_name.rs` and nested modules in `module_name/nested_module_name.rs`
- if-let chains are stable in Rust now, always collapse nested if-lets into a single statement using `&&`
- Avoid redundant closures - use `.map(func)` instead of `.map(|x| func(x))`
- Prefer tuple structs over named field structs for simple wrappers (e.g., `struct Foo(Arc<Inner>)` not `struct Foo { inner: Arc<Inner> }`)
- Prefer structs with methods over freestanding functions to encapsulate state and provide a cleaner API
- Use named imports (`use foo::{Bar, Baz}`) instead of wildcard imports (`use foo::*`)
- Keep test-only functions, types, and modules out of production code paths; put them under `mod tests` or a dedicated `mod test_support`, using `#[cfg(test)]` only to gate those test modules

# Build Verification

- For Rust projects: always run `just fmt` and `just clippy` after changes (most projects use a justfile; fall back to `cargo fmt` and `cargo clippy` if no justfile exists). For Android/Kotlin: verify builds compile. For iOS/Swift: verify builds compile. Never submit changes without verifying they compile.
  - The only exception is when i'm asking you to just commit changes, assume that means the work was already verified.

# Testing

- Add or update tests when they protect user-visible behavior, reproduce a bug, cover compatibility/migration risk, or lock down a non-obvious invariant
- Do not add tests that only restate edited literals or implementation details; before adding a test, identify the behavior or invariant it would catch
- For static config/list changes, prefer compile/lint verification unless there is selection, fallback, parsing, migration, or filtering behavior that needs coverage

# Subagents

- Use subagents for bounded, context-isolated work when delegation is likely to reduce total context or token usage, such as targeted exploration, log inspection, external research, or focused verification
- Launch one with `agents.spawn_agent`, `fork_turns="none"`, and the chosen `reasoning_effort`. Non-forked is the default because inherited parent history is carried through later child model cycles and can compound input-token usage
- Make every non-forked prompt self-contained. Include the exact objective, relevant files or commands, constraints, concrete ownership, and expected concise output; point to repository files or raw artifacts when available and state any other necessary context directly
- Choose each subagent's effort level based on task difficulty; default to `medium`, reserve `high` and `xhigh` for work that genuinely requires deeper reasoning, and use `low` for simple, mechanical tasks whose results can be verified cheaply
- Before launching any delegated review, define its scope, reasoning effort, time or runtime budget, evidence surface, and maximum review rounds. Default reviews to `high`; reserve `xhigh` for exceptional architectural, safety-critical, or unusually ambiguous work, `med` for quick reviews
- Never define completion as launching fresh broad reviewers until one reports no findings. After the review-round cap, the primary agent must integrate and fix remaining actionable findings, run targeted verification, and record any genuine unresolved blocker or residual risk. The cap limits review repetition, not completion criteria, and never dismisses a known blocker
- Prefer larger, phase-sized, non-overlapping ownership slices over many small assignments. Each worker verifies its own slice and returns one concise final report with changed files, verification results, risks, and integration notes
- While a worker wave is active, the primary agent may do unrelated work but should not reread worker-owned files or duplicate focused verification. After all workers in the wave finish, perform one integration checkpoint: inspect the combined diff, reconcile boundaries, run cross-cutting checks, and update durable progress or audit state.
- When waiting for a long-running command or subagent, choose the wait timeout from the expected remaining duration, capped at 120 secs. Prefer one appropriately sized wait over repeated short polls
- The primary agent remains responsible for integration and verification of delegated results
