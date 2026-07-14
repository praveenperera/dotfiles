---
name: hyperframes-keyframes
description: Use when a HyperFrames composition needs seek-safe 2D or 3D keyframes, GSAP timelines, CSS keyframes, Anime.js, WAAPI, FLIP, paths, masks, SVG morph or draw, text trails, 3D depth, or `hyperframes keyframes` diagnostics. Do not use for broad scene strategy, brand design, media sourcing, captions, or general video planning.
---

# HyperFrames Keyframes

Treat keyframes as a pose contract: visible states, continuous subject identity, a seek-safe runtime, and verified pixels.

Use `hyperframes-animation` for broad scene recipes and `hyperframes-cli` for full command documentation.

## Workflow

1. Identify the animated subject, visible states, final state, and runtime.
2. Choose the smallest mechanism that proves the prompt. Read [the mechanism reference](references/keyframe-patterns.md) only when the mechanism is unclear.
3. Read [the authoring contract](references/authoring.md), then build and register seek-safe keyframes synchronously.
4. Follow [the verification workflow](references/verification.md): lint, validate, inspect keyframes, capture one focused shot, and snapshot proof times.
5. Fix source keyframes and rerun the smallest failing diagnostic until the painted pixels prove the motion.

## Core contract

- Name the moving subject and the poses needed to prove the motion, including the final state.
- Keyframe visible channels, not hidden helper state.
- Preserve object identity when continuity matters; crossfade only for replacement or dissolve.
- Hold readable or semantic states long enough to see.
- Treat the final frame as part of the animation. Do not reset to rest or end on black unless requested.
- Preserve starter layout, copy, assets, colors, and final state unless asked to redesign.
- Build synchronously, register the runtime instance, and keep every duration and loop finite.
- Never drive render-critical motion with clocks, unseeded randomness, hover or scroll triggers, timers, async-created timelines, or unregistered animation loops.

## Routing

| Need | Read |
| --- | --- |
| Runtime registration, GSAP forms, channels, timing, text, SVG, 3D, canvas, or WebGL | [Authoring contract](references/authoring.md) |
| FLIP, path, stroke, morph, mask, text, state-machine, depth, or shader mechanism selection | [Mechanism reference](references/keyframe-patterns.md) |
| CLI proof commands, selector choice, diagnostic interpretation, or failure repair | [Verification workflow](references/verification.md) |

## Completion gate

Run lint, validate, `hyperframes keyframes`, one focused `--shot`, and snapshots. Confirm the first frame, proof poses, final-minus-hold, exact final, subject-owned motion, and absence of debug overlays.
