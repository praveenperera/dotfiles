# Opus 5 prompting guide

How to write Opus 5 prompts for this workflow. Source: Thariq Shihipar's [context engineering post for Claude 5 generation models](https://claude.com/blog/the-new-rules-of-context-engineering-for-claude-5-generation-models) (also [on X](https://x.com/trq212/status/2080710971228918066)), adapted for delegated Agent-tool runs rather than a standing Claude Code system prompt.

Opus 5 is a Claude 5 generation model. Anthropic found they could remove over 80% of Claude Code's system prompt for Opus 5 and Fable 5 with no measurable coding-eval loss. The old pattern of rules, examples, and repetition now hurts more than it helps. Prefer short, high-signal prompts and rich references over long instruction dumps.

This guide does not replace the shared prompt contract in [codex-cli.md](codex-cli.md). It changes how you write the body of that contract when the delegate is Opus 5.

## Core shift

| Older prompting habit | Opus 5 default |
| --- | --- |
| Hard rules for every edge case | Judgment plus local context |
| Few-shot examples of good behavior | Expressive interfaces and clear parameters |
| Dump every practice into the prompt | Progressive disclosure: load detail when needed |
| Repeat the same instruction in several places | One clear home for each instruction |
| Prose-only specs and plans | Rich references: code, tests, mockups, rubrics |

Unhobble style. Keep authority, scope, and verification hard.

Anthropic's lesson is that stylistic constraints ("never write comments", "always do X") force the model to reconcile conflicts and often produce worse code. That is different from this workflow's authority boundaries. Opus 5 still gets exact owned scope, excluded scope, no-commit rules, stop conditions, and an observable success condition. Do not soft-pedal those.

## Prompt shape for Opus 5

Keep the same seven sections as the shared contract, but write them for a model that has good judgment and weak tolerance for instruction bulk:

1. **Objective and success condition** — one concrete outcome and how you will know it is finished. Prefer checkable states over process prose.
2. **Authority and owned scope** — exact files, directories, or responsibilities. Hard boundary.
3. **Excluded scope** — what must not change. Hard boundary.
4. **Evidence and references** — files, diffs, tests, mockups, or rubrics to inspect. Prefer `@` paths and code over re-explaining the repo.
5. **Constraints** — only non-negotiables that judgment cannot recover from context. Preserve user requirements verbatim.
6. **Verification** — specific commands or read-only checks. Prefer "run X and report the exact outcome" over a general "make sure it works".
7. **Stop conditions and final report** — when to stop short, and what the report must contain.

Omit sections that are empty. Do not pad with policy that AGENTS.md or the surrounding code already make obvious.

### Length budget

- Target a short self-contained prompt, not a skill dump.
- Put durable team opinion in progressive references, not in the task body.
- If the prompt grows past roughly one screen of instruction, split: keep the task contract short, and point at one or two reference files for detail.
- Do not paste large skill files, CLAUDE.md dumps, or multi-page style guides into the Opus 5 prompt. Point at the path and the relevant section instead.

Launch-era reports say Opus 5 follows large instruction-dense skill files less reliably than short focused ones. Treat prompt bulk as a reliability risk, not a safety net.

## Then and now, applied to delegation

### Rules → judgment

Do not write:

```text
Default to writing no comments. Never write multi-paragraph docstrings.
Always add tests for every changed function.
Never refactor anything outside the bugfix.
```

Write:

```text
Match the surrounding code's comment density, naming, and idiom.
Make the smallest coherent change that satisfies the success condition.
Preserve established abstractions. If the fix needs a broader redesign, stop and report rather than expanding scope.
```

Style belongs to the codebase. Hard rules that are sometimes wrong create conflict with the user's actual request. Keep absolute language for authority and safety only: owned scope, no commits, no external writes, listed stop conditions.

### Examples → interfaces

Do not fill the prompt with few-shot transcripts of how a previous agent solved a similar task. Those constrain exploration to the example's shape.

Instead, design the task interface:

- Mode is an enum: `read-only analysis` or `implementation`.
- Owned scope is a precise list of paths or a single responsibility.
- Success condition is observable.
- Final report fields are fixed: result vs success condition, changed files, verification outcomes, blockers, residual risks.

The shared prompt contract already does this. For Opus 5, resist the urge to add "for example, you might..." sections that restate the contract in narrative form.

### All upfront → progressive disclosure

Do not front-load verification recipes, review rubrics, migration checklists, and product background into every Opus 5 prompt.

Do:

- Put the task contract in the prompt body.
- Point at references the agent should load only if needed:
  - `Read AGENTS.md at the repo root and any AGENTS.md under owned paths before editing.`
  - `For API shape taste, use the rubric in _scratch/.../api-rubric.md after the implementation draft exists.`
  - `The reference behavior is the test suite in tests/billing/; treat failing cases there as the spec.`
- Prefer a tree of small files over one encyclopedic task prompt.

If verification is unique and always required for this task, keep the exact commands in the Verification section. Move reusable multi-step verification playbooks into a skill or reference file and link them.

### Repetition → one home

State each requirement once. Do not restate "do not commit" in Constraints, Operating rules, and the closing paragraph. The shared operating rules already cover commits, subagents, and external state.

If a requirement is important enough to emphasize, make it sharper in its one home rather than repeating it:

```text
## Success condition

All items below are true, or the run is not done:
- `just test` exits 0
- no files outside `src/billing/` are modified
- the final report lists the exact commands run and their exit codes
```

### Simple specs → rich references

Prefer high-fidelity references over prose restatements:

