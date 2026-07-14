# Transition Catalog

Hard rules, scene template, and routing to implementation code. Read the reference file for the transition type you need — don't load all of them.

## Contents

- Hard rules for CSS transitions
- Shader transitions
- Scene template
- CSS transition examples
- Shader transition routing

## Hard Rules (CSS)

These cause real bugs if violated.

**Scene visibility:** Scene 1 visible by default (no `opacity: 0`). Scenes 2+ have `opacity: 0` on the CONTAINER div. GSAP reveals them. No visibility shim (`timedEls`).

**Fonts:** Just write the `font-family` you want — the compiler embeds supported fonts automatically via `@font-face` with inline data URIs. No need for `<link>` tags or `@import`. Works in all contexts including sandboxed iframes.

**Element structure:** No `class="clip"` on scene divs in standalone compositions. Only the root div gets `data-composition-id`/`data-start`/`data-duration`.

**Overlay elements:** Staggered blocks = full-screen 1920x1080, NOT thin strips. Glitch RGB overlays = normal blending at 35% opacity, NOT `mix-blend-mode: multiply` (invisible on dark backgrounds). Light leak overlays = larger than the frame (2400px+), never a visible shape. Overexposure = use `filter: brightness()` on the scene, not just a white overlay.

**VHS tape:** Clone actual scene content with `cloneNode(true)`, NOT colored bars. Each strip: wider than frame (2020px at left:-50px). Red+blue chromatic copies at z-index above main strip. Seeded PRNG for deterministic random offsets.

**Z-index:** Gravity drop, zoom out, diagonal split need outgoing scene ON TOP (`zIndex: 10`) so it exits while revealing the new scene behind (`zIndex: 1`).

**Page burn:** Content burns with the page — no falling debris. Hide scene1 via `tl.set` at burn end, NEVER `onComplete` (not reversible). `onUpdate` must restore `clipPath: "none"` when `wp <= 0` for rewind support. Incoming scene fades from black at 90% through burn.

**Clock wipe:** 9-point polygon with intermediate edge positions. Step through 4 quadrants with separate tweens.

**Grid dissolve:** Cycle 5 palette colors per cell, not monochrome.

**Blinds count by energy:** Calm: 4h/6v. Medium: 6-8h/8v. High: 12-16h/16v.

**Don't use:** Star iris (polygon interpolation broken), tilt-shift (no selective CSS blur), lens flare (visible shape, not optical), hinge/door (distorts too fast).

## Shader Transitions

Shader setup, WebGL initialization, capture, and fragment shaders are handled by `@hyperframes/shader-transitions`. Inspect the installed package for API details. Compositions using shaders must follow the [shader-compatible CSS rules](overview.md#shader-compatible-css-rules).

## Scene Template

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <script src="https://cdn.jsdelivr.net/npm/gsap@3.14.2/dist/gsap.min.js"></script>
    <style>
      body {
        margin: 0;
        width: 1920px;
        height: 1080px;
        overflow: hidden;
        background: #000;
        font-family: "YOUR FONT", sans-serif; /* compiler embeds supported fonts automatically */
      }
      .scene {
        position: absolute;
        top: 0;
        left: 0;
        width: 1920px;
        height: 1080px;
        overflow: hidden;
      }
      #scene1 {
        z-index: 1;
        background: #color;
      }
      #scene2 {
        z-index: 2;
        background: #color;
        opacity: 0;
      }
    </style>
  </head>
  <body>
    <div
      id="root"
      data-composition-id="main"
      data-width="1920"
      data-height="1080"
      data-start="0"
      data-duration="TOTAL"
    >
      <div id="scene1" class="scene"><!-- visible --></div>
      <div id="scene2" class="scene"><!-- hidden --></div>
    </div>
    <script>
      window.__timelines = window.__timelines || {};
      var tl = gsap.timeline({ paused: true });
      // Transition code here
      window.__timelines["main"] = tl;
    </script>
  </body>
</html>
```

Every transition follows: position new scene → animate outgoing → swap → animate incoming → clean up overlays.

## CSS Transitions

All code examples use `old` for the outgoing scene-inner selector and `new` for the incoming, with `T` as the transition start time. Read the reference file for the type you need.

| Type           | Transitions                                          | Reference                        |
| -------------- | ---------------------------------------------------- | -------------------------------- |
| Push           | Push slide, vertical push, elastic push, squeeze     | [Push](css-push.md)               |
| Radial / Shape | Circle iris, diamond iris, diagonal split            | [Radial](css-radial.md)           |
| 3D             | 3D card flip                                         | [3D](css-3d.md)                   |
| Scale / Zoom   | Zoom through, zoom out                               | [Scale](css-scale.md)             |
| Dissolve       | Crossfade, blur crossfade, focus pull, color dip     | [Dissolve](css-dissolve.md)       |
| Cover          | Staggered blocks, horizontal blinds, vertical blinds | [Cover](css-cover.md)             |
| Light          | Light leak, overexposure burn, film burn             | [Light](css-light.md)             |
| Distortion     | Glitch, chromatic aberration, ripple, VHS tape       | [Distortion](css-distortion.md)   |
| Mechanical     | Shutter, clock wipe                                  | [Mechanical](css-mechanical.md)   |
| Grid           | Grid dissolve                                        | [Grid](css-grid.md)               |
| Other          | Gravity drop, morph circle                           | [Other](css-other.md)             |
| Blur           | Blur through, directional blur                       | [Blur](css-blur.md)               |
| Destruction    | Page burn                                            | [Destruction](css-destruction.md) |

## Shader Transitions

WebGL shader transitions are provided by `@hyperframes/shader-transitions`. The package handles setup, capture, WebGL initialization, the render loop, and GSAP integration. Inspect the installed package for available shaders and APIs; do not copy raw GLSL manually.

The built-ins are not a ceiling. For an effect no built-in covers, you can write custom GLSL from scratch, adapt shader code found online (ShaderToy, GLSL Sandbox, GitHub), or build a custom CSS transition that fits no existing category — combine clip-path, transforms, and filters in new ways. If the storyboard calls for an effect that doesn't exist yet, build it; the framework renders anything a browser can run.
