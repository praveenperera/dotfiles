# Skill packs

Skill packs group project-local skills, MCP snippets, installed Codex plugin sources, and other packs so a project can opt into a working agent setup with one command.

## Usage

```bash
cmd pack add web
cmd pack add native
cmd pack add cli
cmd pack add web native
cmd pack refresh
cmd pack refresh --all
cmd pack add
```

Running without names opens an `fzf` multi-select picker.

Packs are installed into the current Git repository:

- `skills` are symlinked into `.agents/skills`
- `mcps` are merged into `.codex/config.toml`
- `plugin_sources` link skills from installed Codex plugins and merge any plugin MCP servers into `.codex/config.toml`

`.agents/skills` is globally ignored, so linked project skills stay local unless a project explicitly force-adds them.

Codex does not load plugin enablement from project-local `.codex/config.toml`, so packs do not write `[plugins]` entries. Plugin-sourced skills are linked through the resolved Codex home under `plugin-skill-links` so the dotfiles repo does not vendor plugin contents.

`cmd pack add` records the current Git repository and selected packs in local untracked state at `~/.local/state/cmd/pack-projects.toml`. `cmd pack refresh` refreshes the current registered project, and `cmd pack refresh --all` refreshes every registered project. Missing project directories are removed from the registry during `--all`.

`cmd cfg` also runs `cmd pack refresh --all`, so registered project packs are refreshed whenever dotfiles config is reapplied.

## Pack format

Each pack is a TOML file in this directory. The file stem is the pack name.

```toml
description = "Svelte UI work"
skills = ["svelte"]
mcps = []
plugin_sources = []
packs = []
```

All fields are optional.

- `skills` must match directories under `agents/project-skills`
- `mcps` must match TOML files under `agents/project-mcps`
- `plugin_sources` are installed Codex plugin IDs, such as `build-ios-apps@openai-curated`
- `packs` are other pack names in this directory

Nested packs are expanded recursively. `cmd pack add` dedupes repeated skills, MCPs, and plugin sources, and fails on missing references or pack cycles.
