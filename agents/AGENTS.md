# Workflow

- Scope changes precisely to the request. If needed changes exceed the planned or requested scope, ask before proceeding
- Do not leave tech debt or shortcuts behind; go back and do them right before finishing
- I authorize subagent use according to the guidance below

# General

- Only add important comments explaining why; inline comments start lowercase, higher level doc comments start capitalized, and comments should not require conversation context or end with a period
- For commits, follow `$HOME/.agents/commit-message-guide.md`; use Praveen Perera when an author is needed, and never add Claude/Codex/AI co-authors or generated-by notes
- Minimize nesting in functions
- Don't default to leaving deprecated code in place, remove it or ask if this is a full replacement or if old code is still needed
- Put ad hoc files the user may want to inspect, such as Markdown, HTML, screenshots, and image-generation outputs, in a repo-root `_scratch/` directory and create it if needed
- When writing public-facing copy, output only reader-visible content and omit implementation notes, source labels, workflow state, or edit instructions
- When working with this user's projects: always read existing config/code before answering from general knowledge. Never assume defaults — check the actual files first
- Preserve unrelated user/agent changes. Use hunk staging for commits and never undo unrelated edits

# Background Commands

- For long-running non-interactive commands that should resume work when finished, use `exec_command(..., on_exit: "wake")`; after they yield, do not poll with `write_stdin` or send status-only updates
- Keep `on_exit: "none"` for interactive commands, commands requiring stdin, and commands whose completion should not start another turn
- Give a delegated checker the exact command and working directory. It must use completion wakeup, avoid polling or interim messages, and return exactly one final response, which the parent receives through its mailbox. Require the exact command, exit status, duration, concise result, first actionable failure, and artifact paths. It must not diagnose or edit unless assigned, and the primary agent must not poll or duplicate its run

# Progress Communication

- Make progress communication event-driven, not timer-driven. Send updates for meaningful state changes, actionable failures, integration checkpoints, user decisions, or final results; do not send periodic heartbeats merely to report that work is still running
- While waiting for delegated work or a background command, rely on completion wakeups and final mailbox reports instead of status-only updates

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

- Use subagents for bounded, context-isolated work when delegation is likely to reduce total context or token usage, such as targeted exploration, log inspection, external research, or focused verification
- Launch one with `agents.spawn_agent`, `fork_turns="none"`, and the chosen `reasoning_effort`. Non-forked is the default because inherited parent history is carried through later child model cycles and can compound input-token usage
- Make every non-forked prompt self-contained. Include the exact objective, relevant files or commands, constraints, concrete ownership, and expected concise output; point to repository files or raw artifacts when available and state any other necessary context directly
- Choose each subagent's effort level based on task difficulty; default to `medium`, reserve `high` and `xhigh` for work that genuinely requires deeper reasoning, and use `low` for simple, mechanical tasks whose results can be verified cheaply
- Fork only when an essential recent decision cannot be supplied through repository files, raw artifacts, or a concise prompt without making the task costly or unsafe. Use the smallest positive `fork_turns` value that supplies the missing context. Do not use `fork_turns="all"` unless I explicitly request it
- Default implementation work to at most two concurrent workers. Prefer larger, phase-sized, non-overlapping ownership slices over many small assignments. Each worker verifies its own slice and returns one concise final report with changed files, verification results, risks, and integration notes
- While a worker wave is active, the primary agent may do unrelated work but should not reread worker-owned files or duplicate focused verification. After all workers in the wave finish, perform one integration checkpoint: inspect the combined diff, reconcile boundaries, run cross-cutting checks, and update durable progress or audit state. Do not launch another implementation wave before integrating the current one
- Exceed two concurrent implementation workers only when the additional slices are genuinely independent and the expected savings clearly exceed coordination and context costs
- The primary agent remains responsible for integration and verification of delegated results
