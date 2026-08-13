# Overflow

The engine wraps text only when the box has a width. A figure whose content leaves its canvas fails the build. Do not turn the check off.

## Author so copy can wrap

- Put copy in `txt(...)`, `kicker(...)`, or `body_text(...)`. Do not drop a long string into a row of unsized leaves.
- Use `card` for panels. Use a `row` of `.grow(1.0)` children when several cards sit side by side. Grown siblings divide the axis in proportion, however unequal their copy is.
- `txt` keeps `flex-shrink: 1`, so `row([txt("…")])` wraps to the card instead of running off the side.
- Keep a kicker to a few words. Put the long sentence on the next line.
- Do not pin `.w()` / `.h()` unless the figure needs a fixed box. Auto-size is the default. A fixed height that is shorter than the wrapped copy is a hard error (`TextOverflow`).
- Name a node with `.label("…")` when you want the overflow report to say that, not the node path.

## When the gate fires

Shorten the copy or shrink the type. Re-run:

```sh
cargo run --release --manifest-path engine/Cargo.toml -- --only my-figure --out _scratch
```

`--check` renders and reports without writing files.

`gate-smoke` overflows on purpose and is hidden from the default build. It is only reachable through `--only`.
