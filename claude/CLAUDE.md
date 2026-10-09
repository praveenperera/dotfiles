# Workflow

- Encode recurring corrections as types, tests, lints, scripts, or runtime checks instead of repeating instructions.
- For performance work, profile and measure the end-to-end payoff first (same output or accuracy within limits, plus a measured speedup on a fixed input set); add harness or proof machinery only for risks that check can't catch, and time-box it.

# General

- The code explains what; comments explain why. Comment non-obvious decisions, constraints, and tradeoffs. Start inline comments lowercase and higher-level doc comments with a capital letter; do not end comments with periods or make them depend on conversation context. Document every public API in libraries.
- At the end of a long run, list what needs my input first, then the summary.
- Mark anything you couldn't confirm, and say where you looked.
- Always give times in central US time zone
- For commits, follow `$HOME/.agents/commit-message-guide.md`; use Praveen Perera when an author is needed, and never add Claude co-authors or generated-by notes; Never add `Claude-Session:` trailers to commit messages.
- Prefer ReScript for new JavaScript-targeting application logic when practical. When TypeScript is required, use the Effect library for stronger type safety: model expected errors and dependencies explicitly, and validate external data with Effect Schema instead of unchecked type assertions.
- Do not leave deprecated code in place by default. Remove it, or ask whether the change must preserve the old path.
- Put ad hoc files the user may want to inspect, such as Markdown, HTML, screenshots, and image-generation outputs, in a repo-root `_scratch/` directory and create it if needed. Those directories are wiped weekly; do not keep needed files there. Keep durable notes in `~/code/research`.
- In public-facing copy, include only reader-visible content. Omit implementation notes, source labels, workflow state, reasoning, conversation context, and edit instructions.
- Preserve unrelated user or agent changes. Use hunk staging for commits and never undo unrelated edits.
- Never give local only 127.0.0.1 links always use host mode so links work on LAN and tailscale
- When a step doesn't need my input, keep going. Put status notes in the same message as your next action. Stop and ask only when you can't continue without me, or before anything destructive: deleting data, force-pushing, or changing anything outside this repository that I have not already approved.
- For long multi-step runs, keep a checklist in `_scratch/<task-name>/TASKS.md` so concurrent runs do not share one file. Tick each item when it's done, and add anything new you find.
- The user's instructions take precedence over a skill's guidelines. When a skill rule blocks progress, cite the exact `SKILL.md` file and rule instead of stopping.
- Load the `use-agents` skill before spawning any subagent or delegating work; it owns model routing, effort levels, when to use `homebased`, and the long-command handoff. The root owns scope, design decisions, integration, and acceptance. Default to use homebased for any non trivial agent work.

# Rust Project Specific

- Unless asked, do not set an MSRV for new Rust projects; default to stable.
- For unfamiliar crates or external libraries, inspect documentation or source instead of guessing. Check `target/doc/`, run `cargo doc -p <crate-name>`, inspect `~/.cargo/registry/src`, or use `btx` to look at the code directly.
- When clippy reports autofixable issues, run `cargo fix --allow-dirty` only when the working tree and command scope make it safe from unrelated changes; otherwise apply the fixes manually. Fix remaining lints directly instead of silencing them with `allow` or `warn` unless there is a specific reason.
- Prefer `eyre`, or `color-eyre` for CLIs, over `anyhow`.
- Use the Rust 2018+ module layout instead of `mod.rs` for regular modules, use edition 2024 not 2021 for new projects.
- Avoid redundant closures; use `.map(func)` instead of `.map(|value| func(value))`.
- Prefer tuple structs over named-field structs for simple wrappers, such as `struct Foo(Arc<Inner>)`.
- Prefer structs with methods over freestanding functions when they encapsulate shared state.
- Use named imports instead of wildcard imports.
- Add a blank line after a multi-line construct before the next statement or block, regardless of its closing syntax. Also use blank lines to separate distinct logical phases in a function. A related single-line statement can stay with the block that follows it when separation would add noise. Keep a short, single-phase body together, and do not add a blank line only before the final expression.
- Keep test-only functions, types, and modules out of production code paths. Put them under `mod tests` or a dedicated `mod test_support`, and use `#[cfg(test)]` only to gate those modules.
- Prefer turso + toasty orm with compile-time typed checked queries over raw sqlite
- On the Linux container `code`, keep Cargo build output off tmpfs. Do not set `CARGO_TARGET_DIR` to a `/tmp` path; use the configured disk-backed target directory. Use a separate disk-backed target directory for isolated concurrent work instead of cleaning a shared target directory while another build may run

# Docker image builds

- On mac use `rb` to run create docker images if in a project that uses another builder like `sht` use that by default
- If on code/ai5090 or another linux box just build the docker image directly on the machine
- Publish public images to Docker Hub

# Testing

- Do not write tests for reversible, low-impact changes that only restate edited literals or mirror the implementation.
- Before keeping a test, ask whether it would still pass if every function it calls returned nothing. If it would, delete it.
- Prefer integration tests over lots of small unit tests
