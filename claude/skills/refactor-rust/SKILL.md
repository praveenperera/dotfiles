---
name: refactor-rust
description: Refactor Rust code for clarity, structure, and maintainability. Splits large modules, reduces nesting, extracts functions, then applies style conventions. Use when asked to "refactor this Rust", "clean up this module", "split this file", "simplify this Rust code", or `/refactor-rust`.
---

# Refactor Rust

## Goal

Code should be easy to read, easy to follow, easy to review, and easy to change. But "no refactor needed" is a valid and good outcome. Don't refactor for refactoring's sake — only change code that has real readability, maintainability, or correctness pain.

## Setup

Read the project's CLAUDE.md for project-specific rules before starting.

Preserve behavior by default. If a refactor exposes a correctness bug, isolate it, write a reproducing test, and report it to the user — don't fix it in the same pass. Ask the user before major module splits.

Discover repo-specific verification first: check for a justfile, then fall back to `cargo fmt` / `cargo clippy` / `cargo test`. Run tests before and after.

## Scope

- **Recent changes (default)**: Only refactor recently touched code — could mean uncommitted changes, commits on this branch, or the last day or two of work. Ask the user to confirm scope before starting
- **Full sweep**: When the user explicitly asks to refactor a whole module, crate, or codebase

Infer from context: bare `/refactor-rust` or "clean up my changes" = recent. "Refactor this codebase" or "refactor this module" = full sweep. When unsure, ask.

## Findings First

Before any refactoring, produce a findings report. Start skeptical — do not assume a refactor is justified.

- Review the code in scope and list concrete problems: maintainability issues, correctness risks, testing gaps, unclear control flow
- Every finding must point to specific code with a concrete reason — not "this file is large" or "this could be cleaner"
- Separate actionable problems from code smells. Code smells alone don't justify a refactor
- If the code is already in good shape, say so explicitly and stop
- Order findings by severity. Only proceed to Pass 1/2 for findings that warrant action

## Pass 1: Structure

Only proceed if Findings First identified structural issues that warrant action.

Spawn an agent focused exclusively on structural refactoring. Do not touch style in this pass. Clarity over brevity — explicit code is often better than overly compact.

### Module splitting
- Use line count as a heuristic, not a trigger — large modules may warrant splitting by concern, but too many tiny files can make code harder to follow
- Use Rust 2018+ layout: `module_name.rs` + `module_name/nested.rs`, never `mod.rs`

### Function extraction
- Functions doing multiple things: extract helpers
- Each function should have a single clear responsibility
- Same cohesion warning: too many tiny helpers can hurt readability more than a slightly long function

### Nesting reduction
- Early returns and guard clauses at function start
- `?` operator instead of match on Result/Option
- Collapse nested if-lets into chains with `&&`
- Flatten deeply nested control flow
- Prefer pattern matching over nested conditionals
- Sometimes an imperative loop is easier to reason about than a complex iterator chain — prefer clarity

### Type design
- Make impossible states impossible — encode logic and system state in the type system when it makes things clearer, so the compiler rejects invalid states rather than relying on runtime checks
- Reduce booleans — use enums and state machines instead
- Prefer structs with methods over freestanding functions
- Methods on structs with state, not passing state as function arguments
- Tuple structs for simple wrappers (e.g., `struct Foo(Arc<Inner>)`)
- Newtypes over primitives for domain concepts (e.g., `UserId(u64)` not `u64`)
- Avoid gratuitous `.clone()` where a borrow works just as well (it's noise), but don't contort ownership to avoid a clone when cloning is the clearest option
- If the project already uses actors (`ractor` or `act_zero` for async; crossbeam channels with small actor structs for sync), `Arc<Mutex<T>>` patterns are likely actor candidates — refactor to match, but consider that some one-off Mutex usage may be intentional. If the project doesn't use actors, mention it once as an option but don't repeat if already discussed

### Error handling
- `.unwrap()` only in tests; `.expect()` sparse in prod, mostly for early error-out
- eyre over anyhow; color_eyre for CLIs; `thiserror` for library code errors
- `.context()` / `.wrap_err()` to add meaning at each error layer

### Dead code and suppression audit
- Review `#[allow(dead_code)]` and unused code — remove if genuinely dead, don't leave deprecated code in place
- Review other `#[allow(...)]` attributes — sometimes needed, but often a smell hiding something that needs a restructure rather than a suppression

### Verify
- Run repo-specific lint and test commands discovered during setup

## Pass 2: Style

Spawn a separate agent focused on style and conventions. Always runs if Pass 1 ran. May also run independently for style-only cleanup even without structural findings.

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
- `#[must_use]` on functions with important return values
- `&str` over `String` in params when ownership not needed; `impl AsRef<Path>` / `impl Into<String>` for flexible params
- Conversion naming: `as_` (borrow), `to_` (expensive), `into_` (consume)
- `matches!()` macro for simple boolean pattern checks
- `derive_more` for common traits: `From`, `Into`, `Deref`, `Display`
- `tracing` for logging; `println!` is fine in CLIs for user-facing output

### Verify
- Run repo-specific fmt, lint, and test commands discovered during setup

## Refactoring Discipline

- When refactoring code that calls external crates, read the dependency source to verify behavior — use `/rust-crate-source` or `/btx`, or check `~/.cargo/registry/src/`
- When code has documented assumptions, trace the data flow backwards to verify callers satisfy them
- Refactoring is an opportunity to catch correctness bugs — question the logic, not just the structure
