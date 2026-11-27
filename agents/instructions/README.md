# Agent instructions

`claude/CLAUDE.md` (Claude Code), `agents/AGENTS.md` (Codex and OpenCode) and `agents/GEMINI.md` (Gemini Neon in Antigravity) are generated from the fragments in this directory. Edit the fragments, never the generated files.

- `common.md` holds the rules for every client.
- `claude.md`, `codex.md` and `gemini.md` hold the rules that only make sense for one client.

Each fragment is plain Markdown made of `# ` sections. A client section with the same heading as a common section adds its bullets to the end of that section; any other client section comes after the common ones.

After an edit, run from anywhere in this checkout:

```bash
cmd instructions render   # rewrite the generated files
cmd instructions check    # fail if any generated file differs from a fresh render
```

`cmd bootstrap` and `cmd config` run the check before they link anything, and `cargo test` in `cmd/` runs it against the checked-in files.

## Antigravity

`cmd config` links `agents/GEMINI.md` as `GEMINI.md`, and writes a `skills.json` that exposes `~/.agents/skills` by absolute path, into `~/.gemini/config/` for the standalone Antigravity CLI. It does the same in `config/` under every instance dir in `~/.t3/userdata/providers/antigravity/`. T3 starts Antigravity with `--gemini-dir` set to that instance dir, which replaces `~/.gemini`, so rerun `cmd config` after T3 creates a new Antigravity provider instance.
