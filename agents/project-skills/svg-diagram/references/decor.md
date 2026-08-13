# Decor

Arrows, elbows, curves, axes, and bars are a second pass. Give a node `.id("slot")` to reserve its rect, then draw into it:

```rust
Fig::tree(root).decor(|rects, painter| {
    painter.connector(
        &Connector::arrow(rects.get("from"), rects.get("to")).color(ArrowColor::Muted),
    );
})
```

A missing id fails the render with the list of published ids. A duplicate id fails the render.

Card-stack figures are usually a pure `Fig::tree`. Geometric figures often build an unpainted skeleton of sized `.id()` nodes and paint in `.decor()`.

Copyable connector figure: [examples.md](examples.md).

## Connectors

`engine/src/connect.rs`. Constructors: `Connector::arrow`, `::elbow`, `::curve`.

| Method | Role |
|---|---|
| `.color(ArrowColor)` | required for a headed connector |
| `.from_side(Side)` / `.to_side(Side)` | `Top` `Right` `Bottom` `Left`; auto-picked when omitted |
| `.route(Route::Hv \| Vh)` | elbow leg order |
| `.bend(n)` | curve control-point offset (default 60) |
| `.label("…")` / `.label_size(n)` / `.label_bg(color)` | halo label |
| `.stroke_width(n)` | default 4 |
| `.clearance(n)` | gap before a rect target |
| `.headless()` | plain line, no arrowhead |

`Side` and `Point` live in the same module. `Anchor` is a rect or a literal point.

## Charts

`engine/src/chart.rs`. d3-compatible `LinearScale` and `BandScale`, `plot_area`, bottom axis, bars.

```rust
painter.axis_bottom(&axis);
painter.bars(&bars);
```

There is no left axis, gridline, line series, or area helper yet.

## Painter marks

For geometry that is not a connector or a bar (`engine/src/paint.rs`):

| Method | Role |
|---|---|
| `painter.node(rect, &NodeSpec)` | labeled box |
| `painter.text_block(rect, &TextBlockSpec)` | wrapped copy in a rect |
| `painter.badge(rect, &BadgeSpec)` | pill with a centered label |
| `painter.headline` / `subline` / `heading` | chrome lines |
| `painter.rect_raw(rect, style)` | raw rect |
| `painter.report(label, extent)` | feed a mark into the overflow gate |
| `painter.raw_bounded(label, rect, markup)` | escape hatch for markup the engine does not model |

Every mark must report an extent. Unreported drawing is how a figure ships a clipped image.
