# General

- Stop puttting Created by claude code into my files, if it has an author use me Praveen Perera
- All comments should make sense without the context if this particular conversation
- If creating git commits never co author the commit with claude/codex or add any notes about claude just use my name
- Start inline code comments with a lowercase
- Capitalize higher level doc comments like functions and modules (ex: in rust comments starting with /// instead of //)
- Don't end comments with a period (periods within comments are fine)
- Only add important comments to explain why something is being done
- Try to minimize nesting in functions
- Don't default to leaving deprecated code in place, remove it or ask if this is a full replacement or if old code is still needed
- When working with this user's projects: always read existing config/code before answering from general knowledge. Never assume defaults — check the actual files first
- Scope changes precisely to what the user asks for. Do not modify files or components beyond the explicit request without asking first. If unsure about scope, ask before making changes — not after

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

# Build Verification

- For Rust projects: always run `just fmt` and `just clippy` after changes (most projects use a justfile; fall back to `cargo fmt` and `cargo clippy` if no justfile exists). For Android/Kotlin: verify builds compile. For iOS/Swift: verify builds compile. Never submit changes without verifying they compile

# Project Context

- This user's primary stack: Rust (dominant), TypeScript/Svelte, Kotlin (Android), Swift (iOS). Cross-platform mobile wallet app with Rust core. Also: Terraform/infrastructure, web scraping tools in Rust
