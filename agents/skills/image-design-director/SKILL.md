---
name: image-design-director
description: Use only when the user explicitly invokes $image-design-director or explicitly asks to use Image Gen/image-gen for beautiful UI design, UI concepts, visual directions, interface mockups, app or website redesign concepts, dashboard concepts, mobile/desktop/native UI concepts, game screen concepts, or design-system extraction from generated UI concepts. Do not use for routine frontend implementation, normal UI edits, or broad app-building requests unless Image Gen or this skill is explicitly requested.
---

# Image Design Director

Use this skill to direct Image Gen toward polished digital UI concepts and then turn the accepted concept into a practical design handoff. Keep Image Gen usage deliberate: this skill exists to improve visual direction, not to spend image generations accidentally.

## Trigger and Quota Rules

- Use this skill only after explicit invocation with `$image-design-director` or an explicit request to use Image Gen/image-gen for UI design.
- Do not trigger this skill for ordinary frontend tasks such as building an app, fixing spacing, refactoring components, or restyling a screen when the user did not ask for Image Gen.
- If image generation is not clearly authorized, draft the Image Gen brief, concept criteria, and planned image count without calling the image tool.
- Generate one strong concept by default when generation is authorized and the user did not request alternatives.
- Before generating more than one image, state the planned count and wait for approval.
- If the user asks to explore concepts without giving a number, ask for the exact count before calling Image Gen.
- In Plan Mode, do not call Image Gen unless the user explicitly asks to generate now or explicitly invokes this skill for concept generation.

## Core Workflow

1. Gather the concrete UI requirements from the request, existing app, screenshots, plans, or code.
2. Read `references/imagegen-ui-concepts.md` when writing briefs, judging concept quality, planning assets, or extracting a design spec.
3. Use the installed `imagegen` skill/tool for authorized image generation. Follow its save-path, editing, transparency, and fallback rules.
4. Write a natural design-director brief, not a generic template. Include the requested surface, audience, exact visible content, structure, interaction model, visual system, implementation constraints, and negative constraints.
5. Generate the complete requested surface. For multi-section pages, dense apps, tools, dashboards, native interfaces, or game screens, use separate section/state/detail concepts when a single image would make text, controls, spacing, or assets unreadable.
6. Reject weak concepts before handoff: header-only concepts for full surfaces, cluttered or generic layouts, unreadable text, impractical assets, filler metrics, repetitive card stacks, and visual systems that cannot be implemented cleanly.
7. Ask the user to approve or choose a concept before treating it as the source of truth.
8. Extract a handoff spec from the accepted concept: visible copy, layout, palette, typography, spacing, component families, icon treatment, asset needs, interaction states, responsive behavior, and unresolved details.

## Concepting Standards

- Aim for clean, airy, distinctive, senior-product-designer quality UI with about 7/10 creativity: visually directed, readable, and buildable.
- Preserve information architecture from the user, screenshots, or existing product. Do not invent unrelated sections, fake claims, extra product areas, or decorative data.
- Keep true app UI text and controls code-native in later implementation. Product renders, posters, packaging, signs, branded scenes, and background assets may contain their own raster text or branding.
- Prefer strong typography, clear hierarchy, intentional whitespace, implementation-friendly media frames, and a coherent container model over decorative filler.
- Avoid default bento/card grids, hero eyebrows/kickers/pretitle labels, badges, pills, fake metrics, icon rows, bokeh/orb decoration, excessive glow, and generic stock-like assets unless explicitly requested or present in the reference.
- For assets that need to be reused in implementation, plan standalone Image Gen asset passes instead of cropping full UI concepts.

## Handoff and Implementation Guidance

- Treat an accepted concept as a visual spec, not a mood board.
- Do not reinterpret palette, typography, spacing, hierarchy, container model, icons, or image treatment for taste after approval.
- If implementation happens in the same task, follow the repo's existing framework and conventions. Keep this skill focused on visual fidelity and design-system extraction, not framework selection.
- Implement from extracted tokens and reusable component families so repeated UI elements remain consistent.
- Preserve the accepted concept's white/off-white/dark/tinted background choice exactly.
- Do not add color overlays, decorative gradients, cards, floating panels, or visible copy that the accepted design did not show unless the user approves the deviation.
- When icon fidelity matters, match metaphor, filled vs outline style, stroke weight, size, color, alignment, spacing, and states. Use existing icon sets only when they match the concept.

## Visual QA When Coding

Use strict visual QA only when code is implemented from a generated concept.

- Verify the rendered UI in a browser or platform-appropriate preview, including desktop and a mobile or narrow viewport when relevant.
- Compare the latest rendered screenshot with the accepted concept using `view_image` on both in the same QA pass.
- Check at least five concrete comparison points: copy, layout, typography, palette, spacing, asset treatment, icon treatment, responsive behavior, and interaction state.
- Write a short fidelity ledger before final handoff: mismatch, concept evidence, render evidence, and fix made or reason not fixed.
- Keep iterating on visible drift until the implementation is faithful enough for design sign-off or a concrete blocker remains.
- Remove temporary QA artifacts unless the task explicitly asks to keep them.
