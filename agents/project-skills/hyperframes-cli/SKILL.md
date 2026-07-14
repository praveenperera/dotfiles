---
name: hyperframes-cli
description: Run and troubleshoot the HyperFrames CLI development and rendering loop. Use for init, add, catalog, capture, lint, check, validate, inspect, layout, beats, keyframes, snapshot, compare, grade-compare, preview, play, present, render, publish, feedback, cloud, lambda, cloudrun, doctor, browser, info, upgrade, skills, compositions, docs, benchmark, telemetry, auth, transcribe, tts, and remove-background commands.
---

# HyperFrames CLI

Run commands through `npx hyperframes` unless project instructions require a local wrapper. Require Node.js 22 or newer and FFmpeg for local rendering.

## Core workflow

1. Scaffold with `npx hyperframes init <dir>` or capture a URL with `npx hyperframes capture <url>`.
2. Author the composition using the `hyperframes-core` skill.
3. Run `npx hyperframes lint`, then `npx hyperframes check`. Use `snapshot` or `keyframes --shot` for visual verification.
4. Open `npx hyperframes preview` and let the user review or edit in Studio.
5. Render only after approval:
   - iterate: `npx hyperframes render --quality draft`
   - deliver: `npx hyperframes render --quality high --output out.mp4`
   - reproduce in CI: `npx hyperframes render --docker --strict --output out.mp4`
6. Verify the output exists and is non-empty. After a successful render, submit one `feedback` report unless telemetry is disabled or the user opted out.

`check` is the current combined lint, runtime, layout, motion, and WCAG verification command. `validate`, `inspect`, and `layout` remain compatibility commands but their help marks them deprecated in favor of `check`.

## Agent conventions

- Prefer `--json` when a command offers it. Server modes do not offer general JSON output; `render --json` applies only to batch progress.
- JSON output redacts home-directory paths to the literal `$HOME`, making it suitable for agent contexts and bug reports.
- In non-TTY mode, `init` requires `--example`. `--skip-skills` is temporarily ignored; set `HYPERFRAMES_SKIP_SKILLS=1` to opt out in CI or tests.
- Gate `doctor --json` on the payload's `ok` field because the command exits zero even when checks fail.
- Use `render --strict` to fail on lint errors, `--strict-all` to fail on warnings, and `--strict-variables` to reject undeclared or mistyped variable overrides.
- After render exit zero, verify `[ -s "$OUTPUT" ]`; for long outputs, also inspect duration with `ffprobe`.
- When the user refers to a selected Studio element, run `npx hyperframes preview --context --json --context-fields selection`. Use the returned `selection.target.hfId` or `selector` and `selection.sourceFile`; if the error code is `no-selection`, ask the user to select the element and retry.
- Keep Studio context compact with `--context-fields selection`, `lint`, or `selection,lint`. Use `--context-detail full` only when styles or editable text metadata are necessary.

## Command routing

| Task | Read |
| --- | --- |
| Scaffold or update skills (`init`, `capture`, `skills`) | [init-and-scaffold.md](references/init-and-scaffold.md) |
| Install registry items (`add`, `catalog`) | Use the `hyperframes-registry` skill |
| Verify correctness (`lint`, `check`, compatibility `validate` / `inspect` / `layout`, `snapshot`, `keyframes`, `beats`) | [lint-validate-inspect.md](references/lint-validate-inspect.md) |
| Preview, present, render, or publish (`preview`, `play`, `present`, `render`, `publish`, `feedback`) | [preview-render.md](references/preview-render.md) |
| Compare candidate frames or grades (`compare`, `grade-compare`) | [lint-validate-inspect.md](references/lint-validate-inspect.md) |
| Diagnose the environment (`doctor`, `browser`) | [doctor-browser.md](references/doctor-browser.md) |
| Choose or run hosted/distributed rendering (`cloud`, `lambda`, `cloudrun`) | [cloud-rendering.md](references/cloud-rendering.md) |
| Operate AWS Lambda in detail (`lambda deploy`, `sites`, `render`, `progress`, `destroy`, `policies`) | [lambda.md](references/lambda.md) |
| Use project/account/tooling commands (`info`, `upgrade`, `compositions`, `docs`, `benchmark`, `auth`, `telemetry`) | [upgrade-info-misc.md](references/upgrade-info-misc.md) |
| Preprocess assets (`tts`, `transcribe`, `remove-background`) | Use the `media-use` skill; see [upgrade-info-misc.md](references/upgrade-info-misc.md) for CLI entry points |

Run `npx hyperframes <command> --help` before using uncommon or release-sensitive flags.

## Cross-skill hand-offs

- For Tailwind projects created with `init --tailwind`, use the `hyperframes-core` Tailwind reference before editing classes or theme tokens.
- For registry blocks and components, use `hyperframes-registry` for install paths and wiring.
- For voice, captions, transcription, or background removal, use `media-use`.
- For `--variables`, declare the schema with `data-composition-variables`; use `hyperframes-core` for the domain model.
