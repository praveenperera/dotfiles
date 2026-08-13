---
name: svg-diagram
description: Generate themed SVG diagrams, OG images, and post figures in ~/code/diagrams. Use when creating or changing programmatic diagrams, social cards, or blog SVGs.
---

# SVG diagrams

Do not invent another maker and do not hand-write post SVGs. Use `~/code/diagrams`.

The TypeScript/Satori pipeline is gone. There is no `bun`, no `.tsx`, and no `build.ts`.

CLI, DSL, and checks: `~/code/diagrams/README.md`.

## Overflow (read this first)

The usual failure is copy that leaves a card or the canvas. The engine wraps text only when the box has a width.

In `~/code/diagrams`:

- Put copy in `txt(...)` or `kicker(...)` / `body_text(...)`. Do not drop a long string into a row of unsized leaves.
- Use `card` for panels. Use `row` of `.grow(1.0)` children when several cards sit side by side.
- Keep a kicker to a few words. Put the long sentence on the next line.
- Do not pin `.w()` / `.h()` unless the figure needs a fixed box. Auto-size is the default.
- After `just build`, a canvas overflow error means shorten the copy or shrink the type. Do not turn the check off.

## Build

```sh
cd ~/code/diagrams
just build
```

That renders every registered figure and writes `{name}-light.svg` / `{name}-dark.svg` (or `.png`) into the directories the figure registers. A figure whose content leaves its canvas fails the build.

```sh
cargo run --release --manifest-path engine/Cargo.toml -- --list
cargo run --release --manifest-path engine/Cargo.toml -- --only my-figure --out _scratch
cargo run --release --manifest-path engine/Cargo.toml -- --check
```

`--theme light|dark` builds one theme. `--png` also rasterizes SVG-only figures. `--out <dir>` writes everything to one directory instead of the published ones.

## Add or change a figure

A figure is a `FigureDef` plus a `build` function that returns a `Fig`.

1. Add a module under `engine/src/figures/<wave>/`. Copy the `FigureDef` skeleton from the README, or start from a neighbor:
   - card stacks: `engine/src/figures/satori_wave/confirmed_inferred_open.rs`
   - geometric flow: `engine/src/figures/flow_a/rng_routing.rs`
   - article / OG: `engine/src/figures/og_wave/`
2. Register `DEF` in that wave's `DEFS` slice. `figures::all()` walks the waves.
3. Build one figure to `_scratch` until it is clean, then `just build`.
4. Embed the pair in the post. Do not edit the generated SVG by hand.

`name` is the file stem. `title` and `desc` become the SVG `<title>` and `<desc>`. `height: None` lets the canvas take the height the content needs.

Post figures are usually `1500×720` and `Format::Svg` in both themes. Social OG cards are `1200×630`, dark-only, `Format::Png`.

## Authoring

Trees of `El` (`engine/src/el.rs`). Building a tree does not measure or layout.

- `row(children)` / `col(children)` — flex; `.gap()`, `.pad()` / `.px()` / `.py()`, `.items()`, `.justify()`, `.grow()`
- `txt("…")` — wrapping text; `.size()`, `.bold()`, `.semibold()`, `.color()`, `.mono()`, `.italic()`, `.tracking()`, `.center()`
- Box style — `.bg()`, `.border(width, color)`, `.rounded()`, `.w()` / `.h()`, `.min_w()`
- `.absolute()` with `.top()` / `.right()` / `.bottom()` / `.left()` for the rare overlay

House style (`engine/src/components.rs`): `shell(p, title, sub, content)`, `card(p, variant, children)`, `kicker(p, text)`, `body_text(p, text)`.

Arrows, elbows, curves, axes, and bars are a second pass. Give a node `.id("slot")`, then:

```rust
Fig::tree(root).decor(|rects, painter| {
    painter.connector(&Connector::arrow(rects.get("from"), rects.get("to")).color(ArrowColor::Muted));
})
```

`engine/src/connect.rs` and `engine/src/chart.rs`.

## Theming

`build` receives `&Palette` for the theme being rendered (`engine/src/color.rs`). Take colors from it: `p.bg`, `p.card`, `p.stroke`, `p.text`, `p.muted`, `p.kicker`, and the `ok` / `warn` / `bad` / `info` / `blue` families. Never inline a hex. `Variant` names those families for `card()`. `p.og` is the separate scheme for OG and article figures.

## Output sets

A figure names an `OutputSet`, not a path. Paths live on `OutputSet` in `engine/src/figures.rs`.

| Set | Use |
|---|---|
| `Coldcard` | coldcard post figures (`blog-posts` and `static_sites` `images/posts`) |
| `Og` | main-site article figures and OG cards |
| `Bip110` | BIP-110 site images |
| `Messianic` | prophecies-of-jesus OG cards |
| `Scratch` | unpublished figures, repo `_scratch/` |

## Fonts

Inter (400, italic, 500, 600, 700) and JetBrains Mono (500) are vendored in `engine/fonts/`. Stick to characters those faces contain. Symbols such as `⊕` and `≈` become missing glyphs.

## Embed in a post

From a year folder such as `2026/`:

```html
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/posts/my-diagram-dark.svg">
  <img src="../images/posts/my-diagram-light.svg" alt="Short factual description">
</picture>
```

## Checks

```sh
just check   # fmt-check, clippy, test
```

`engine/tests/metrics_parity.rs` holds text metrics against a frozen corpus from the old pipeline. That corpus cannot be regenerated.
