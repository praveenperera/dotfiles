# Frameworks and package bindings

Use this reference before choosing a web framework, router, or third-party binding. The ecosystem
observations were checked on 2026-07-27; verify package freshness and compatibility again before
installation.

## Framework choices

| Need | Recommended boundary |
| --- | --- |
| interactive SPA, dashboard, form, or local tool | Vite + ReScript React |
| small client-side router | `RescriptReactRouter` |
| content-first site with interactive sections | Astro/TypeScript shell + ReScript React islands |
| Svelte or SvelteKit application | TypeScript in `.svelte`; optional ReScript domain modules |
| Next.js application | ReScript React behind thin framework adapters where needed |
| static page without meaningful behavior | HTML/CSS without ReScript |

React is the strongest pairing because `@rescript/react` is official, supports modern React, and
provides hooks, JSX, event types, server rendering APIs, and a small router.

The official project generator supports Vite + React. Prefer that combination when a
metaframework provides no concrete benefit.

## Astro

Astro officially renders and hydrates React components as islands, but `.astro` component scripts
and templates use JavaScript/TypeScript and Astro syntax. Keep pages, layouts, endpoints, and
Astro-specific code in TypeScript. Put substantial interactive React UI and shared domain logic in
ReScript.

Do not treat Astro configuration or development middleware as an automatic logic exemption. When
production and development need the same parser, route classifier, or validation policy, compile
that owner before Astro loads and import the generated module from the configuration instead of
maintaining a second JavaScript implementation.

Generated ReScript modules usually expose a React component as a named `make` export. In an
existing production site, follow the established ReScript/React island pattern and use the normal
project checks; do not create a separate compatibility probe. In a new site, integrate the first
real island and let the normal build verify it. Add a tiny `.tsx` wrapper only if the actual
integration exposes a renderer or export-shape failure, and keep it free of domain logic.

Keep one top-level `@react.component` per hydrated island file. Nested component-module exports can
typecheck while Astro's production build fails to match the requested export. Treat scripts
embedded in `.astro` as application code to inspect: keep a synchronous pre-paint bootstrap inline
when timing requires it, but move reusable state, policy, or DOM plans into ReScript.

Production compilation does not verify hydration timing or browser-only effects. Browser-test
each hydration directive the site relies on, including:

- server rendering without evaluating `window`, `document`, storage, or media-query globals at
  module initialization
- initial deep links and URL hashes before an island hydrates
- persistence restoration and writes
- external events such as media-query, storage, hash, and history changes

Choose `client:load` when initial URL or persisted state must be applied immediately. Use a lazier
directive only when the resulting pre-hydration behavior is acceptable.

Inspect the actual island wrapper before trusting `client:visible`. Astro commonly renders an
island wrapper with `display: contents`; a zero-sized wrapper may never intersect even when its
child is visible. Verify that the `ssr` attribute disappears and the intended interaction works
in a production preview.

Give static fallbacks and hydrated island renderers distinct ID namespaces. If no-JavaScript deep
links must retain the canonical target, put that target in the `noscript` fallback rather than
emitting the same ID in both live and static markup.

Astro adds little value when the page is one fully hydrated application. Use Vite directly in
that case.

## Svelte

Svelte's compiler and language tooling support JavaScript and TypeScript in `.svelte` files.
There is no maintained first-class ReScript component integration comparable to
`@rescript/react`; the ReScript/Svelte work found during research was proof-of-concept quality.

Keep Svelte components in TypeScript. Use ReScript only for sufficiently valuable domain or data
modules, then import their compiled JavaScript. Do not create or maintain a custom Svelte
preprocessor merely to make the stack language-pure.

## Next.js

ReScript React components work, but Next-specific file conventions, route handlers, server/client
directives, fonts, metadata, and default exports may require ReScript externals or thin
JavaScript/TypeScript wrappers. Use Next only when its routing, rendering, or deployment model is
required. Prefer Vite for local tools and client-only applications.

## Package-binding reality

The official package index has a small official core and broad community coverage. Bindings exist
for many established areas - React Native, Relay, MUI, Jotai, RxJS, Firebase, Supabase, Stripe,
date libraries, and test tools - but existence does not imply current compatibility.

Before installing a binding:

1. compare its supported ReScript version with the project
2. compare its supported upstream major with the installed JavaScript package
3. inspect its last release, open compatibility issues, source, and `.resi` surface
4. confirm it models only real runtime behavior and does not rely on unsafe casts
5. compile a minimal usage before designing the feature around it

Prefer a narrow local binding when only a few stable exports are needed. Prefer a TypeScript or
JavaScript adapter when the library is fast-moving, deeply generic, callback-overloaded, or has a
large configuration surface. Expose domain values rather than the third-party API across that
adapter.

Bind stable reusable primitives such as Card, Button, Accordion, and icons once, then reassess
their consumers. Ordinary JSX and compound-component consumers often work directly in ReScript.
Keep narrow adapters for APIs such as chart configuration objects or compound tooltip triggers
whose render props and generic callbacks materially dominate the component.

## Router

`RescriptReactRouter` is included with ReScript React. It exposes URL path segments, hash, and
search values and leaves route matching to exhaustive ReScript pattern matching. Use it for
simple client routing. Choose a larger router only when nested data loading, route-owned pending
states, or framework integration justifies another binding.

## Sources

- https://rescript-lang.org/packages/
- https://rescript-lang.org/docs/manual/installation/
- https://rescript-lang.org/docs/react/introduction/
- https://rescript-lang.org/docs/react/router/
- https://docs.astro.build/en/guides/framework-components/
- https://docs.astro.build/en/basics/astro-components/
- https://svelte.dev/docs/typescript/
