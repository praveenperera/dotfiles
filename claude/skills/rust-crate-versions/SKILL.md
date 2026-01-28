---
name: rust-crate-versions
description: Fetch latest crate versions from crates.io. Use when creating Rust projects, adding dependencies to Cargo.toml, or when the user asks about current crate versions.
---

# Rust Crate Versions

This skill fetches the latest versions of Rust crates from crates.io using the `cmd crate versions` command.

## When to Use

Use this skill when:
- Creating a new Rust project with dependencies
- Adding dependencies to an existing Cargo.toml
- User asks what the latest version of a crate is
- User wants to update dependencies to latest versions
- You need to write Cargo.toml dependency entries

**IMPORTANT**: Always use this skill instead of hardcoding crate versions. Hardcoded versions quickly become outdated.

## How to Use

### Fetch Latest Versions

```bash
cmd crate versions serde tokio eyre
```

Output (toml format by default):
```
serde = "1.0.210"
tokio = "1.41.0"
eyre = "0.6.12"
```

### Output Formats

**TOML (default)** - Ready to paste into Cargo.toml:
```bash
cmd crate versions serde tokio --format toml
```

**JSON** - Structured output:
```bash
cmd crate versions serde tokio --format json
```

**Plain** - Just name and version:
```bash
cmd crate versions serde tokio --format plain
```

### Exact Version Pinning

Use `--exact` to get `=1.0.0` format:
```bash
cmd crate versions serde --exact
# Output: serde = "=1.0.210"
```

## Example Workflow

User asks: "Create a Rust CLI app with serde, tokio, and clap"

1. First, fetch the latest versions:
```bash
cmd crate versions serde tokio clap color-eyre
```

2. Use the output to create Cargo.toml with current versions

## Notes

- Fetches from crates.io API in parallel for efficiency
- Returns the latest non-yanked version
- Errors are reported to stderr; successful results still print
