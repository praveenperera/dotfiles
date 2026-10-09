# Workflow

- Encode recurring corrections as types, tests, lints, scripts, or runtime checks instead of repeating instructions.
- When a step doesn't need my input, keep going, and put status notes in the same message as your next action. Stop and ask only when you can't continue without me, or before anything destructive: deleting data, force-pushing, or changing anything outside this repository that I have not already approved.
- For long multi-step runs, keep a checklist in `_scratch/<task-name>/TASKS.md` so concurrent runs do not share one file; tick each item when it's done and add anything new you find. At the end, list what needs my input first, then the summary.
- Put ad hoc files I may want to inspect, such as Markdown, HTML, screenshots, and image-generation outputs, in a repo-root `_scratch/` directory, creating it if needed. Those directories are wiped weekly, so keep durable notes in `~/code/research`.
- Mark anything you couldn't confirm, and say where you looked.
- My instructions take precedence over a skill's guidelines. When a skill rule blocks progress, cite the exact `SKILL.md` file and rule instead of stopping.
- For performance work, profile and measure the end-to-end payoff first (same output or accuracy within limits, plus a measured speedup on a fixed input set); add harness or proof machinery only for risks that check can't catch, and time-box it.
- Always give times in the US Central time zone.
- Never give local-only 127.0.0.1 links; use host mode so links work on LAN and Tailscale.
- In public-facing copy, include only reader-visible content. Omit implementation notes, source labels, workflow state, reasoning, conversation context, and edit instructions.

# Code

- The code explains what; comments explain why. Comment non-obvious decisions, constraints, and tradeoffs. Start inline comments lowercase and higher-level doc comments with a capital letter; do not end comments with periods or make them depend on conversation context. Document every public API in libraries.
- Prefer ReScript for new JavaScript-targeting application logic when practical. When TypeScript is required, use the Effect library for stronger type safety: model expected errors and dependencies explicitly, and validate external data with Effect Schema instead of unchecked type assertions.
- Do not leave deprecated code in place by default. Remove it, or ask whether the change must preserve the old path.
- For unfamiliar crates or external libraries, inspect documentation or source instead of guessing. For Rust, check `target/doc/`, run `cargo doc -p <crate-name>`, or inspect `~/.cargo/registry/src`; `btx` works for any repository.

# Git

- For commits, follow `$HOME/.agents/commit-message-guide.md` and use Praveen Perera when an author is needed. Never add Claude co-authors, generated-by notes, or `Claude-Session:` trailers.
- Preserve unrelated user or agent changes: stage by hunk and never undo unrelated edits.

# Delegation

- The root owns scope, design decisions, integration, and acceptance. It reads the code it will change and its delegates' conclusions; other reading goes to subagents.
- Delegate exploration to `Explore` subagents: reading or searching outside the files you will edit, other repositories, vendored or minified code, logs, and transcripts. Search directly only for a single lookup where you already know the file and symbol; once a second search looks likely, delegate.
- Conserve my weekly plan usage limits: name the model on every subagent so it does not inherit Opus, and never run one at `xhigh` or `max` effort. Default to Haiku 5.5, including for large or minified files; past 100K prompt tokens it costs 5x its base rate but stays about 4x cheaper than Sonnet 5.5. Haiku auto-compacts at 256K (`modelSettings` in `settings.json`), so split Haiku work into slices under 100K when the work divides naturally, and into slices under 256K when one run would otherwise compact. Use Sonnet 5.5 only when the work needs judgment Haiku lacks, not because the input is large.
- Choose the runner by how long the work runs, not by whether to delegate: use the Agent tool for work that finishes in this turn, including all exploration, and `homebased` for long or unattended work and for every non-Claude model. When unsure which runner fits, use `homebased`.
- Do not let a delegate wait inside a long build, test run, or CI watch. Its brief must tell it to submit the command through `homebased`, report `waiting`, and exit, as the `homebased` skill describes.
- For a larger task, run one fresh review of the related change set; add a second only when another risk area would bloat that prompt. Do not start one reviewer per package or file, and do not repeat completed checks.

# Testing

- Prefer integration tests over lots of small unit tests.
- Do not write tests for reversible, low-impact changes that only restate edited literals or mirror the implementation. Before keeping a test, ask whether it would still pass if every function it calls returned nothing; if it would, delete it.

# Rust

- For new projects, use edition 2024 and stable Rust; do not set an MSRV unless asked.
- Use the Rust 2018+ module layout instead of `mod.rs` for regular modules.
- Prefer `eyre`, or `color-eyre` for CLIs, over `anyhow`.
- Prefer turso + toasty ORM with compile-time checked queries over raw SQLite.
- When clippy reports autofixable issues, run `cargo fix --allow-dirty` only when the working tree and command scope make it safe from unrelated changes; otherwise apply the fixes manually. Fix remaining lints directly instead of silencing them with `allow` or `warn` unless there is a specific reason.
- Avoid redundant closures; use `.map(func)` instead of `.map(|value| func(value))`.
- Prefer tuple structs over named-field structs for simple wrappers, such as `struct Foo(Arc<Inner>)`.
- Prefer structs with methods over freestanding functions when they encapsulate shared state.
- Use named imports instead of wildcard imports.
- Add a blank line after a multi-line construct before the next statement or block, regardless of its closing syntax. Also use blank lines to separate distinct logical phases in a function. A related single-line statement can stay with the block that follows it when separation would add noise. Keep a short, single-phase body together, and do not add a blank line only before the final expression.
- Keep test-only functions, types, and modules out of production code paths. Put them under `mod tests` or a dedicated `mod test_support`, and use `#[cfg(test)]` only to gate those modules.
- On the Linux container `code`, keep Cargo build output off tmpfs. Do not set `CARGO_TARGET_DIR` to a `/tmp` path; use the configured disk-backed target directory. For isolated concurrent work, use a separate disk-backed target directory instead of cleaning a shared one while another build may run.

# Docker image builds

- On macOS, build images with `rb` unless the project uses another builder, such as `sht`. On `code`, `ai5090`, or another Linux box, build directly on the machine.
- Publish public images to Docker Hub.
