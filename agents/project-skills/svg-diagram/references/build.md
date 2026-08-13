# Build

```sh
cd ~/code/diagrams
just build
```

That renders every registered figure and writes `{name}-light.svg` / `{name}-dark.svg` (or `.png`) into the directories the figure registers. A figure whose content leaves its canvas fails the build.

## One figure

```sh
cargo run --release --manifest-path engine/Cargo.toml -- --list
cargo run --release --manifest-path engine/Cargo.toml -- --only my-figure --out _scratch
cargo run --release --manifest-path engine/Cargo.toml -- --check
```

| Flag | Effect |
|---|---|
| `--only <name>` | build named figures only; repeatable |
| `--theme light\|dark` | one theme, whatever the figure registers |
| `--png` | also rasterize figures that only register SVG |
| `--out <dir>` | write everything here instead of the published directories |
| `--check` | render and report, write nothing |
| `--list` | print the default set and stop |

An unknown figure name or theme stops the build.

## Checks

```sh
just check   # fmt-check, clippy, test
```

`engine/tests/metrics_parity.rs` holds text metrics against a frozen corpus from the old pipeline. That corpus cannot be regenerated. A failure there means shipped figures would wrap differently than they do today.
