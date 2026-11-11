<!-- Generated from agents/instructions by `cmd instructions render`; edit those fragments, not this file -->

# Workflow

- Ship production-quality changes. For changes to domain state, public interfaces, or ownership, model the domain first and use typed models to exclude invalid states. Prefer the proper owner or abstraction over caller-specific conditionals. Small local changes should follow established patterns without a separate architecture exercise. Repeated fixes in one area signal that the model may be wrong; revisit it and remove resulting shortcuts before finishing.
- Encode recurring corrections as types, tests, lints, scripts, or runtime checks instead of repeating instructions.
- When a step doesn't need my input, keep going, and put status notes in the same message as your next action. Stop and ask only when you can't continue without me.
- For long multi-step runs, keep a checklist in `_scratch/<task-name>/TASKS.md` so concurrent runs do not share one file; tick each item when it's done and add anything new you find. At the end, list what needs my input first, then the summary.
- Put ad hoc files I may want to inspect, such as Markdown, HTML, screenshots, and image-generation outputs, in a repo-root `_scratch/` directory, creating it if needed. Those directories are wiped weekly, so put durable findings and decisions in `~/code/research`, and tools, scripts, automation prompts, runbooks and project ops notes in `~/code/private-workbench`.
- Keep private to-dos in `~/code/private-workbench/TODO.md`, and track work that can be public in that repo's GitHub issues. Put private work with no deadline in `~/code/private-workbench/TOKEN-BURN.md`, to pick up when my weekly usage is about to expire.
- Mark anything you couldn't confirm, and say where you looked.
- When you change behavior, update the docs and READMEs that describe it in the same change, so they never describe the old behavior.
- My instructions take precedence over a skill's guidelines. When a skill rule blocks progress, cite the exact `SKILL.md` file and rule instead of stopping.
- For performance work, profile and measure the end-to-end payoff first (same output or accuracy within limits, plus a measured speedup on a fixed input set); add harness or proof machinery only for risks that check can't catch, and time-box it.
- Before adding complexity for a speculative or vanishingly unlikely race or security edge case, explain the concrete failure mode, likelihood, and complexity cost to me and get my buy-in. Do not silently expand scope for theoretical robustness.
- Always give times in the US Central time zone.
- Never give local-only 127.0.0.1 links; use host mode so links work on LAN and Tailscale.
- In public-facing copy, include only reader-visible content. Omit implementation notes, source labels, workflow state, reasoning, conversation context, and edit instructions.
- Run all GPU work through the homebased queue; never run work that needs GPU time directly.

# Code

- The code explains what; comments explain why. Comment non-obvious decisions, constraints, and tradeoffs. Start inline comments lowercase and higher-level doc comments with a capital letter; do not end comments with periods or make them depend on conversation context. Document every public API in libraries.
- Minimize nesting in functions.
- Prefer ReScript for new JavaScript-targeting application logic when practical. When TypeScript is required, use the Effect library for stronger type safety: model expected errors and dependencies explicitly, and validate external data with Effect Schema instead of unchecked type assertions.
- Do not leave deprecated code in place by default. Remove it, or ask whether the change must preserve the old path.
- For unfamiliar crates or external libraries, inspect documentation or source instead of guessing. For Rust, check `target/doc/`, run `cargo doc -p <crate-name>`, or inspect `~/.cargo/registry/src`; `btx` works for any repository.

# Git

- New repos should always use `master` as the default branch name, never `main`.
- For commits, follow `$HOME/.agents/commit-message-guide.md` and use Praveen Perera when an author is needed. Never add Claude, Codex, or other AI co-authors, generated-by notes, or `Claude-Session:` trailers.
- Preserve unrelated user or agent changes: stage by hunk and never undo unrelated edits.

# Delegation

