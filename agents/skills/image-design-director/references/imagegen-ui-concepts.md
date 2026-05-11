# Image Gen UI Concept Guidance

Source note: adapted for personal use from OpenAI's `build-web-apps` plugin, especially `plugins/build-web-apps/skills/frontend-app-builder/references/imagegen-website-concepts.md`.

Use this reference with the installed `imagegen` skill when creating visual concepts for websites, web apps, dashboards, tools, mobile apps, desktop apps, native interfaces, game screens, and other digital UI surfaces. This is guidance, not a prompt template. Write a natural design-director brief tailored to the task.

## Must Include

Copy concrete details from the user request, screenshots, existing app, or plan. Do not reduce them to a generic category like "modern landing page" or "clean dashboard."

- Scope: complete page, complete app screen, multi-state product surface, dashboard, mobile screen, desktop/native interface, game screen, or coordinated section/state concepts
- Purpose and audience: what the page or product helps the user do, and who it is for
- Exact visible content: headlines, labels, CTAs, section names, nav items, table fields, sample entities, dates, prices, statuses, media requirements, and required copy
- Structure: first viewport composition, downstream section order, sidebars, rails, drawers, grids, tables, charts, media areas, forms, footer/status regions, and responsive continuation
- Interaction model: selected state, hover/focus affordances, filters, tabs, mode switches, creation/editing flow, success state, playback state, game HUD, or other local-state behavior the implementation should support
- Visual system: palette mood, typography personality, content text scale, UI chrome/control text scale, density, spacing rhythm, radii, shadows, borders, container model, card usage, icon style, image treatment, brand mark direction, and reference style
- Implementation constraints: code-native app UI text and controls, fully rendered product/background assets with their own text and branding when appropriate, separable assets, reusable component families, clear design-system tokens, accessible/responsive layout, and practical design handoff
- Negative constraints: no header-only crops for full-surface work, no extra product areas, no fake metrics, no decorative filler, no default card grids, no hero eyebrow/kicker/pretitle/badge/pill above the main heading unless explicitly requested or present in the reference, no gradients that conflict with the design direction, no pasted-in images that fail to blend with the background, no unrelated sections, no new claims, and no moving true app UI text into images

## Quality Bar

Every concept should feel like a professional product mockup by a senior product designer:

- Clean, airy, distinctive, and not repetitive by default
- One clear creative idea or visual point of view
- Full requested surface, not just a hero, unless the user only asked for a hero
- Strong first viewport with clear offer, product signal, or primary task
- Coherent rhythm across sections, states, and responsive continuation, without repetitive card stacks or repeated formulas
- Cohesive flow through shared spacing, palette, type rhythm, media treatment, and subtle transitions
- Excellent typography and intentional whitespace, including buttons, tabs, inputs, sidebars, table cells, labels, and other control chrome
- Fewer, stronger visual elements instead of dense illustration, iconography, decorative widgets, or complex UI chrome
- Consistent palette, gradients, spacing, components, icon style, imagery, shadows, borders, and container model
- Icon fidelity when icons are present: preserve metaphor, stroke weight, filled vs outline style, corner shape, size, color, alignment, spacing, and states
- Color fidelity: preserve the generated design's actual background, surface, text, border, shadow, and accent colors
- Clear design-system signal: typography scale, control text styles, reusable component families, variants, spacing rhythm, and tokens that can be extracted before coding
- High-quality assets for logos, brand marks, hero imagery, product renders, background scenes, illustrations, textures, thumbnails, posters, avatars, empty states, game sprites, and UI-adjacent visual objects
- Purposeful motion or interaction cues that can be implemented later
- Specific, non-generic copy when the user has not supplied exact copy

Default to roughly 7/10 creativity: distinctive and art-directed, but still implementable. Clean means airy, edited, legible, and not cluttered.

Avoid unnecessary cards, hero eyebrow/kicker labels, pills, badges, stats, icon rows, excessive illustrations, decorative iconography, overcomplicated header UI, fake charts, fake metrics, fake jargon, generic brand names, bokeh/orb decoration, neon grids, excessive glow, mismatched gradients, pasted-looking images, unreadable text, and filling whitespace just because it exists.

## Visual Direction Defaults

