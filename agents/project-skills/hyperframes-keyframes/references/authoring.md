# Keyframe Authoring Contract

Use this reference after choosing the animated subject and mechanism.

## Contents

- [Runtime rules](#runtime-rules)
- [GSAP skeleton](#gsap-skeleton)
- [Keyframe forms](#keyframe-forms)
- [Channels](#channels)
- [Mechanism choice](#mechanism-choice)
- [Timing](#timing)
- [Text and SVG](#text-and-svg)
- [3D, canvas, and WebGL](#3d-canvas-and-webgl)

## Runtime rules

GSAP:

- build synchronously at page load
- use `gsap.timeline({ paused: true })`
- register as `window.__timelines[compositionId]`
- match the registry key to `data-composition-id`
- do not call `tl.play()` for render-critical motion
- keep repeats finite

CSS keyframes:

- use a finite duration and iteration count
- use a deterministic delay and `animation-fill-mode: both`
- use `data-start` when timing belongs to a clip

Anime.js:

- create synchronously with `autoplay: false`
- keep duration and loops finite
- push every instance to `window.__hfAnime`

WAAPI:

- use finite `duration`, `fill: "both"`, and deterministic construction
- verify with `--shot` and snapshots because the text diagnostic does not list WAAPI even though shots seek it

Never use clocks, unseeded randomness, hover or scroll triggers, timers, async-created timelines, unregistered `requestAnimationFrame`, or infinite loops for render-critical motion.

## GSAP skeleton

```js
const root = document.querySelector("[data-composition-id]");
const compositionId = root.dataset.compositionId;
const tl = gsap.timeline({ paused: true });

tl.addLabel("state-a", 0);
tl.to(".subject", {
  keyframes: [
    { x: 0, opacity: 1, duration: 0.2 },
    { x: 120, opacity: 1, duration: 0.4, ease: "power2.out" },
    { x: 100, opacity: 1, duration: 0.2, ease: "power2.inOut" },
  ],
  ease: "none",
});

window.__timelines = window.__timelines || {};
window.__timelines[compositionId] = tl;
```

Use labels for semantic states and position parameters instead of chained delays. Use `immediateRender: false` for later `from()` or `fromTo()` tweens that touch the same property.

## Keyframe forms

- Use array keyframes for a pose ladder with per-step duration or ease.
- Use percentage keyframes for exact timing inside one tween.
- Use property arrays for compact multi-stop changes.
- Set `ease: "none"` on the parent when each stop has its own easing.
- Use `easeEach` when every segment should share the same feel.

Derive numeric distances and timing from the composition geometry and duration. For one subject moving between two boxes, prefer one continuous transform tween or FLIP. Split transforms into several eased segments only when viewers should feel distinct beats; every segment changes velocity and can read as a hitch.

## Channels

Prefer compositor and visual channels: `x/y/z`, `xPercent/yPercent`, `scale`, `rotationX/Y/Z`, `skew`, `transformOrigin`, `svgOrigin`, `opacity`, `autoAlpha`, `clip-path`, masks, CSS variables, SVG path or dash values, camera transforms, and shader uniforms.

Avoid layout and lifecycle channels: `top/left/right/bottom`, `width/height`, `margin/padding`, `display`, `visibility`, late DOM creation, and helper overlays that perform the subject's motion.

## Mechanism choice

Choose the smallest mechanism that proves the prompt. Read the [mechanism reference](keyframe-patterns.md) for implementation skeletons and verification guidance.

| Need | Mechanism |
| --- | --- |
| Same subject changes box or hierarchy | shared element or FLIP |
| Subject travels a visible route | path travel |
| Stroke grows or traces | stroke draw |
| Shape becomes another shape | shape interpolation |
| Reveal boundary is visible | clip, mask, or shader uniform |
| Many items move with order | stagger or indexed delay |
| Text itself moves | line, word, character, or band subdivision |
| Surface bends, stretches, or crops | parent and child counter-transform |
| UI has states | explicit state machine |
| Scene has depth | DOM 3D, Three.js, or WebGL camera and object keyframes |

Mechanisms can combine, but each one must clarify the idea. Decoration is not proof.

## Timing

- Use anticipation only when it clarifies cause or direction.
- Let acceleration leave rest, peak proof show the mechanism, and follow-through sell energy and direction.
- Use overshoot only for elastic or tactile subjects.
- Use `ease: "none"` for constant-speed path travel and a sharp ease-out for discrete UI states.
- Give repeated elements ordered offsets and final lockups longer holds than transition poses.
- Preserve continuous velocity on the same subject.
- Do not overlap tweens writing the same transform unless intentional and verified.
- Avoid large mask or `clip-path` changes while the same hero surface scales or travels; use a nested reveal after the main move settles.

## Text and SVG

Preserve line boxes, word spacing, readability, and final fit. Move glyphs or masked bands when text moves internally, not only decorations around the text. Snapshot readable frames.

For stroke growth, prefer `DrawSVGPlugin`, then `stroke-dasharray` and `stroke-dashoffset`. For shape interpolation, prefer `MorphSVGPlugin`; convert primitives to paths when necessary and split complex silhouettes into simpler parts.

## 3D, canvas, and WebGL

Scale alone is not depth. Use perspective on a stable parent, `transform-style: preserve-3d`, z travel, rotation, camera or world motion, occlusion, and correct layer order when objects cross. Capture one or two diagnostic angles that expose the depth relationship.

Keyframe camera position and target, object transforms, material opacity, shader uniforms, and post-process intensity through deterministic state. Render from HyperFrames time. Use `--ghost` for canvas and WebGL because marker boxes cannot see their internal motion.
