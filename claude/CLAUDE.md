# Workflow

- Ship production-quality changes. For changes to domain state, public interfaces, or ownership, model the domain first and use typed models to exclude invalid states. Prefer the proper owner or abstraction over caller-specific conditionals. Small local changes should follow established patterns without a separate architecture exercise. Repeated fixes in one area signal that the model may be wrong; revisit it and remove resulting shortcuts before finishing.
- Encode recurring corrections as types, tests, lints, scripts, or runtime checks instead of repeating instructions.
- For performance work, profile and measure the end-to-end payoff first (same output or accuracy within limits, plus a measured speedup on a fixed input set); add harness or proof machinery only for risks that check can't catch, and time-box it.

# General

- The code explains what; comments explain why. Comment non-obvious decisions, constraints, and tradeoffs. Start inline comments lowercase and higher-level doc comments with a capital letter; do not end comments with periods or make them depend on conversation context. Document every public API in libraries.
- At the end of a long run, list what needs my input first, then the summary.
- Mark anything you couldn't confirm, and say where you looked.
- Always give times in central US time zone
- For commits, follow `$HOME/.agents/commit-message-guide.md`; use Praveen Perera when an author is needed, and never add Claude/Codex/AI co-authors or generated-by notes.
- Minimize nesting in functions.
- Prefer ReScript for new JavaScript-targeting application logic when practical. When TypeScript is required, use the Effect library for stronger type safety: model expected errors and dependencies explicitly, and validate external data with Effect Schema instead of unchecked type assertions.
- Do not leave deprecated code in place by default. Remove it, or ask whether the change must preserve the old path.
- Put ad hoc files the user may want to inspect, such as Markdown, HTML, screenshots, and image-generation outputs, in a repo-root `_scratch/` directory and create it if needed. Those directories are wiped weekly; do not keep needed files there. Keep durable notes in `~/code/research`.
- In public-facing copy, include only reader-visible content. Omit implementation notes, source labels, workflow state, reasoning, conversation context, and edit instructions.
- Preserve unrelated user or agent changes. Use hunk staging for commits and never undo unrelated edits.

# Claude Code Specific

- When a step doesn't need my input, keep going. Put status notes in the same message as your next action. Stop and ask only when you can't continue without me, or before anything destructive: deleting data, force-pushing, or changing anything outside this repository that I have not already approved.
- For long multi-step runs, keep a checklist in `_scratch/<task-name>/TASKS.md` so concurrent runs do not share one file. Tick each item when it's done, and add anything new you find.
- The user's instructions take precedence over a skill's guidelines. When a skill rule blocks progress, cite the exact `SKILL.md` file and rule instead of stopping.
- Use the `use-agents` skill for model routing and effort levels. The root owns scope, design decisions, integration, and acceptance.
- Opus 5.5 is the usual root. Default to Sonnet 5.5 as the subagent for most work. Use Sonnet `medium` for bounded implementation after the design is settled, tests, and migrations. Use Sonnet `high` for bug investigation and root-cause analysis, codebase triage, review, audits, verification, and computer use. Never use Sonnet `xhigh` or `max`; send a deep audit to Opus 5.5 `high` instead.
- Use Haiku 5.5 for mechanical work with a cheap check, such as renames, fixtures, boilerplate, inventories, classifications, and repeated transforms. If a Haiku pass fails its check or needs judgment, rerun it on Sonnet `medium`. Haiku is cheap only under 100K prompt tokens (its compaction is set there), so prefer many small Haiku agents, each with a slice that finishes well under 100K, over one long Haiku run.
- Use Opus 5.5 subagents for long unattended builds and heavy rewrites, merge-ready implementation that needs judgment, and front-end, UI, or design work. Opus 5.5 defaults to `medium`; use `high` when edge cases matter, `xhigh` only for rare hard problems, and never `max`.
- Save Opus for kernels and hard work. Removals, cleanups, harness or tooling deletions, history tidying, docs, and other mechanical changes go to Haiku 5.5, or Sonnet 5.5 `medium` when they need judgment about what to keep, even when they touch many files.
- Astra is a read-only advisor for big architecture or design decisions, or when the root is stuck. Fable is opt-in only; the root may suggest it.
- For a larger task, use one fresh Sonnet 5.5 `high` review of the related change set; add another only when a second risk area would bloat that prompt. Do not start one reviewer per package or file, and do not repeat completed checks.
- Default to the `homebased` skill for any agent work that is not a short task, including Sonnet, Haiku, and Opus subagents: implementation with verification, investigation, review, audits, and multi-source research. Run Claude subagents there through the Claude CLI, which also sets an explicit effort; the Agent tool inherits the session effort. When it is unclear whether a task is short, use `homebased`.
- Always use `homebased` when using a non-Claude model.
- Do not let a delegate wait inside a long build, test run, or CI watch. Its brief must tell it to hand the command to `homebased`, write `RESUME.md`, report `blocked`, and exit; the `use-agents` skill gives the protocol and how to resume each provider.
- Use the Agent tool only for a short task whose scope is narrow and the root can check and integrate its result in this turn, or when non-overlapping owned scopes can run at the same time. Give each subagent a self-contained prompt. Do not spawn when the child would reload the same large context, the scopes overlap, the current thread already has the needed files, or a nested agent would review or edit the same work.
- Never add `Claude-Session:` trailers to commit messages.

# Rust Project Specific

- Unless asked, do not set an MSRV for new Rust projects; default to stable.
- `info` and `error` logs may start with uppercase letters.
- In log and `println!` macros, prefer inline variable capture such as `warn!("person id={id} ...")` over positional placeholders.
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

# Verification

- After implementation changes, run the repository's formatter and linter. For Rust, run `just fmt` and `just clippy`; fall back to `cargo fmt` and `cargo clippy` when no justfile exists.
- Run the checks appropriate to the change. Reuse reliable results for unchanged code; repeat or broaden checks only for new changes, failures, missing evidence, or unresolved concerns.
- Never give local only 127.0.0.1 links always use host mode so links work on LAN and tailscale
- Prefer integration tests over lots of small unit tests

# Testing

- Add or update tests when they protect user-visible behavior, reproduce a bug, cover compatibility or migration risk, or lock down a non-obvious invariant.
- Do not write tests for reversible, low-impact changes that only restate edited literals or mirror the implementation.
- Before keeping a test, ask whether it would still pass if every function it calls returned nothing. If it would, rewrite it to assert a concrete result for a concrete input, or delete it.
- For static configuration or list changes, prefer compile or lint verification unless selection, fallback, parsing, migration, or filtering behavior needs coverage.
