# Figures

A figure is a `FigureDef` plus a `build` function that returns a `Fig`. `name` is the file stem. `title` and `desc` become the SVG `<title>` and `<desc>`. `height: None` lets the canvas take the height the content needs.

## Add or change

1. Add a module under `engine/src/figures/<wave>/`, or edit the existing one.
2. Register `DEF` in that wave's `DEFS` slice. `figures::all()` walks the waves. Nothing else needs touching.
3. Build one figure to `_scratch` until it is clean. Then `just build`.
4. Embed the pair in the post. Do not edit the generated SVG by hand.

Copy a worked figure from [examples.md](examples.md), or start from a neighbor:

| Family | Wave | Neighbor |
|---|---|---|
| card stacks | `satori_wave` | `satori_wave/confirmed_inferred_open.rs` |
| geometric flow | `flow_a`, `flow_b` | `flow_a/rng_routing.rs` |
| article / OG | `og_wave` | `og_wave/` |
| unpublished smoke | `tree_smoke` | `tree_smoke.rs` (in `ALL`, writes `Scratch`) |

A new family gets its own wave module. Extend `figures::all()` with that slice.

`HIDDEN` holds figures that exist but are not in the default build. `gate-smoke` lives there.

## Sizes and formats

| Kind | Size | Themes | Format |
|---|---|---|---|
| post figure | usually `1500×720` | both | `Format::Svg` |
| shorter post | existing ones use 380, 400, 580, 620 | both | `Format::Svg` |
| social OG card | `1200×630` | dark only | `Format::Png` |

`BOTH_THEMES` is `&[Theme::Light, Theme::Dark]`. Social cards use a dark-only slice; they have no light pair.

PNG-only figures skip the font-substitution pad: resvg rasterizes them with the vendored faces. A figure that also ships SVG keeps the pad.

## Output

Pick the `OutputSet` in [publish.md](publish.md). Paths live on `OutputSet` in `engine/src/figures.rs`.
