# Resolver

## Contents

- [Types and providers](#types-and-providers)
- [Common commands](#common-commands)
- [Flags](#flags)
- [Reuse rules](#reuse-rules)
- [Resolution floor](#resolution-floor)
- [Adopt and ingest](#adopt-and-ingest)
- [Inventory and cache](#inventory-and-cache)
- [Usage stats](#usage-stats)

## Types and providers

| Type | Resolves | Default path |
| --- | --- | --- |
| `bgm` | background music | HeyGen catalog |
| `sfx` | sound effects | bundled library, then HeyGen catalog |
| `image` | photos and backgrounds | HeyGen search, optional local mflux, Codex upsell |
| `icon` | symbols and icons | HeyGen asset search |
| `logo` | official brand marks | svgl, simple-icons, GitHub org avatar, domain favicon |
| `voice` | TTS voiceover | HeyGen OAuth/free-usage path, optional local Kokoro |
| `grade` | `data-color-grading` blocks | core preset, look index, deterministic cube |
| `lut` | reusable `.cube` files | look index, deterministic cube |

## Common commands

```bash
node <SKILL_DIR>/scripts/resolve.mjs -t bgm -i "upbeat tech launch" -p .
node <SKILL_DIR>/scripts/resolve.mjs -t sfx -i "short whoosh" -p .
node <SKILL_DIR>/scripts/resolve.mjs -t image -i "gradient tech background" -p .
node <SKILL_DIR>/scripts/resolve.mjs -t icon -i "rocket" -p .
node <SKILL_DIR>/scripts/resolve.mjs -t logo -e linkedin -i "LinkedIn logo" -p .
node <SKILL_DIR>/scripts/resolve.mjs -t grade -i "warm daylight" -p . --json
node <SKILL_DIR>/scripts/resolve.mjs -t lut -i "teal orange blockbuster" -p .
```

## Flags

| Flag | Meaning |
| --- | --- |
| `--type, -t` | bgm, sfx, image, icon, logo, voice, grade, or lut |
| `--intent, -i` | natural-language need |
| `--entity, -e` | optional entity identity for cache matching |
| `--project, -p` | project directory; defaults to `.` |
| `--candidates` | list project and global candidates without mutation |
| `--reuse <sha>` | import a chosen global-cache asset |
| `--from <path-or-url>` | freeze and register existing media |
| `--for <media>` | analyze an image/video for a grade suggestion |
| `--local-only` | skip every network provider |
| `--provider <name>` | force one provider |
| `--adopt` | register an existing `assets/` tree |
| `--doctor` | check dependencies without manifest changes |
| `--stats` | summarize local usage without manifest changes |
| `--days N` | window timestamped stats and misses |
| `--json` | emit machine-readable JSON |

## Reuse rules

Run `--candidates` before resolving when existing media may fit. A project candidate can be referenced directly. Import a global candidate with `--reuse <sha>` so the project stays self-contained. Judge semantic fit from description, prompt, type, duration or dimensions, and entity. Never loosely match a global brand asset to another entity.

Exact normalized prompt matches reuse automatically; fuzzy similarity never does. A miss prints a candidate hint but does not choose for the agent.

## Resolution floor

The resolver checks the project manifest, matching unregistered files under `assets/`, and the global cache before calling a provider. It then freezes the result under `.media/`, writes the manifest, regenerates the index, and promotes reusable files to `~/.media/`.

## Adopt and ingest

```bash
node <SKILL_DIR>/scripts/resolve.mjs --adopt --project .
node <SKILL_DIR>/scripts/resolve.mjs --type image --from ./output.png --project .
```

Adoption probes duration and dimensions with `ffprobe`. Use `--from` for transform or generation outputs from tools outside the resolver.

## Inventory and cache

- `.media/manifest.jsonl`: machine source of truth
- `.media/index.md`: compact agent-readable inventory
- `~/.media/`: content-addressed global cache
- `~/.media/misses.jsonl`: local resolve misses used by stats

## Usage stats

```bash
node <SKILL_DIR>/scripts/resolve.mjs --stats --project . --days 7
node <SKILL_DIR>/scripts/resolve.mjs --stats --project . --json
```

Stats cover project records, global-cache assets and bytes, cross-project reuse, misses, hit rate, provider/source/via counts, and top missed intents. Intent text stays local.
