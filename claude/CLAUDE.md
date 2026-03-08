# Core Rules

- Stop puttting Created by claude code into my files, if it has an author use me Praveen Perera
- Don't add comments that need old removed code to make sense in context
- All comments should make sense without the context if this particular conversation
- If creating git commits never co author the commit with claude or add any notes about claude just use my name
- Start inline code comments with a lowercase
- Capitalize higher level doc comments like functions and modules (ex: in rust comments starting with /// instead of //)
- Don't end comments with a period (periods within comments are fine)
- Only add important comments to explain why something is being done
- Try to minimize nesting in functions
- Separate distinct logical steps within functions with blank lines, but not between a comment and the code it describes
- When I report a bug, don't start by trying to fix it. Instead, start by writing a test that reproduces the bug. Then, have subagents try to fix the bug and prove it with a passing test
- Don't default to leaving deprecated code in place, remove it or ask if this is a full replacement or if old code is still needed
- When working with this user's projects: always read existing config/code before answering from general knowledge. Never assume defaults — check the actual files first
- Scope changes precisely to what the user asks for. Do not modify files or components beyond the explicit request without asking first. If unsure about scope, ask before making changes — not after

# Refactoring Discipline

- When refactoring code that calls external crates, read the dependency source to verify behavior — don't trust variable names or comments. Use `/rust-crate-source` or `/btx` skills, or check `~/.cargo/registry/src/` to read crate source
- When code has documented assumptions, trace the data flow backwards to verify callers satisfy those assumptions
- Refactoring is an opportunity to catch correctness bugs, not just move code around — question the logic, not just the structure

# Rust Project Specific

- `info` and `error` logs are okay to start capitalized
- log/println! macros, prefer inline variable capture like `warn!("person id={id} ...")` instead of positional placeholders like `warn!("person id={} ...", id)`
- Generate docs for a crate with `cargo doc -p <crate-name>`, this will then be available at `target/doc/<crate-name>/index.html`, if you are unsure about how to use a crate, please generate the docs and read them
- If docs have already been generated, check `target/doc/` for existing documentation of the project and its dependencies before regenerating
- Whenever you get clippy errors first run cargo fix --allow-dirty and then fix whatever remains
- I always prefer eyre (color-eyre if cli) to anyhow
- if-let chains are stable in Rust now, always collapse nested if-lets into a single statement using `&&`
- Avoid redundant closures - use `.map(func)` instead of `.map(|x| func(x))`
- Prefer tuple structs over named field structs for simple wrappers (e.g., `struct Foo(Arc<Inner>)` not `struct Foo { inner: Arc<Inner> }`)
- `#[act_zero_ext::into_actor_result]` on `fn foo()` generates: public async `foo() -> ActorResult<T>` wrapper + private `do_foo()` with original logic
- Prefer structs with methods over freestanding functions to encapsulate state and provide a cleaner API

# Python Project Specific

- Always use `uv` NEVER pip

# Build Verification

- For Rust projects: always run `just fmt` and `just clippy` after changes (most projects use a justfile; fall back to `cargo fmt` and `cargo clippy` if no justfile exists). For Android/Kotlin: verify builds compile. For iOS/Swift: verify builds compile. Never submit changes without verifying they compile

# Project Context

- This user's primary stack: Rust (dominant), TypeScript/Svelte, Kotlin (Android), Swift (iOS). Cross-platform mobile wallet app with Rust core. Also: Terraform/infrastructure, web scraping tools in Rust

## Browser Automation

Use `agent-browser` for web automation. Run `agent-browser --help` for all commands.

Core workflow:

1. `agent-browser open <url>` - Navigate to page
2. `agent-browser snapshot -i` - Get interactive elements with refs (@e1, @e2)
3. `agent-browser click @e1` / `fill @e2 "text"` - Interact using refs
4. Re-snapshot after page changes
