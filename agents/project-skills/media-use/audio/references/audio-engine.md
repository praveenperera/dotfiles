# Shared audio engine

Use the engine for one coordinated TTS, BGM, and SFX pass:

```bash
node <SKILL_DIR>/audio/scripts/audio.mjs --request ./audio_request.json --out ./audio_meta.json
```

The request shape is:

```json
{
  "provider": "optional",
  "lang": "optional",
  "speed": 1,
  "lines": [{ "id": "scene-1", "text": "Narration", "sfx": ["whoosh"] }],
  "bgm": { "mode": "retrieve", "query": "calm cinematic underscore" }
}
```

`bgm.mode` accepts `retrieve`, `generate`, or `none`; omit it for automatic selection. `--only tts,bgm,sfx` runs a subset and merges it into an existing output.

The engine writes `audio_meta.json` plus assets under `.media/audio/{voice,bgm,sfx}`. Voice records include stable line ids, paths, duration, and word timestamps for captions. It also reports SFX, BGM, total duration, and any pending BGM generation.

HeyGen CLI auth unlocks TTS and music/SFX retrieval through the OAuth/free-usage path. Local or provider-specific generators remain explicit alternatives where installed. If BGM is pending, run `audio/scripts/wait-bgm.mjs` before final assembly.

Read the topic guide needed for the task:

- [requirements.md](requirements.md) for dependencies and credentials
- [tts.md](tts.md) for provider selection, voices, languages, and long scripts
- [bgm.md](bgm.md) for retrieval, generation, default volume, and pending jobs
- [sfx.md](sfx.md) for bundled effects and cue placement
- [transcribe.md](transcribe.md) for speech-to-text
- [remove-background.md](remove-background.md) for cutouts and compositing
- [tts-to-captions.md](tts-to-captions.md) for word timestamps to captions
- [captions/authoring.md](captions/authoring.md) for caption layout and styling
- [captions/motion.md](captions/motion.md) for caption animation
- [captions/transcript-handling.md](captions/transcript-handling.md) for script/transcript reconciliation

For single-shot TTS, use `audio/scripts/heygen-tts.mjs`. For media operations beyond the shared pass, read [../../references/operations.md](../../references/operations.md).
