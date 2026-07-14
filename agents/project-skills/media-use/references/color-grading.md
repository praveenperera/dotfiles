# Color grading and LUTs

Use `grade` for a paste-ready HyperFrames `data-color-grading` value. Use `lut` when only the reusable `.cube` file is needed.

## Resolve a look

```bash
node <SKILL_DIR>/scripts/resolve.mjs -t grade -i "warm daylight" -p . --json
node <SKILL_DIR>/scripts/resolve.mjs -t lut -i "teal orange blockbuster" -p .
```

Preset-backed output is local and compact:

```json
{"preset":"warm-daylight","intensity":1}
```

Paste the JSON as the escaped `data-color-grading` value. Looks beyond the runtime preset vocabulary freeze a validated cube and return a block with a LUT source and intensity.

## Author a technical look

Use bounded params for describable technical adjustments:

```bash
node <SKILL_DIR>/scripts/resolve.mjs -t lut --params '{"contrast":0.2,"temperature":-0.3}' -p .
node <SKILL_DIR>/scripts/resolve.mjs -t grade --params '{"exposure":0.2}' -p . --json
```

Parametric math cannot reproduce real film stocks or emulsion looks. Use a scanned CDN-backed cube or ingest a real cube for those.

## Ingest and validate

```bash
node <SKILL_DIR>/scripts/resolve.mjs -t lut --from custom.cube -p .
node <SKILL_DIR>/scripts/lib/cube-validate.mjs .media/luts/lut_001.cube
```

Do not commit generated cube bodies. The resolver validates and freezes them under `.media/luts/`. Never read cube contents into model context; a 33³ LUT contains about 36,000 data lines with no useful semantic signal.

## Compare visually

List looks with `resolve -t grade --candidates`, write plausible entries to `grades.json`, and run `hyperframes grade-compare --for <frame> --grades grades.json`. Commit the winner with a final `resolve -t grade` call.

Use `grade --for <media>` for measured `ffmpeg`/`ffprobe` signalstats and a bounded adjustment suggestion. Treat it as a starting point, not an automatic neutralization of an intentional sunset, neon look, or other art direction.

The look catalog lives in `luts/index.json`. Each entry supplies a description, tags, intensity, and either compact params or a direct CDN URL.
