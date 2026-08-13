# Theming

`build` receives `&Palette` for the theme being rendered (`engine/src/color.rs`). Take colors from it. Never inline a hex.

## Tokens

| Token | Use |
|---|---|
| `p.bg` | canvas / chrome backdrop |
| `p.shell_bg` | `shell()` backdrop; differs from `p.bg` in light mode |
| `p.card` | elevated surface |
| `p.stroke` | default border |
| `p.text` | primary copy |
| `p.muted` | secondary copy |
| `p.kicker` | small heading; also the blue variant's foreground |
| `p.ok` / `p.ok_bg` / `p.ok_stroke` | success family |
| `p.warn` / `p.warn_bg` / `p.warn_stroke` | warning family |
| `p.bad` / `p.bad_bg` / `p.bad_stroke` | failure family |
| `p.info` / `p.info_bg` / `p.info_stroke` | info family |
| `p.blue_bg` / `p.blue_stroke` | blue family |
| `p.fill_ok` / `p.fill_bad` / `p.fill_warn` / `p.fill_blue` / `p.fill_purple` | solid fills |
| `p.og` | separate scheme for OG and article figures |

## `Variant`

Names a family for `card()` and other variant-colored marks:

`Card`, `Ok`, `Warn`, `Bad`, `Info`, `Blue`.

```rust
let colors = p.variant_colors(Variant::Warn); // .bg, .border, .fg
let fg = p.variant_fg(Variant::Ok);
```

## `ArrowColor`

Connectors must use one of these, or `marker-end` resolves to nothing:

`Text`, `Muted`, `Ok`, `Bad`, `Warn`, `Info`.

## OG scheme

`p.og` is the article / social-card palette: `bg`, `panel`, `card`, `header_text`, `body_text`, `code_text`, `muted_text`, `line`, icon-box tokens, and `pill(Pill::{Default, Secure, Untrusted})`.

Social OG cards are composed against a dark backdrop and ship as one dark PNG. Article figures still render both themes.
