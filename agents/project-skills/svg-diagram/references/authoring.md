# Authoring

Figures are trees of `El` (`engine/src/el.rs`). Building a tree does not measure text or run layout, so `build` is cheap to test.

Auto-size is the default: a node with no `.w()` / `.h()` takes the size of its content. Fixed sizes are the escape hatch.

## Constructors

```rust
row(children)   // horizontal flex
col(children)   // vertical flex
txt("…")        // wrapping text leaf
spacer()        // empty box that eats free space on the main axis
```

`.size()`, `.bold()`, `.color()`, and the other text methods panic if they are called on a row or col. `.child()` on `txt` panics.

## Layout

| Method | Role |
|---|---|
| `.w(n)` / `.h(n)` | fixed size |
| `.min_w(n)` / `.min_h(n)` | floor |
| `.grow(n)` | take `n` shares of the parent main axis; also restores shrink |
| `.gap(n)` | space between children |
| `.pad(n)` | padding on all sides |
| `.px` / `.py` / `.pt` / `.pr` / `.pb` / `.pl` | padding per side |
| `.margin(n)` | margin on all sides |
| `.mt` / `.mr` / `.mb` / `.ml` | margin per side |
| `.items(Align)` | cross-axis alignment of children |
| `.justify(Justify)` | main-axis distribution |
| `.align_self(Align)` | override the parent's `.items` |
| `.absolute()` | leave the flex line; place by insets |
| `.top` / `.right` / `.bottom` / `.left` | insets; may be negative |

`Align`: `Start`, `Center`, `End`, `Stretch`.

`Justify`: `Start`, `Center`, `End`, `Between`, `Around`, `Evenly`.

Absolute insets are measured from the parent's border box, not its padding box.

## Text

`txt` defaults to Inter, weight 400, size 16.

| Method | Role |
|---|---|
| `.size(n)` | px |
| `.weight(n)` / `.bold()` / `.semibold()` | 700 / 600 |
| `.color(p.token)` | palette color |
| `.mono()` | JetBrains Mono, pinned at weight 500 |
| `.italic()` | italic face |
| `.tracking(n)` | letter spacing in px; not scaled by the safety pad |
| `.center()` | center lines once the box is wider than the copy |

## Box style

| Method | Role |
|---|---|
| `.bg(color)` | fill |
| `.border(width, color)` | stroke; width insets the content box |
| `.rounded(n)` | corner radius |

A container with neither fill nor border paints nothing.

## Identity and composition

| Method | Role |
|---|---|
| `.id("slot")` | publish the laid-out rect for the decor pass; ids are unique |
| `.label("…")` | name used in overflow messages |
| `.child(el)` / `.children(iter)` | append |
| `.child_if(cond, \|\| el)` | append only when `cond` |

House chrome lives in [components.md](components.md). Marks that the box vocabulary cannot make live in [decor.md](decor.md). Copyable figures: [examples.md](examples.md).
