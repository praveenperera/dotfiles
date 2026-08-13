# Publish

A figure names an `OutputSet`, not a path. Paths live on `OutputSet` in `engine/src/figures.rs`.

| Set | Writes to |
|---|---|
| `Coldcard` | `~/code/static_sites/posts/images/posts`, `~/code/blog-posts/images/posts` |
| `Og` | `~/code/static_sites/posts/images`, `~/code/static_sites/priv/static/images/posts` |
| `Bip110` | `~/code/bip110/web/public/images` |
| `Messianic` | `~/code/propheciesofjesus/public/og` |
| `Scratch` | repo `_scratch/`, from the crate root, not the working directory |

`--out <dir>` redirects every figure to one directory instead of these.

Article figures on the main site use `Og`. Coldcard post figures use `Coldcard`. Do not copy generated files into a directory the set does not name.

## Fonts

Inter (400, italic, 500, 600, 700) and JetBrains Mono (500) are vendored in `engine/fonts/` and embedded at build time. Stick to characters those faces contain. Symbols such as `⊕` and `≈` become missing glyphs.

## Embed in a post

From a year folder such as `2026/`:

```html
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/posts/my-diagram-dark.svg">
  <img src="../images/posts/my-diagram-light.svg" alt="Short factual description">
</picture>
```
