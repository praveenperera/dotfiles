---
name: media-use
description: Resolve, generate, transform, catalog, and reuse media for HyperFrames projects. Use for BGM, SFX, images, icons, official brand logos, voiceover, TTS, captions, transcription, background removal, color grades, LUTs, media cuts or reframes, and cross-project media reuse. Produces frozen local assets or paste-ready blocks plus manifest records while keeping search noise out of context.
---

# media-use

Use media-use as the single media entry point for HyperFrames. HyperFrames owns playback; this skill owns discovery, generation, transformation, freezing, provenance, and reuse.

## Start here

Install and authenticate the HeyGen CLI to unlock the OAuth free-usage path for catalog search, TTS, and avatar video, then verify the machine:

```bash
curl -fsSL https://static.heygen.ai/cli/install.sh | bash
heygen update
heygen auth login --oauth
node <SKILL_DIR>/scripts/resolve.mjs --doctor
```

Use OAuth for web-plan/free allowances where eligible; API-key authentication follows API billing. The documented default path requires authenticated HeyGen CLI 0.3.0 or newer, Node 18 or newer, `ffmpeg`, and `ffprobe`; `--doctor` fails when any is unhealthy. Individual local-only operations may need only their selected local provider.

## Core workflow

1. Choose the media type and describe the intended role, mood, subject, or look.
2. List reusable candidates before fetching when an existing asset may fit.
3. Resolve fresh only when no candidate is a trustworthy match.
4. Use the returned path or block; read `.media/index.md` for the inventory.
5. Register outputs from external transforms with `resolve --from`.

```bash
node <SKILL_DIR>/scripts/resolve.mjs --type <type> --intent "<description>" --candidates --project .
node <SKILL_DIR>/scripts/resolve.mjs --type <type> --intent "<description>" --project .
node <SKILL_DIR>/scripts/resolve.mjs --type <type> --from <file-or-url> --project .
```

The command returns one resolved path or paste-ready block and records it in `.media/manifest.jsonl`. For resolver types, flags, reuse rules, manifests, adoption, stats, and examples, read [references/resolver.md](references/resolver.md).

## Route by task

- Resolve BGM, SFX, image, icon, logo, voice, grade, or LUT: [references/resolver.md](references/resolver.md)
- Author or select color grades and LUTs: [references/color-grading.md](references/color-grading.md)
- Install providers, understand cascades, or inspect telemetry/privacy: [references/providers.md](references/providers.md)
- Cut, reframe, stitch, transcribe, transcript-cut, duck, normalize, upscale, or generate video: [references/operations.md](references/operations.md)
- Run the shared voice/BGM/SFX pipeline: [audio/references/audio-engine.md](audio/references/audio-engine.md)
- Configure provider dependencies: [audio/references/requirements.md](audio/references/requirements.md)
- Produce TTS: [audio/references/tts.md](audio/references/tts.md)
- Choose or generate BGM: [audio/references/bgm.md](audio/references/bgm.md)
- Place SFX: [audio/references/sfx.md](audio/references/sfx.md)
- Transcribe media: [audio/references/transcribe.md](audio/references/transcribe.md)
- Turn TTS into captions: [audio/references/tts-to-captions.md](audio/references/tts-to-captions.md)
- Author caption layout and styling: [audio/references/captions/authoring.md](audio/references/captions/authoring.md)
- Animate captions: [audio/references/captions/motion.md](audio/references/captions/motion.md)
- Reconcile scripts and transcripts: [audio/references/captions/transcript-handling.md](audio/references/captions/transcript-handling.md)
- Remove image backgrounds through the shared engine: [audio/references/remove-background.md](audio/references/remove-background.md)
- Inspect telemetry locally: [references/telemetry-dashboard.md](references/telemetry-dashboard.md)

## Guardrails

- Reuse only when description, prompt, type, duration or dimensions, and entity identity support the choice. Resolve fresh when uncertain.
- Reuse a cross-project brand asset only on an exact entity match. Resolve official marks through the logo cascade; never redraw them.
- Never read a `.cube` body into context. Inspect metadata, validate with `cube-validate.mjs`, and compare rendered looks with `hyperframes grade-compare`.
- Confirm before an agent-initiated call that may incur paid credits. Run an explicitly user-requested provider as directed.
- Keep transformations reproducible, then ingest their outputs so the project remains self-contained.

## Media opportunity pass

When building or reviewing a composition, make one grounded pass for missing media. Offer a consolidated, concrete set of improvements only when the artifact shows a signal: scripted text without voice, placeholder or upscaled imagery, emoji used as icons, hard cuts without SFX, a piece over about ten seconds without a music bed, or visibly misexposed/color-cast footage. Ask once for all, some, or none; never silently mutate the composition or re-raise a declined offer.
