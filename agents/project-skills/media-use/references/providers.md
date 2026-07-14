# Providers, setup, and privacy

## Setup

```bash
curl -fsSL https://static.heygen.ai/cli/install.sh | bash
heygen update
heygen auth login --oauth
node <SKILL_DIR>/scripts/resolve.mjs --doctor
```

The documented default path requires authenticated HeyGen CLI 0.3.0 or newer, Node 18 or newer, `ffmpeg`, and `ffprobe`; `--doctor` fails if any check is unhealthy. OAuth can use eligible web-plan allowances; API keys use the normal API billing path. The resolver holds no provider keys and delegates authentication to each external tool. A targeted local-only operation may need only its selected local provider.

## Provider cascade

| Type | Provider path |
| --- | --- |
| bgm/sfx | HeyGen catalog free-usage path |
| image | HeyGen search, optional local mflux, Codex image upsell |
| voice | HeyGen TTS, optional local Kokoro fallback |
| icon | HeyGen asset search |
| logo | svgl, simple-icons, GitHub org avatar, domain favicon |
| grade/lut | local preset map, look index, deterministic cube |
| video | HeyGen avatar video, optional local LTX |

`--local-only` skips network providers and leaves caches plus installed local providers. Force a provider with `--provider`. Confirm an agent-initiated operation that may bill credits because remaining allowances are not always observable.

## Tools

| Tool | Purpose | Install |
| --- | --- | --- |
| `ffmpeg` / `ffprobe` | probing, grade analysis, cuts, ducking, loudness | `brew install ffmpeg` |
| `heygen` | catalog, TTS, avatar video | installer above, then OAuth login |
| `mflux-generate` | local FLUX image generation | mflux 0.9.6 in a dedicated uv venv |
| `codex` | ChatGPT-subscription image generation | logged-in Codex CLI |
| `parakeet-mlx` | local transcription | install in a dedicated uv venv |
| `ltx-2-mlx` | local video generation | clone and `uv sync --all-extras` |
| `npx hyperframes` | Kokoro, whisper fallback, background removal | HyperFrames CLI |

Read `scripts/lib/local-models.mjs` for RAM-graded local model ladders and exact invocations. Missing tools print a diagnostic and fall through when another provider exists.

## Telemetry and privacy

Resolver and edit-tool telemetry shares the HyperFrames CLI/studio install id in `~/.hyperframes/config.json`. When signed in to HeyGen, it identifies the same account by email or username so the surfaces deduplicate. Events contain coarse media type, source, provider, and counts; they omit intent text, filenames, and paths, and set `$ip: null`.

Disable telemetry with `DO_NOT_TRACK=1` or `HYPERFRAMES_NO_TELEMETRY=1`; it is also disabled in CI and development. Delivery is best-effort and never fails a media operation. Use `resolve --stats` or [telemetry-dashboard.md](telemetry-dashboard.md) for local inspection.
