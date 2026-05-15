# Skill packs

Skill packs group project-local skills, MCP snippets, Codex Desktop plugins, and other packs so a project can opt into a working agent setup with one command.

## Usage

```bash
cmd pack add web
cmd pack add native
cmd pack add cli
cmd pack add web native
cmd pack add
```

Running without names opens an `fzf` multi-select picker.

Packs are installed into the current Git repository:

- `skills` are symlinked into `.agents/skills`
- `mcps` are merged into `.codex/config.toml`
- `plugins` are enabled in `.codex/config.toml`

`.agents/skills` is globally ignored, so linked project skills stay local unless a project explicitly force-adds them.

## Pack format

Each pack is a TOML file in this directory. The file stem is the pack name.

```toml
description = "Svelte UI work"
skills = ["svelte"]
mcps = []
plugins = []
packs = []
```

All fields are optional.

- `skills` must match directories under `agents/project-skills`
- `mcps` must match TOML files under `agents/project-mcps`
- `plugins` are Codex plugin IDs, such as `build-ios-apps@openai-curated`
- `packs` are other pack names in this directory

Nested packs are expanded recursively. `cmd pack add` dedupes repeated skills, MCPs, and plugins, and fails on missing references or pack cycles.
