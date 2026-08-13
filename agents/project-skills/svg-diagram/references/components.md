# House style

Use the three functions in `engine/src/components.rs` rather than re-deciding the look per figure.

## `shell(p, title, sub, content)`

Full-canvas frame: title, subtitle, then the figure's content.

- Title is 34px bold `p.text`. Subtitle is 18px `p.muted`, 8px below the title.
- Content sits 24px below the subtitle and grows to fill the remaining height.
- Padding is 40px horizontal, 32px vertical.
- Backdrop is `p.shell_bg`, which is not the same token as `p.bg` in light mode.

If `content` has no grow of its own, `shell` gives it `.grow(1.0)`.

## `card(p, variant, children)`

Rounded panel in a variant's colors. A column.

- Radius 16, border 2, padding 20×16.
- Fill and border come from `p.variant_colors(variant)`.

## `kicker(p, text)`

Small tracked label. Returns the text node itself, so a figure can recolor it:

```rust
kicker(p, "CONFIRMED").color(p.variant_fg(variant))
```

14px, bold, tracking 0.4, default color `p.kicker`.

## `body_text(p, text)`

Muted body copy at 16px. Default item style inside a card.

## Typical card stack

```rust
let body = row([
    card(p, Variant::Card, [kicker(p, "First"), body_text(p, "Some copy")]),
    card(p, Variant::Ok, [kicker(p, "Second"), body_text(p, "More copy")]),
])
.gap(24.0);

Fig::tree(shell(p, TITLE, SUB, col([body])))
```

Worked figures: [examples.md](examples.md).
