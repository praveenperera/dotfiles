# Agent instructions

`claude/CLAUDE.md` (Claude Code) and `agents/AGENTS.md` (Codex and OpenCode) are generated from the fragments in this directory. Edit the fragments, never the generated files.

- `common.md` holds the rules for every client.
- `claude.md` and `codex.md` hold the rules that only make sense for one client.

Each fragment is plain Markdown made of `# ` sections. A client section with the same heading as a common section adds its bullets to the end of that section; any other client section comes after the common ones.

After an edit, run from anywhere in this checkout:

```bash
cmd instructions render   # rewrite both generated files
cmd instructions check    # fail if either file differs from a fresh render
```

`cmd bootstrap` and `cmd config` run the check before they link anything, and `cargo test` in `cmd/` runs it against the checked-in files.