Before writing the Image Gen brief, choose a compact visual direction that fits the product and audience. Do not expose this as a rigid template; use it to make the brief more specific.

- Use roughly 7/10 creativity, low-to-medium visual density, generous spacing, high implementation clarity, strong typography discipline, and image-led moments when real visuals improve the UI
- Choose one theme paradigm, one background character, one typography character, one primary-screen architecture, one rhythm, 2-4 signature component motifs, and 1-2 motion cues
- Keep every generated section/state/detail concept inside the same design world
- For first viewports, ask for one clear focal point, a short headline or primary task, restrained supporting copy, a visible primary CTA/control, and enough breathing room for a small laptop viewport
- Keep headers quiet by default: brand mark, essential navigation, and one primary action or control
- Prefer one or two high-quality image or illustration moments over many small decorative visuals
- Use iconography only when it clarifies navigation, controls, or product meaning
- Prefer open layouts, strong bands, rails, lists, tables, canvases, and purposeful single frames over nested cards, giant rounded section wrappers, default bento grids, or overcompartmentalized dashboards
- Vary rhythm across long surfaces: density, image-to-text ratio, alignment, scale, whitespace, and visual tempo should change deliberately while preserving one system
- Use implementation-friendly media frames with stable aspect ratios, consistent crop/radius/shadow logic, and clear placement
- Let small labels, utility pills, pseudo-system markers, fake metrics, and decorative dashboard jargon appear only when they communicate real structure or product meaning

## Image Count and Clarity

Readability and extraction quality outrank compact presentation.

- For a one-section request, generate one primary section concept
- For 2-10 requested or implied sections, use coordinated primary section concepts, one fresh image per major section, when that improves readability
- Use an optional full-surface overview only for rhythm, section order, and transition logic
- Do not treat a compressed overview as the only implementation spec if it makes text, buttons, cards, or spacing unclear
- Generate extra extraction-oriented detail concepts when text, buttons, card anatomy, pricing/testimonial details, typography, palette, image treatment, or spacing is not readable
- For dashboards, tools, editors, and dense app screens, generate the full primary screen plus focused state/detail concepts for sidebars, tables, inspector panels, modals, charts, toolbars, forms, and selected states
- Never crop, slice, zoom, or reuse part of an older full-page image as the main section/detail reference
- Do not reduce image count for convenience when doing so makes later implementation rely on guesswork

## Surface Guidance

Full page or app:

- Ask for enough structure to implement the whole requested surface: first viewport, rhythm, product/workflow anatomy, downstream sections or states, and responsive continuation
- Marketing/product pages need a strong hero and clear CTA before proof or feature density
- Do not put an eyebrow, kicker, pretitle, badge, or pill above the hero heading unless the user asked for it or the reference already uses it
- Use interactive hero UI only for SaaS/software previews, product demos, or purposeful interactive animation
- App screens, dashboards, and tools need the real interaction model: sidebars, panels, tables, timelines, charts, controls, modes, selected states, and primary workflow
- Complex app screens, dashboards, editors, and tools should be clear enough to break into real components: app shell, navigation, major feature regions, reusable controls, table/chart/form modules, sample data/state boundaries, and responsive layout behavior
- Games need the play surface, HUD/control placement, art direction, reward/hazard language, interaction affordances, and a follow-on asset pass for sprites, tiles/platforms, collectibles, hazards, goals, props, and background/parallax layers

Redesign from screenshot:

- Use the screenshot as the edit target when preserving information architecture matters
- Preserve navigation meaning, product/brand cues, content hierarchy, controls, and page purpose
- Improve spacing, typography, visual hierarchy, color, image treatment, and component polish without inventing unrelated sections, fake metrics, new claims, or new product areas

Hero or section:

- Use this path when the user asks for a section, hero, pricing block, feature section, or another page slice
- Include surrounding context and enough visual language to continue the page consistently

Content-heavy pages:

- For multi-section websites and long landing pages, prefer coordinated section concepts with one fresh, large, readable image per major section
- Avoid one huge full-page screenshot when it weakens detail, hierarchy, or implementation matching
- Keep one accepted layout concept responsible for overall structure and section order
- All supporting concepts must share brand language, typography, palette, component geometry, asset style, spacing, and density

