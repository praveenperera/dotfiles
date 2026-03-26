---
name: refactor-rust
description: Refactor Rust code for clarity, structure, and maintainability. Splits large modules, reduces nesting, extracts functions, then applies style conventions. Use when asked to "refactor this Rust", "clean up this module", "split this file", "simplify this Rust code", or `/refactor-rust`.
---

# Refactor Rust

## Goal

Code should be easy to read, easy to follow, easy to review, and easy to change.

## Setup

Read the project's CLAUDE.md for project-specific rules before starting.

Never change behavior. Run `cargo test` before and after. Ask the user before major module splits.

## Pass 1: Structure

Spawn an agent focused exclusively on structural refactoring. Do not touch style in this pass.

### Module splitting
- Modules over ~300 lines: split by concern
- Use Rust 2018+ layout: `module_name.rs` + `module_name/nested.rs`, never `mod.rs`

### Function extraction
- Functions doing multiple things: extract helpers
- Each function should have a single clear responsibility

### Nesting reduction
- Early returns and guard clauses at function start
- `?` operator instead of match on Result/Option
- Collapse nested if-lets into chains with `&&`
- Flatten deeply nested control flow

### Type design
- Reduce booleans — use enums and state machines instead
- Prefer structs with methods over freestanding functions
- Methods on structs with state, not passing state as function arguments
- Tuple structs for simple wrappers (e.g., `struct Foo(Arc<Inner>)`)

### Error handling
- `.unwrap()` only in tests; `.expect()` sparse in prod, mostly for early error-out
- eyre over anyhow; color_eyre for CLIs
- `.context()` / `.wrap_err()` to add meaning at each error layer

### Clippy lint suppression audit
- Review `#[allow(...)]` attributes — sometimes needed, but often a smell hiding something that needs a restructure rather than a suppression

### Verify
- `just clippy` (fall back to `cargo clippy`)
- `cargo test`

## Pass 2: Style

Spawn a separate agent focused on style and conventions. Run after structure pass is complete.

### Comments
- Inline comments (`//`): start lowercase
- Doc comments (`///`, `//!`): start capitalized
- `SAFETY` blocks: all-caps
- No trailing period on comments
- Only "why" comments, not "what"
- Remove comments that only made sense in context of an agent conversation or that reference code no longer present

### Idiomatic patterns
- Avoid redundant closures: `.map(func)` not `.map(|x| func(x))`
- Named imports (`use foo::{Bar, Baz}`) over wildcards
- serde with derive + serde_json over manual JSON
- Inline variable capture in log macros: `warn!("id={id}")` not `warn!("id={}", id)`

### Verify
- `just fmt` (fall back to `cargo fmt`)
- `just clippy` (fall back to `cargo clippy`)

## Refactoring Discipline

- When refactoring code that calls external crates, read the dependency source to verify behavior — use `/rust-crate-source` or `/btx`, or check `~/.cargo/registry/src/`
- When code has documented assumptions, trace the data flow backwards to verify callers satisfy them
- Refactoring is an opportunity to catch correctness bugs — question the logic, not just the structure