- The root owns scope, design decisions, integration, and acceptance. It reads the code it will change and its nearby context (callers, types, tests), delegate reports and diffs, command output, and the skill and guide files it follows; it delegates broader reading.
- Spend my weekly plan usage where it buys correctness: a cheaper delegate saves nothing if the root has to redo or second-guess its work. Pick each subagent's model for what that task needs.
- Run commands you wait on longer than a few minutes, such as builds, test suites, deploys, and Fleet or GPU work, as homebased tasks, not under a polling loop or a raw `Monitor` in the turn, since every line a Monitor prints wakes the thread and re-reads its whole context.
- Always wait on CI, PR reviews, and release runs through a homebased task, however short, never with a polling loop or `gh run watch` in the turn. For a PR, submit `cmd ship watch-pr <n>` instead of T3's `watch_pull_request`, which wakes the thread for every bot comment.
- A delegate never waits in the foreground for a long command, and the root never waits on a delegate's jobs. A T3 `delegate_task` child on a Claude model submits through `homebased`, keeps a background `homebased wait --next --final-only` registered with `t3_background_wait`, and reports only its final result. A homebased worker reports `waiting` and exits. The `use-agents` skill has the details.
- When this thread coordinates several delegates, PRs, or queued jobs over a long run, use the `orchestrate` skill.
- For a larger task, run one fresh review of the related change set; add a second only when another risk area would bloat that prompt. Do not start one reviewer per package or file, and do not repeat completed checks.
- Use Grok 4.5 at high effort only to read X posts; never ask it to analyze or do other work.
- As the root, delegate exploration to `Explore` subagents: sweeps across many files, other repositories, dependency source, vendored or minified code, logs, and transcripts. Make a few targeted searches or reads in the repository you are changing yourself; delegate once a lookup would pull more than a few files into your context or leaves that repository. Ask delegates to cite `path:line` for each finding. Subagents do their own reading instead of delegating again.
- Model facts (`modelSettings` in `settings.json`): an Agent call without `model` inherits the session's model (`model` sets Fable as the default), and Opus 5.5 and Sonnet 5.5 both default to `high` effort. Every subagent first reads its system prompt and these instructions. Haiku 5.5 costs 5x once its prompt, which includes everything it has read, passes 100K tokens, still about 4x cheaper than Sonnet 5.5, and auto-compacts at 256K, which loses detail. Sonnet 5.5 cache reads cost the same as Opus 5.5, so in long agent loops, where re-reading context dominates cost, switching to Sonnet saves well under half.
- Pick the runner by duration. Use the Agent tool for work that finishes in this turn, which covers most exploration, and `homebased` for long or unattended agent work. In a T3 thread, use T3's `delegate_task` for that agent work instead, so the work shows under the thread and the orchestrator's compact watches can reach it. Keep `homebased` there for commands, Fleet machines, the GPU queue, and `after` chains.
- Set `model` on every Agent call; a hook denies calls that leave it out, and the brief then has to be sent again.

# Verification

- After implementation changes, run the repository's formatter and linter. For Rust, run `just fmt` and `just clippy`; fall back to `cargo fmt` and `cargo clippy` when no justfile exists.
- Run the checks appropriate to the change. Reuse reliable results for unchanged code; repeat or broaden checks only for new changes, failures, missing evidence, or unresolved concerns.

# Testing

- Prefer integration tests over lots of small unit tests.
- Add or update tests when they protect user-visible behavior, reproduce a bug, cover compatibility or migration risk, or lock down a non-obvious invariant.
- Do not write tests for reversible, low-impact changes that only restate edited literals or mirror the implementation.
- Before keeping a test, ask whether it would still pass if every function it calls returned nothing. If it would, rewrite it to assert a concrete result for a concrete input, or delete it.
- For static configuration or list changes, prefer compile or lint verification unless selection, fallback, parsing, migration, or filtering behavior needs coverage.

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
- `info` and `error` logs may start with uppercase letters.
- In log and `println!` macros, prefer inline variable capture such as `warn!("person id={id} ...")` over positional placeholders.
- On the Linux container `code`, keep Cargo build output off tmpfs. Do not set `CARGO_TARGET_DIR` to a `/tmp` path; use the configured disk-backed target directory. For isolated concurrent work, use a separate disk-backed target directory instead of cleaning a shared one while another build may run.

# Docker image builds

- To build or push a container image, follow the `rb` skill, which says where to build from each machine and where to publish. When a project uses another builder, such as `sht`, use that instead.

# Tools

- `homebased t3` can add projects and start new threads on different Fleet machines; the `fleet` skill describes them.
- MCP servers available through `codemode-mcp`: `linear`, `gmail`, `zoho_infraops`, `calendar`, `reminders`, `xcode`, `secrets`, `onepassword`, `logins`, `superalert`, `superlogical`, `deepwiki`, `context7`, `cloudflare_docs`, `openai_docs`