Native, mobile, and desktop UI:

- Respect platform density, navigation patterns, chrome, touch or pointer targets, and safe areas/window regions
- Keep controls and labels code-native in later implementation
- Ask for enough state detail to understand selected rows, active tabs, modal/sheet behavior, toolbar controls, empty states, and responsive or adaptive layout

## Asset Planning

- Keep real app UI text, form fields, nav, metrics, and controls in code
- Product images, background assets, posters, packaging, signage, hero photos, and brand scenes should be rendered completely by Image Gen with text, logos, marks, labels, and branding that belong in the asset
- Quote exact asset text and require verbatim rendering when text matters
- If the concept includes a logo, brand mark, product label, package, poster, sign, product render, or branded background object, use Image Gen editing to create standalone matching assets before coding
- Request transparent backgrounds or clean cutouts when assets need to layer into code-native UI
- For games, generate transparent character/state sprites or sprite sheets, terrain/platform tiles, collectibles, hazards, goal/checkpoint objects, props, and 2-3 parallax/background layers when the concept calls for depth
- Keep game HUD text, score, controls, collision boxes, physics, and game state code-native
- Use generated assets for logos, brand marks, hero imagery, product renders, editorial imagery, background scenes, cutouts, textures, posters, thumbnails, avatars, empty-state art, and illustrated objects
- Do not crop a full-page concept into production UI as a shortcut
- SVG is fine for faithful icons. Use Image Gen for logos, brand marks, and non-icon visual assets
- Supporting asset concepts must match the accepted layout concept and must not introduce a new visual direction

## After Generation

- Reject concepts that are header-only for full-surface asks, cluttered, generic, repetitive, under-specified, unreadable, over-decorated, or impractical to implement
- For every generated section/state/detail image, extract the section purpose, visual priority, readable text, typography relationships, spacing, button/control styling, component/container logic, colors, image treatment, and unclear details
- Extract an icon inventory before coding: every visible icon, glyph, chevron, logo-like mark, toolbar symbol, status symbol, and empty-state symbol, including meaning, outline vs filled style, stroke width, size, color, container, alignment, spacing, and state treatment
- If any required detail is still unclear, generate a new standalone section/state/detail image before coding
- Extract a design system before coding: native aspect, layout, section order, copy, nav, CTAs, palette, spacing scale, content typography, UI chrome typography, reusable component families, variants, container model, assets, state, and responsive continuation
- Identify whether each background is true white, off-white, cream, gray, dark, or tinted
- Treat the accepted concept as the visual spec. Match composition, hierarchy, palette, gradients, typography, spacing, imagery, components, container model, and asset treatment
- Do not reinterpret palette for taste or replace white backgrounds with cream/off-white
- Do not add a color overlay, tint, or translucent wash over a hero image unless the accepted concept clearly has one
- Do not substitute generic nearby icons for the accepted design's icons
- Preserve the accepted container model. Do not add cards, floating panels, bordered tiles, or card grids where the spec uses open layout, bands, lists, tables, rails, canvases, or full-bleed composition

## Fidelity Checks When Coding

Use these checks only when implementing code from an accepted generated concept.

- Verify one section or contiguous viewport at a time for multi-section pages
- Compare the browser or platform screenshot to the relevant concept image, not only to a full overview
- Build an allowed above-the-fold copy list from the accepted concept and user-provided copy
- Do not add new hero, nav, eyebrow/kicker, CTA, label, subtitle, category, or proof text unless it is recorded as an intentional deviation
- Use `view_image` on both the accepted concept and latest rendered screenshot in the same QA pass before final handoff
- Capture at the accepted concept's native dimensions when practical; otherwise record the blocker and verify the current viewport
- Write a fidelity ledger: mismatch, concept evidence, render evidence, and fix made or reason not fixed
- Inspect at least five concrete comparison points before claiming fidelity: copy, nav, section order, layout, typography, palette, gradients, spacing, borders, radii, container model, asset blending, icon treatment, motion, or interaction states
- Keep iterating if a skilled design review would call out fixable visual drift
- Remove temporary QA screenshots, reports, scratch notes, and unused generated assets unless the task requires keeping them
