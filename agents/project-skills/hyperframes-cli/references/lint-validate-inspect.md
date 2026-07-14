# lint, check, visual diagnostics

Run `lint` for a fast static pass, then `check` for combined runtime, layout, motion, and WCAG verification. Use `snapshot`, `keyframes`, `compare`, and `grade-compare` for visual diagnostics. `validate`, `inspect`, and `layout` remain compatibility commands but are deprecated in favor of `check`.

## Contents

- [Discipline for motion-heavy work](#discipline-motion-heavy-work)
- [lint](#lint)
- [check](#check)
- [Compatibility commands](#compatibility-commands)
- [Motion verification](#motion-verification-motionjson-sidecar)
- [snapshot](#snapshot)
- [Sub-composition smoke test](#sub-composition-smoke-test)
- [keyframes](#keyframes)
- [beats](#beats)
- [compare and grade-compare](#compare-and-grade-compare)

## Discipline (motion-heavy work)

When the composition is animation-driven, run the checks before you reach for `preview` or `render`:

- Run `lint` after the first HTML pass — earlier, not later.
- Capture `snapshot` at meaningful timeline states; look at the PNGs.
- Inspect snapshots _before_ tuning automated warnings — your eye catches what the auditor misses.
- Treat layout warnings as defects unless a snapshot proves the overflow is intentional, in which case mark it with `data-layout-allow-overflow`.
- State motion intent in a `*.motion.json` sidecar so `check` verifies entrances firing under seek, stagger order, in-frame motion, and liveness.

## lint

```bash
npx hyperframes lint                  # current directory
npx hyperframes lint ./my-project     # specific project
npx hyperframes lint --verbose        # info-level findings
npx hyperframes lint --json           # machine-readable
```

Lints `index.html` and all files in `compositions/`. Reports errors (must fix), warnings (should fix), and info (with `--verbose`). Catches missing `data-composition-id`, overlapping tracks on the same `data-track-index`, and unregistered timelines.

**Blind spot — media inside a sub-composition.** A `<video>` or `<audio>` inside a `compositions/*.html` template can render blank because framework media belongs in the host composition. Check manually before render:

```bash
grep -nE '<(video|audio)\b' compositions/*.html   # expect NO matches; media belongs in index.html
```

A non-empty result is a defect. Then `snapshot` each scene that has a video and confirm the panel actually shows footage (a blank/black panel where a clip should play is a bug, not a placeholder — treat it as render-blocking).

## check

```bash
npx hyperframes check                         # combined verification
npx hyperframes check ./my-project --json     # specific project, agent-readable output
npx hyperframes check --at 1.5,4,7.25         # explicit hero-frame timestamps
npx hyperframes check --at-transitions        # sample tween boundaries
npx hyperframes check --strict                # exit non-zero on warnings
npx hyperframes check --snapshots             # retain contrast-pass PNGs
```

`check` runs static lint, runtime error detection, layout overflow checks, `*.motion.json` assertions, and WCAG AA contrast sampling in one browser session. Tune sampling with `--samples`, `--at`, or `--at-transitions`; tune overflow with `--tolerance`; use `--no-contrast` only during iteration.

Fix all errors before preview. Treat warnings as defects unless visual evidence proves they are intentional.

## Compatibility commands

### validate

```bash
npx hyperframes validate              # current directory
npx hyperframes validate ./my-project # specific project
npx hyperframes validate --json       # agent-readable findings
npx hyperframes validate --timeout 5000  # ms to wait for scripts (default 3000)
npx hyperframes validate --no-contrast   # skip WCAG contrast audit while iterating
```

`validate` is retained for compatibility. Its help marks it deprecated in favor of `check`. It loads the composition in headless Chrome and reports:

- JavaScript console errors and unhandled exceptions
- Failed network requests (media-file `ERR_ABORTED` filtered out)
- WCAG AA contrast violations on visible text — sampled at 5 timestamps across the timeline. Disable with `--no-contrast`.

**Fixing contrast warnings** — thresholds are 4.5:1 for normal text, 3:1 for large text (24px+, or 19px+ bold):

- On dark backgrounds, brighten the failing color until it clears the threshold; on light backgrounds, darken it.
- Stay within the palette family — don't invent a new color, adjust the existing one.
- Re-run `validate` until clean.

Prefer `check` for new workflows. Use `validate` only when a script or existing project explicitly requires its narrower output.

### inspect and layout

```bash
npx hyperframes inspect                 # inspect rendered layout over the timeline
npx hyperframes inspect ./my-project    # specific project
npx hyperframes inspect --json          # agent-readable findings (schemaVersion, samples, issues, bboxes)
npx hyperframes inspect --samples 15    # denser timeline sweep (default 9)
npx hyperframes inspect --at 1.5,4,7.25 # explicit hero-frame timestamps
npx hyperframes inspect --tolerance 4   # allowed overflow in px before reporting (default 2)
npx hyperframes inspect --strict        # exit non-zero on warnings too (default: only errors)
```

`inspect` and its `layout` alias are retained for compatibility; both help surfaces mark them deprecated in favor of `check`. They report:

- Text extending outside the nearest visual container or bubble
- Text clipped by its own fixed-width/fixed-height box
- Text extending outside the composition canvas
- Children escaping clipping containers

Errors should be fixed before rendering. Warnings are surfaced for agent review; add `--strict` to fail on warnings too. Repeated static issues are collapsed by default so JSON output stays compact for LLM context windows.

**Escape hatches:**

- `data-layout-allow-overflow` — mark an element or ancestor when overflow is intentional for an entrance/exit animation.
- `data-layout-ignore` — mark a decorative element that should never be audited.

Prefer `check` for new workflows.

### Motion verification (`*.motion.json` sidecar)

`check` and the compatibility `inspect` command check **motion intent** against the same seeked timeline the renderer uses. This catches an entrance reveal the seek lands past, a broken stagger order, an element drifting off-frame mid-tween, or a frozen shot.

Drop a `*.motion.json` sidecar next to the composition, matching the HTML basename when several compositions share a directory. Verification discovers it automatically.

```json
{
  "duration": 6,
  "assertions": [
    { "kind": "appearsBy", "selector": "#headline", "bySec": 0.5 },
    { "kind": "before", "a": "#headline", "b": "#cta" },
    { "kind": "staysInFrame", "selector": ".card" },
    { "kind": "keepsMoving", "withinSelector": ".scene" }
  ]
}
```

| Assertion                      | Fails (code) when                                                           |
| ------------------------------ | --------------------------------------------------------------------------- |
| `appearsBy(selector, bySec)`   | not visible (opacity ≥ 0.5) by `bySec` — `motion_appears_late`              |
| `before(a, b)`                 | `a` does not first appear strictly before `b` — `motion_out_of_order`       |
| `staysInFrame(selector)`       | once visible, its box leaves the canvas — `motion_off_frame`                |
| `keepsMoving(withinSelector?)` | a fully-static window exceeds `maxStaticSec` (default 2s) — `motion_frozen` |

`duration`, `withinSelector`, and `maxStaticSec` are optional. Findings are **errors by default** (a failed assertion fails the run, like a layout error — `--strict` still gates warnings) and appear in the same human and `--json` output as layout findings. A selector that matches nothing is reported as `motion_selector_missing` rather than silently passing — so a typo'd selector fails loudly. Use this in the feedback loop instead of eyeballing the render: assert what the motion is supposed to do, and let `inspect` tell you when the seek diverges from intent.

## snapshot

```bash
npx hyperframes snapshot                       # 5 key frames as PNG
npx hyperframes snapshot ./my-project          # specific project
npx hyperframes snapshot --frames 10           # evenly-spaced N frames
npx hyperframes snapshot --at 1.5,4,7.25       # exact timestamps
npx hyperframes snapshot --zoom '#hero'        # high-density crop of one selector
npx hyperframes snapshot --angle iso           # orthogonal 3D inspection
```

Captures still PNGs from the composition for visual diffing, thumbnails, or attaching to a PR. Faster than rendering a video when you only need a few hero frames. Output lands in the project's snapshots directory.

## Sub-composition smoke test

Static checks can miss cross-file mount failures. When `index.html` mounts files with `data-composition-src`, capture a frame near the midpoint of every mounted slot:

```bash
npx hyperframes snapshot --at <t1>,<t2>,<t3>,...
# or use a broad sample when exact scene times are unnecessary
npx hyperframes snapshot --frames 9
```

Inspect every frame against the scene plan. Tiny unstyled text or canvas-sized icons usually mean a sub-composition's styles were left outside its template. A missing hero or a timeline-registration timeout usually means the host composition ID does not match the template ID. Fix these before rendering.

## keyframes

```bash
npx hyperframes keyframes ./my-project --json
npx hyperframes keyframes ./my-project --selector '#hero' --runtime gsap
npx hyperframes keyframes ./my-project --selector '#hero' --shot out.png --samples 9
npx hyperframes keyframes ./my-project --selector 'canvas' --shot out.png --ghost
```

Use `keyframes` to inspect GSAP, CSS keyframes, Anime.js, and motion paths. `--shot` renders a seek-safe onion diagnostic; use `--layout strip` for overlapping or in-place motion, and `--angle` for 3D motion.

## beats

```bash
npx hyperframes beats ./my-project
npx hyperframes beats ./my-project --json
```

Detect beats in the project's music track and write `beats/<audio>.json`. Use the generated timing data as an input to composition choreography rather than re-detecting beats in authored code.

## compare and grade-compare

```bash
npx hyperframes compare variant-a variant-b --at 3 --labels a,b --out compare.png --json
npx hyperframes grade-compare --for frame.png --grades grades.json --out grade-compare.png --json
npx hyperframes grade-compare --for frame.png --luts warm.cube,cool.cube
```

`compare` renders each composition through its own runtime and creates one labeled sheet. `grade-compare` applies grading candidates or LUTs through the real grading runtime. Inspect the resulting sheet before choosing a variant.
