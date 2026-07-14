# Visual design — product-launch per-frame shot method

## Contents

- [Time-coded shot sequences](#the-unit-is-a-time-coded-shot-sequence)
- [Blueprint selection](#pick-the-shape--instantiate-a-blueprint)
- [Per-frame fields](#what-you-add-to-each-frame)
- [Layout](#layout--named-inline-per-scene)
- [Video direction](#video-direction)
- [Palette and type](#palette-and-type)
- [Caption keep-out](#caption-keep-out)
- [Reference map](#where-the-detail-lives)
- [Checklist](#checklist)

> The method behind **Step 4 (Frame visual design)**. Enrich `STORYBOARD.md` in place with a **time-coded shot sequence** for each frame. Write no HTML, do not reread `capture/`, and do not change story-owned assets or transitions. Use `frame.md` for palette and type, [`motion-language.md`](motion-language.md) for motion, and [`../../hyperframes-animation/blueprints-index.md`](../../hyperframes-animation/blueprints-index.md) for proven shot shapes.

## The unit is a time-coded shot sequence

A frame's visual layer is **a sequence of time windows paced to the voiceover**, not a bag of effect tags. The failure that reads as PowerPoint is **front-loading**: the agent rushes the whole canvas on screen in the first ~25%, and then it just sits (the old representation — a flat set of effect names + a prose note — fired everything at entrance and left the rest empty). A time-coded shot sequence written **against the VO** makes that impossible: each window states what is on screen and what is moving, and **nothing appears before the voiceover reaches it.**

Write each frame as a handful of windows cued by the spoken line:

```
Scene 1 (0.0–Xs):  only what the VO is saying at t=0 enters — never the whole canvas
Scene 2 (Xs–Ys):   the next piece reveals as the VO names it (a line / card / stat / icon)
  …                one window per spoken cue — as many or as few as the line calls for
Scene N (…–end):   content has resolved; hold the read (stillness; subtle jitter at most)
```

- Each `Scene` line names **what's on screen**, **what moves in this window**, and **where it sits** (layout, inline). Times are real seconds across the frame's `duration`.
- **Pace reveals to the voiceover; never front-load.** This is the core anti-PowerPoint mechanism (→ `motion-language.md` Part 2 Rule 2). At t=0 show only what the VO is saying then; reveal each further piece — a line, a card, even an h1 — **when the VO names it**, spreading reveals across the shot and especially the **back ~50%**. **The window count = the number of spoken cues the line calls for** — a two-beat line is two windows, a five-feature list is five or six. There is **no fixed count and no mandatory "middle" act**; the only sin is dumping everything up front.
- **End on a held read.** Once the content has resolved it holds and reads — **prefer stillness to bad motion**: no forced camera drift, no lazy breathing, no back-half pan/push; at most a subtle jitter keeps it alive (→ `motion-language.md`). On a short shot the final reveal and the hold are the **same window** — the hold is not a separate mandatory act. Only the final frame has a real exit; every other frame's exit is the harness transition (story's `transition_in`).
- A **deliberately held** frame — content already revealed, now reading still — is legitimate and often right (a climax, a breather). The failure is never "too still"; it is **front-loaded-then-frozen** (everything dumped by ~25%, nothing cued to the VO). Place held beats deliberately for rhythm so the video isn't uniformly busy (allocate them in `## Video direction`). Reveal pacing + holds + the idle budget → `motion-language.md`.

## Pick the shape — instantiate a blueprint

Don't invent each shot from scratch. The frame's **role** (its `type` / `beat`) points to a proven shape:

1. **Match the role to a blueprint.** Open the [`blueprints index`](../../hyperframes-animation/blueprints-index.md), find the frame's role in the **role→blueprint menu**, and pick the blueprint whose intent fits this beat (story may already have named a candidate id — confirm or override it). Read its file under [`../../hyperframes-animation/blueprints/`](../../hyperframes-animation/blueprints/): it is a short, product-agnostic, **time-coded shot template with `[slots]`** and a named **signature move**.

2. **Instantiate its `[slots]` with THIS product's content** — three postures:
   - **Reproduce** — the blueprint fits the beat and your content maps onto its slots cleanly. Fill every `[slot]` with this product's word / asset / stat and follow its Scene timing. Write the resulting Scene lines.
   - **Adapt** — the _structure_ fits but the content / asset-count / surface doesn't (or you want a fresher surface to avoid templating). State **what you keep / what you change** in one line, then write the adapted Scene lines. You may extend or vary; you may **never** drop the **signature move** (drop it and you picked the wrong blueprint), and you keep the reveals **paced to the VO** — never collapse the shape to a single front-loaded dump.
   - **Compose** — no blueprint fits the beat. Build the shot from the **motion vocabulary** in `motion-language.md`: still pace the reveals to the VO across the shot, never fire everything at t=0. Mark it `blueprint: compose`.

3. **Keep the signature move.** Whichever posture, the blueprint's signature move (named in its file) is the spine of the shot — it usually lands on the shot's key reveal. Carry it through.

The blueprint's own Scene lines, motion vocabulary, and `rule mapping` are your raw material; you are choosing a shape and casting this product into it, not copying an engineering spec.

## What you add to each frame

Story-design's `## Frame N` block already carries the narrative + `asset_candidates`. You append the shot. Story's `scene` / `voiceover` / `transition_in` / role fields stay untouched.

```
## Frame 3 — The problem
- scene: a 20-minute timer over a stack of rejected takes   ← refine only if it could read sharper
- voiceover: "…"            ← story's; leave it
- transition_in: crossfade  ← story's; leave it
- type: pain_point          ← story's
- persuasion: Pain agitation
- beat: frustration
- blueprint: dataviz-countup (Adapt)   ← you add: the id you instantiated (or "compose")
- focal: assets/reject-stat.png         ← you add: the hero asset for this beat
- roles: reject-stat = cutout · timer = supporting · backdrop = background (dim ~40%)  ← you add: role per candidate
- sfx: impact-soft, riser               ← you add: the sound the beat wants (fetched + mounted at root; never yours to embed)

Adapt: keep the count-up-ring signature; one stat not three, and the trend chart becomes the rejected-takes count climbing.
Scene 1 (0.0–1.2s): solid backdrop (dim ~40%); a circular progress ring + bold center number seat dead-center, ring sweeps and number counts 0→20 on one heavy ease — Centered template, ~50% of frame. Slow push-in runs underneath.
Scene 2 (1.2–3.4s): as the VO names the count, the camera pushes THROUGH the ring into the rejected-takes stack lower-left; the stack grows beat-by-beat as a reject counter ticks up beside it (the count-up reveals on its spoken cue, not at t=0). Asymmetric 60/40, 3 depth layers.
Scene 3 (3.4–5.0s): land the hero stat card dead-center, accent glow blooms behind it and holds; the stat reads clean and STILL — no continuing push, no breathing (a held beat beats bad motion). The stillness reads against the prior motion.
```

The lightweight tags:

- **`blueprint:`** — the id you instantiated (with `(Reproduce)` / `(Adapt)`), or `compose`. One id per frame.
- **`focal:`** — which existing candidate is the hero of this beat.
- **`roles:`** — each candidate's role: `cutout` (foreground subject, lay text around it) · `background` (full-bleed, dim 30–50%) · `supporting` (secondary). You **consume** the candidates story chose — never add, swap, or drop one (coverage is story's call; if a frame truly has the wrong candidates, flag it back, don't reach into `capture/`).
- **`sfx:`** — name the sound the beat wants (an impact for a slam, a whoosh for a push, a riser into a reveal). The audio script's `fetch-sfx` pass retrieves it and the assembler mounts it at the root — you only **name** it, never embed an `<audio>` element.

**Layout is stated INLINE in each Scene line** — name the template, density, depth, and hierarchy as part of "where it sits" (`Centered, ~50% of frame`, `asymmetric 60/40, 3 depth layers`), drawing on the **Layout** vocabulary below; never write px / scale / shadow recipes (the worker writes those).

**Motion is named INLINE in each Scene line** — name the move from [`motion-language.md`](motion-language.md) (`ring sweeps`, `pushes THROUGH`, `count-up`, `glow blooms`) and let it settle on a long-tail curve (`power3` default — smooth beats bouncy). Never write ease curves / ms / stagger; those resolve in Step 5 from [`../../hyperframes-animation/rules/`](../../hyperframes-animation/rules/).

## Layout — named inline per Scene

State each Scene's layout as part of "where it sits." **If the blueprint already implies a composition** (a ring around a center, stations on a wide canvas, two cards from opposite wings), that wins — describe it directly; the vocabulary below is for **composing freely** or a generic beat, not a menu you must pick from. Never write px / scale / shadow (the worker does). One frame's layout can EVOLVE across its Scenes (Scene 1 centered hero → Scene 2 rearranges to a grid).

- **Framing vocabulary** — centered (hero / climax) · rule-of-thirds · split-screen (comparison) · layered-depth (immersive) · asymmetric 60/40 or 70/30 (editorial) · triptych (three panels) · full-width strip. Vary the framing across the video so it doesn't read as one repeated template — let the beat decide, not a quota.
- **Density** — primary visual ≥ 40% of canvas; ≥ 3 depth layers (background + midground + foreground); never a lone small cluster floating in empty space. Squint test: after blur you can still pick out the #1 element.
- **Hierarchy** — combine ≥ 2 of size (3:1) / weight (800 vs 400) / contrast / position (upper-third is golden) / motion, so one element clearly dominates.
- **Depth** — layer 2–3 of: size, blur, opacity gradient, overlap, shadow-stack.
- **Don't show**: nav bars, footers, scrollbars, real cursors / browser chrome, generic decorative shapes standing in for a real asset, floating bokeh / purple-blue "AI" gradients — unless it's an intentional UI-demo reconstruction.

## Video direction

The whole video shares one look and one motion grammar. Write a **`## Video direction`** block ONCE at the top of `STORYBOARD.md` so every frame inherits it and per-frame Scene lines carry only the **delta**. This block is load-bearing — it is what binds many independent shots into one film. **Keep it.**

- **palette system** — from `frame.md`: which roles map to which hues. Never invent.
- **motion grammar + reveal model** — long-tail eases (`power3` default, smooth over bouncy) + the **VO-paced reveal** model every frame follows (reveal each piece on its spoken cue; never front-load) + what may stay alive during a hold (subtle jitter at most; no lazy breathing) (→ `motion-language.md`).
- **rhythm / held-frame allocation** — name the **held / breather frames** (often before a climax) so the video varies its energy: most frames reveal to the VO, a few hold still (a held read beats bad motion; the anti-monotony discipline; → `motion-language.md`).
- **negative list** — what never appears: off-brand textures / effects the pack forbids, **plus both motion failure modes** — slideshow (front-load then freeze) and screensaver (everything floating independently) (→ `motion-language.md`).

Do **not** repeat these per frame — restating video-level rules in every frame is exactly the bloat this layer prevents.

## Palette and type

- **Palette** — `frame.md` (the adopted pack) is the color truth; apply its roles per frame. Generic basics (one accent, tint neutrals, avoid pure `#000`/`#fff`) → `hyperframes-creative/references/house-style.md`.
- **Type** — fonts resolve via `frame.md`'s type tokens; reference them **by role** (display / body / mono / the pack's ramp), never by raw family or px. Generic typography craft (embedded fonts, dark-bg optical compensation, `tabular-nums`) → `hyperframes-creative/references/typography.md`.

## Caption keep-out

The bottom ~17% of the canvas is reserved for the caption pill. Plan every frame's content into the **top ~83%** so nothing important lands in the band (the worker enforces the pixel cutoff; you plan the layout). Holds even when captions are disabled — bottom-edge consistency.

## Where the detail lives

| For…                                                                      | Read                                                                                         |
| ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| the proven shapes + role→blueprint menu + how to pick                     | [`blueprints-index.md`](../../hyperframes-animation/blueprints-index.md)                      |
| motion — shot model, vocabulary, holds, idle budget, stillness, seek-safe | [`motion-language.md`](motion-language.md)                                                    |
| layout — templates, density, depth, hierarchy, caption band               | the **Layout** vocabulary in this file                                                       |
| concrete eases / ms / stagger + rule recipe bodies (Step 5)               | [`rules/`](../../hyperframes-animation/rules/)                                                |
| palette + type tokens                                                     | the project's `frame.md`; basics → `hyperframes-creative` `house-style.md` / `typography.md` |
| "produced, not generated" foreground density                              | `hyperframes-creative/references/video-composition.md`                                       |
| within-frame cuts / seams (zoom-through · cut-the-curve · waterfall)      | [`cut-catalog.md`](cut-catalog.md)                                                            |
| transitions                                                               | story-design owns `transition_in`; you don't touch it                                        |

## Checklist

- **`## Video direction`** written once at the top (palette · motion grammar + shot model + idle budget · stillness allocation · negative list incl. both failure modes); per-frame entries are deltas, not restatements.
- Every frame is a **time-coded shot sequence** with real second windows across its `duration` — not a tag bag.
- **No frame front-loads** — at t=0 only what the VO is saying enters; each further piece reveals on its spoken cue, across the back ~50%. Window count follows the VO, not a fixed number.
- Every frame names an **`blueprint:`** id (Reproduce / Adapt) or `compose`; an Adapt states keep/change and **keeps the signature move**; nothing collapses to a single front-loaded dump — reveals stay paced to the VO.
- **Held frames are deliberate** — allocated in Video direction for rhythm; a held read is fine (prefer stillness to bad motion), but no frame may be front-loaded-then-frozen.
- Each frame's `asset_candidates` have a `focal` + per-candidate `roles`; none added, swapped, or dropped.
- Layout named **inline** per Scene (template / density / depth / hierarchy — the **Layout** vocabulary here); motion named **inline** per Scene from the vocabulary (`motion-language.md`). No px / ease curves / ms / JS.
- Content planned into the top ~83% (caption band clear).
- Palette / type pulled from `frame.md` by role — nothing invented.
- You wrote no HTML and never read `capture/`.
