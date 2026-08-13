# Examples

Copy these, then change the name, copy, and `OutputSet`. Build with `--only` to `_scratch` until the gate is clean.

Shipped figures to read in full:

| Kind | File |
|---|---|
| card stack | `engine/src/figures/satori_wave/confirmed_inferred_open.rs` |
| card stack with helpers | `engine/src/figures/satori_wave/pad_xor_collapse.rs` |
| connectors | `engine/src/figures/flow_a/rng_routing.rs` |
| chart | `engine/src/figures/flow_b/uid_pad_range.rs` |
| auto height | `engine/src/figures/tree_smoke.rs` |
| dark PNG card | `engine/src/figures/og_wave/bip110_og.rs` |

## New figure

`engine/src/figures/satori_wave/my_figure.rs`:

```rust
use crate::color::{Palette, Variant};
use crate::components::{body_text, card, kicker, shell};
use crate::el::{col, row};
use crate::fig::Fig;
use crate::figures::{BOTH_THEMES, FigureDef, Format, OutputSet};

const TITLE: &str = "My figure";
const SUB: &str = "One sentence saying what the reader is looking at.";

pub const DEF: FigureDef = FigureDef {
    name: "my-figure",
    width: 1500.0,
    height: Some(720.0),
    title: TITLE,
    desc: SUB,
    themes: BOTH_THEMES,
    formats: &[Format::Svg],
    out: OutputSet::Scratch,
    build,
};

fn build(p: &Palette) -> Fig {
    let body = row([
        card(p, Variant::Card, [kicker(p, "First"), body_text(p, "Some copy")]),
        card(p, Variant::Ok, [kicker(p, "Second"), body_text(p, "More copy")]),
    ])
    .gap(24.0);

    Fig::tree(shell(p, TITLE, SUB, col([body])))
}
```

Register it in that wave's `DEFS`:

```rust
// engine/src/figures/satori_wave.rs
mod my_figure;

pub(super) const DEFS: &[FigureDef] = &[
    // …
    my_figure::DEF,
];
```

```sh
cargo run --release --manifest-path engine/Cargo.toml -- --only my-figure --out _scratch
```

That writes `_scratch/my-figure-light.svg` and `_scratch/my-figure-dark.svg`.

## Card stack from a table

Needs `use crate::el::{Align, row};` on top of the new-figure imports.

```rust
struct Column {
    title: &'static str,
    variant: Variant,
    items: [&'static str; 3],
}

const COLUMNS: [Column; 3] = [
    Column {
        title: "CONFIRMED",
        variant: Variant::Ok,
        items: ["First fact", "Second fact", "Third fact"],
    },
    Column {
        title: "INFERRED",
        variant: Variant::Warn,
        items: ["First inference", "Second inference", "Third inference"],
    },
    Column {
        title: "OPEN",
        variant: Variant::Info,
        items: ["First question", "Second question", "Third question"],
    },
];

fn build(p: &Palette) -> Fig {
    let columns = COLUMNS.iter().map(|column| {
        card(
            p,
            column.variant,
            [kicker(p, column.title).color(p.variant_fg(column.variant))],
        )
        .children(column.items.iter().map(|item| {
            row([body_text(p, *item)])
                .bg(p.card)
                .border(1.0, p.stroke)
                .rounded(12.0)
                .px(16.0)
                .mt(12.0)
                .min_h(68.0)
                .items(Align::Center)
        }))
        .grow(1.0)
    });

    Fig::tree(shell(p, TITLE, SUB, row(columns).gap(16.0).grow(1.0)))
}
```

Full figure: `engine/src/figures/satori_wave/confirmed_inferred_open.rs`.

## Connectors

Reserve rects with `.id()`, then draw in `.decor()`:

```rust
fn build(p: &Palette) -> Fig {
    let root = row([
        card(p, Variant::Ok, [kicker(p, "Source")]).id("from").grow(1.0),
        card(p, Variant::Bad, [kicker(p, "Sink")]).id("to").grow(1.0),
    ])
    .gap(80.0);

    Fig::tree(shell(p, TITLE, SUB, root)).decor(|rects, painter| {
        painter.connector(
            &Connector::arrow(rects.get("from"), rects.get("to"))
                .color(ArrowColor::Muted)
                .label("routes to"),
        );
    })
}
```

Needs `use crate::color::ArrowColor;` and `use crate::connect::Connector;`.

Full figure: `engine/src/figures/flow_a/rng_routing.rs`.

## Embed

From a year folder such as `2026/`. Full rules: [publish.md](publish.md).

```html
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/posts/my-figure-dark.svg">
  <img src="../images/posts/my-figure-light.svg" alt="Short factual description">
</picture>
```
