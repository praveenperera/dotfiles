---
name: hyperframes-animation
description: "All animation knowledge for HyperFrames — atomic motion rules, multi-phase scene blueprints, scene transitions, broader motion-design techniques, AND the seven runtime adapters (GSAP default, plus Lottie, Three.js, Anime.js, CSS keyframes, Web Animations API, TypeGPU). Use for any motion or animation task: pick 2-4 rules and compose, or load a blueprint, or look up runtime-specific API (e.g. GSAP eases / Lottie player / Three.js mixer). Also covers auditing an existing composition's choreography (animation map) and 24 named text-animation effects. HyperFrames-native: single paused timeline, seek-safe, deterministic."
---

# HyperFrames Animation

All motion knowledge in one skill: **rules** (atomic recipes), **blueprints** (multi-phase scene templates), **transitions** (scene-to-scene), **techniques** (broader motion-design patterns), and **adapters** (per-runtime APIs).

For the composition contract (data attributes, sub-compositions, determinism), use `hyperframes-core`.

## Default: compose atomic rules

Pick 2-4 rules from the [rules index](rules-index.md) and compose them with a single paused GSAP timeline. This is faster and produces less code than starting from a blueprint.

## Load a blueprint when

- The scene matches an existing pre-designed multi-phase template (brand-reveal, social-proof, etc.) and reusing its phase pipeline saves real authoring time
- You want runnable ground-truth code for a complex 4-5 phase choreography

Blueprints live in the [blueprints index](blueprints-index.md). Each entry points to one recipe and, where available, runnable example HTML with relative bundled assets. Do not load a blueprint speculatively; use one only after deciding that scene-level orchestration is needed.

## Routing

| Want to…                                                                       | Read                                                |
| ------------------------------------------------------------------------------ | --------------------------------------------------- |
| Pick an atomic motion pattern by trigger or tag                                | [Rules index](rules-index.md)                                                   |
| Read one rule's full HTML, CSS, and GSAP recipe                                | Follow its direct link from the [rules index](rules-index.md)                   |
| Pick a multi-phase scene template                                              | [Blueprints index](blueprints-index.md)                                         |
| Read one blueprint and its runnable example                                    | Follow its direct links from the [blueprints index](blueprints-index.md)        |
| Author a scene transition between two clips                                    | [Transition overview](transitions/overview.md) and [catalog](transitions/catalog.md) |
| Look up a broader motion-design technique                                      | [Techniques](techniques.md)                                                     |
| Analyze an existing composition's animation map                                | [Animation-map script](scripts/animation-map.mjs)                               |
| GSAP timeline, tweens, and position parameters                                 | [GSAP adapter](adapters/gsap.md)                                                |
| GSAP drop-in effect recipes                                                    | [GSAP effects](rules/gsap-effects.md)                                           |
| GSAP transforms and performance                                                | [Transform guide](adapters/gsap-transforms-and-perf.md)                         |
| GSAP eases and stagger                                                         | [Easing guide](adapters/gsap-easing-and-stagger.md)                             |
| GSAP timeline and labels                                                       | [Timeline guide](adapters/gsap-timeline-and-labels.md)                          |
| Lottie or dotLottie (`window.__hfLottie`)                                      | [Lottie adapter](adapters/lottie.md)                                            |
| Three.js or WebGL (`AnimationMixer`, `hf-seek`)                                | [Three.js adapter](adapters/three.md)                                           |
| Anime.js (`window.__hfAnime`)                                                  | [Anime.js adapter](adapters/animejs.md)                                         |
| CSS keyframes                                                                  | [CSS animation adapter](adapters/css-animations.md)                             |
| Web Animations API (`element.animate()`, `currentTime`)                        | [WAAPI adapter](adapters/waapi.md)                                              |
| TypeGPU or WebGPU (`navigator.gpu`, WGSL, compute pipelines)                   | [TypeGPU adapter](adapters/typegpu.md)                                          |
| HTML-as-texture with WebGL or GLSL post-effects                                | [HTML-in-canvas patterns](adapters/html-in-canvas-patterns.md)                   |
| Named text-animation effects                                                  | [Text-effects adapter](adapters/animate-text.md)                                |

## Picking a runtime

- **GSAP** is the default for 95% of motion work — covers timeline orchestration, transforms, easing, stagger. All atomic rules in this skill are GSAP-based.
- **Lottie** when an asset has its own pre-baked timeline (typically After Effects exports).
- **Three.js** for 3D scenes, camera motion, shader-driven visuals.
- **Anime.js** for lightweight tweening when GSAP is overkill.
- **CSS** for simple repeated motifs, decoration, shimmer — no JavaScript animation cost.
- **WAAPI** for native browser keyframes without a GSAP dependency.
- **TypeGPU / WebGPU** for GPU-rendered canvases (particles, liquid glass, custom shaders).

Multiple runtimes can coexist in one composition. Each registers its instances on the runtime-specific global so HyperFrames can seek all of them in one pass.

## Critical Constraints

**Prerequisite: `hyperframes-core` → Non-Negotiable Rules** (single paused timeline, `data-duration` governs length, no `Math.random` / `Date.now` / `performance.now`, no `repeat: -1`, no `gsap.set` on later-scene clips, no `display` / `visibility` animation, no timeline construction inside `async` / `setTimeout` / `Promise`). Don't restate those here.

Animation-craft additions on top of core's contract:

- **Pre-calculated layout constants** — never derive positions from `getBoundingClientRect()` at tween time. Tween-time DOM measurements desync because the renderer samples in parallel; compute coordinates once at composition setup and reuse.
- **Spatial motion uses GSAP transform aliases only** (`x`, `y`, `scale`, `rotation`). Core's allowlist also permits `opacity` / `color` / `backgroundColor` / `borderRadius` for non-spatial property tweens — but never `width` / `height` / `top` / `left` for layout changes.

## Scripts

```bash
node skills/hyperframes-animation/scripts/animation-map.mjs <composition-dir> \
  --out <composition-dir>/.hyperframes/anim-map
```

Reads every GSAP timeline registered on `window.__timelines`, enumerates tweens, samples bboxes, computes flags, outputs `animation-map.json`. Use it to audit choreography (dead zones, stagger consistency, lifecycle warnings) after authoring.

`animation-map.mjs` resolves helper packages from the current project first, then can bootstrap the bundled HyperFrames package version. Set `HYPERFRAMES_SKILL_PKG_VERSION=<version>` only when running the skill outside the bundled CLI/skill install and you need to pin that bootstrap version explicitly.

## See Also

- `hyperframes-core` — composition structure, data attributes, sub-compositions, deterministic render contract
- `hyperframes-creative` — palettes, typography, narration, beat planning (non-animation creative direction)
- `hyperframes-cli` — `npx hyperframes lint / validate / inspect / preview / render`
