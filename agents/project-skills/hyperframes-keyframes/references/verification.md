# Keyframe Verification

Verify painted pixels, not only runtime registration or diagnostic text.

## Commands

```bash
npx hyperframes lint
npx hyperframes validate
npx hyperframes keyframes .
npx hyperframes keyframes . --json
npx hyperframes keyframes . --runtime all
npx hyperframes keyframes . --selector "<selector>" --shot "<file>" --samples <n>
npx hyperframes keyframes . --selector "<selector>" --shot "<file>" --layout strip --from <t0> --to <t1>
npx hyperframes keyframes . --shot "<file>" --ghost --angle <angle>
npx hyperframes snapshot . --at <times>
```

Choose `<selector>` for the real animated subject. Choose `<times>` for the first frame, proof poses, final-minus-hold, and exact final. Use `<angle>` only when depth must be proven.

| Tool | Proves |
| --- | --- |
| `keyframes` | targets, explicit stops, paths, traces, composed parent and child motion, CSS stops, Anime registration |
| `--shot` | ghosts, route shape, time spacing, DOM 3D projection, focused selector proof |
| `--layout strip` | in-place motion, overlaps, contact, subtle scale or opacity, text waves |
| `--ghost` | canvas, WebGL, shader motion, rendered 3D |
| `snapshot --at` | masks, text readability, full state, final lockup, black or reset tails |

## Selector diagnosis

If selector proof looks wrong:

1. Rerun with `--json`.
2. Find the actual animated target.
3. Shoot that target.
4. Snapshot full frames.
5. Trust painted pixels over logs.

A helper-selector shot is not proof. An onion shot over a broken full frame is not proof.

## Reading diagnostics

- `flat` means no explicit middle poses.
- `keyframes` means explicit stops exist.
- `motionPath` means a route exists.
- `trace` means multi-stroke drawing.
- `composed with` means child motion inherits parent motion.
- Even ghost spacing means constant speed.
- Clustered ghosts mean slow-in or settle.
- Large gaps mean fast travel.

## Repair table

| Failure | Fix |
| --- | --- |
| endpoint-only | add middle poses, hold peak proof, rerun `--shot` |
| identity break | keep one element alive, use shared source and final boxes, remove substitute crossfade |
| fake 3D | add z or camera travel, occlusion, and angled proof |
| wrong final | add a final hold; snapshot final-minus-hold and exact final |
| unseekable runtime | pause autoplay, register the instance, remove timers, build synchronously |
| unreadable text | preserve line boxes, reduce displacement, add a final hold, snapshot text frames |

## Completion gate

Run lint, validate, keyframes, one focused `--shot`, and snapshots. Confirm the first frame, proof poses, final-minus-hold, exact final, subject-owned motion, and absence of debug overlays.