| Prefer | Over |
| --- | --- |
| A failing test file or golden fixture | A paragraph describing expected behavior |
| An existing function or module to port | A bullet list of desired semantics |
| An HTML mockup or real UI component | A screenshot description of the layout |
| A short taste rubric for a verifier pass | "Make it clean and idiomatic" |
| The diff or issue text that defines the bug | A second-hand summary of the bug |

Opus 5 handles complicated references well. Give it the artifact in a language it already executes against: code, tests, types, and structured rubrics.

## Completion and instruction-following riders

Unhobbling does not remove the known Opus 5 failure modes from [model-routing.md](model-routing.md): early stopping, reporting unfinished work as done, and arguing with explicit requirements. Add a short rider when Opus 5 implements. Keep it brief; do not bury it in a wall of policy.

```text
## Completion

Finish the entire objective before reporting. Do not stop at a partial result, and do not report success while any part of the success condition is unmet. Explicit requirements take precedence over your preferred approach; if a requirement seems wrong, satisfy it anyway and note the disagreement in the final report. Stop early only for a listed stop condition, and name which one.
```

When integrating an Opus 5 run:

- Treat "done" as unverified.
- Check the success condition and the diff before accepting a short run.
- Re-prompt to continue rather than integrating partial work.
- Prefer a fresh prompt for repair when the first run drifted, rather than piling corrections onto a confused thread.

## What to put in the prompt vs leave out

### Put in

- User requirements, quoted or preserved closely
- Owned and excluded scope
- Observable success condition
- Exact verification commands for this task
- Paths to the few references that define correctness
- The short completion rider for implementation runs
- Stop conditions that protect the repo and the user

### Leave out

- Generic "write clean code" advice the model already knows
- Comment, docstring, and formatting micro-rules that the surrounding code already demonstrates
- Few-shot examples of tool traces or prior agent sessions
- Full copies of skills, CLAUDE.md, or style guides
- Repeated restatements of the same constraint
- Speculative edge-case policy for situations the task does not touch
- Process narration ("first you should think carefully, then explore, then...")

If the codebase has a real gotcha (types live only in one file, a broken test is skipped on purpose, a migration must be expand-only), put that gotcha in. Gotchas beat generalities.

## Worked prompt skeleton

```markdown
# Delegated task

Mode: implementation

## Objective

<one concrete outcome>

## Authority

Edit only the owned scope below.

## Owned scope

- `path/to/...`

## Excluded scope

- everything else
- commits, pushes, PRs, deploys, and external state

## Evidence

- <bug report, diff, or issue>
- <path to reference implementation, test, mockup, or rubric>

## Constraints

- <verbatim user requirements only>
- Match surrounding code style; smallest coherent change.

## Verification

- <exact commands>
- Report exact exit codes and relevant output.

## Success condition

All of the following are true:

- <checkable condition 1>
- <checkable condition 2>

## Stop conditions

- missing access, ownership conflict, or needed change outside owned scope
- architecture mismatch that requires a broader redesign

## Completion

Finish the entire objective before reporting. Do not stop at a partial result, and do not report success while any part of the success condition is unmet. Explicit requirements take precedence over your preferred approach; if a requirement seems wrong, satisfy it anyway and note the disagreement in the final report. Stop early only for a listed stop condition, and name which one.

## Operating rules

- Read applicable AGENTS.md files before acting.
- Inspect relevant repository context before editing.
- Do all work yourself. Do not spawn subagents or nested agents.
- Preserve concurrent and unrelated changes.
- Do not commit, stage, push, open or modify PRs, deploy, or change external state.

## Final report

- Result against the success condition
- Changed files, or none
- Verification commands and exact outcomes
- Blockers, residual risks, and scope conflicts
```

For read-only analysis, drop the completion rider's implementation tone, keep the success condition observational, and use a read-only sandbox.

## Orchestrator checklist before sending

1. Is the prompt short enough that every sentence earns its place?
2. Are hard boundaries only on authority, scope, safety, and verification?
3. Did you remove stylistic rules that the codebase can teach by example?
4. Are references code, tests, mockups, or rubrics rather than long prose restatements?
5. Is each requirement stated once?
6. For implementation: is the completion rider present and the success condition checkable without trusting the model?
7. Would a progressive reference file be better than another paragraph in the prompt?

## Fable subagent prompts (Opus root)

When Opus is root and Fable is the taste/review/simplification delegate, use the same short contract shape as this guide: hard authority, scope, and verification; soft style; progressive references. Fable does not need the Opus completion rider's "do not argue with requirements" tone by default, but it does need an explicit completion bar against laziness and intent drift:

```text
## Completion

Cover the full owned scope before reporting. Do not stop after a partial pass. Prefer the smallest coherent simplification or review that meets the success condition. Explicit user requirements take precedence over inferred product intent; if they conflict, satisfy the explicit requirement and note the tension in the final report.
```

Keep Fable subagent prompts short. Point at the diff, owned paths, and success condition. Do not paste this skill or a multi-page rubric into the prompt body.

## Sources

- [Thariq Shihipar on X](https://x.com/trq212/status/2080710971228918066) — original thread
- [The new rules of context engineering for Claude 5 generation models](https://claude.com/blog/the-new-rules-of-context-engineering-for-claude-5-generation-models) — full Anthropic post
- [A field guide to Claude Fable](https://claude.com/blog/a-field-guide-to-claude-fable-finding-your-unknowns) — related prompting guide for advanced Claude models
- [model-routing.md](model-routing.md) — when to choose Opus 5 and its known failure modes
- [codex-cli.md](codex-cli.md) — shared delegation contract and artifact capture
