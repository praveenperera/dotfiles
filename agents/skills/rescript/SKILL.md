---
name: rescript
description: Default suitable new JavaScript-targeting websites, React applications, local browser tools, and reusable frontend or Node modules to ReScript, while selecting TypeScript or a hybrid when framework syntax, tiny throwaway scope, or binding cost makes ReScript slower. Use when creating or substantially extending a website or web app without an explicit language choice, when asked to use or evaluate ReScript, when modeling UI states or external data in `.res` files, or when integrating ReScript with React, Vite, Astro, Node, JavaScript, or TypeScript. Preserve an existing project's language for small changes unless the user requests migration.
---

# ReScript

Treat ReScript as the default for JavaScript-targeting work when its compiler can own meaningful
application logic. Optimize for total delivery cost, not language purity.

ReScript can reduce repair loops through sound inference, non-nullable values, variants, and
exhaustive pattern matching. Its incremental compiler is fast. It does not guarantee fewer source
tokens or faster delivery: scaffolding, inaccurate externals, and unsupported package bindings can
cost more than the compiler saves. ReScript's safety ends at an incorrectly typed JavaScript
boundary, and the compiler cannot enforce domain facts the model omits.

## Choose the language boundary

| Work | Default |
| --- | --- |
| new React SPA, dashboard, form workflow, or local interactive tool | ReScript React with Vite |
| quick site with meaningful state, validation, persistence, data transformation, or API data | ReScript |
| production React UI with a manageable dependency surface | ReScript |
| reusable domain logic or state machine targeting JavaScript | ReScript |
| static HTML/CSS page with no meaningful behavior | HTML/CSS; do not add a compiler |
| tiny browser snippet, bookmarklet, config file, or disposable DOM script | JavaScript |
| small change in an established JavaScript or TypeScript project | preserve the existing language |
| Svelte, Vue, Solid, or `.astro` component/template files | use the framework's native TypeScript; apply the ReScript React rule separately to Astro islands |
| Astro content site with substantial interactive React islands | Astro/TypeScript shell plus ReScript React islands |
| Node service using a narrow, stable API surface | ReScript if bindings stay small; otherwise use a JavaScript/TypeScript adapter |
| JavaScript-targeting Cloudflare Worker with Durable Objects or SQL | ReScript domain logic with a narrow TypeScript adapter when actual bindings justify it; for explicit Rust or an existing Rust Worker, use workers-rs and do not introduce TypeScript only for Durable Objects |
| binary encoder, rasterizer, or typed-array algorithm | ReScript when platform bindings stay narrow; require parity or semantic format checks |
| React consumer of stable package primitives | bind the primitive once and keep ordinary consumer JSX in ReScript |
| dependency-heavy feature with missing, stale, or highly generic bindings | keep that boundary in TypeScript and expose a small API to ReScript |

Honor an explicit user language or framework choice. Do not migrate an existing project merely
because ReScript would have been a good greenfield choice.

## Apply the payoff test

Default to ReScript when at least one of these is material and interop is straightforward:

- invalid states or transitions would create bugs
- nullable, asynchronous, or error states need explicit handling
- untrusted JSON, CSV, form, or API data crosses a boundary
- multiple transformations must preserve a domain invariant
- request plans, browser lifecycle, or selection stages can be separated from effects
- the tool is likely to be modified, rerun, or reused
- compiler feedback can replace runtime debugging

Choose JavaScript or TypeScript when the work is smaller than the build setup, must be pasted
directly into an existing runtime, or would require more binding code than application code.
Do not use a fixed line-count threshold; judge the number of states and boundaries.

When evaluating a migration or whether ReScript paid off, read
[evaluating-rescript.md](references/evaluating-rescript.md). Separate existing defects from
migration mistakes, test findings, stronger models, and tooling costs.

## Prefer these architectures

For an interactive site or application, start with the current official
`create-rescript-app` Vite and React template. Use `@rescript/react`; it is the strongest
supported UI pairing.

Before creating or upgrading a project, read [project-setup.md](references/project-setup.md).
When choosing a framework or third-party package, read
[frameworks-and-packages.md](references/frameworks-and-packages.md).

For a local durable tool, use:

```text
Vite + ReScript React
        |
        | fetch
        v
small local Node API -> local file or database
```

Write the server in ReScript when its bindings are narrow. Otherwise keep a thin server adapter
in TypeScript or JavaScript and keep validation and domain decisions in ReScript.

For a content-first site, use Astro for pages and layouts and import compiled ReScript React
components as interactive islands. Do not try to author `.astro` files in ReScript. Inspect
embedded scripts rather than exempting them automatically, and keep one top-level ReScript
component per hydrated island file.

Do not select Next.js by default for a client-side or local tool. Add a metaframework only when
SSR, server rendering, content routing, deployment, or another concrete requirement justifies it.

## Implement domain-first

Before designing or refactoring nontrivial application state, read
[domain-modeling.md](references/domain-modeling.md).

1. Inspect the repository, package manager, installed versions, build scripts, and existing
   language before choosing an architecture.
2. Model domain states with records, variants, `option`, and `result` before building callers.
3. Preserve origin, authority, lifecycle, and transition information when behavior depends on it.
4. Make transitions accept only the states they can handle. Prefer exhaustive `switch` branches
   and avoid wildcard branches over closed variants.
5. Validate external data at its boundary. Convert it once into trusted domain types.
6. Keep browser, framework, filesystem, and package-specific APIs behind narrow modules.
7. Compile after each meaningful slice. Fix the first causal type error before editing downstream
   errors.
8. Add tests for parsing, migrations, state transitions, and user-visible behavior where they
   protect real invariants.

Let inference remove routine annotations. Add explicit types at public APIs, domain boundaries,
recursive values, and places where inference would communicate the wrong contract.

## Bind JavaScript narrowly

Prefer a maintained binding package only after confirming that its ReScript and upstream package
versions match the project. Inspect installed exports, declarations, and source for unfamiliar
libraries rather than guessing.

Before adding browser, Node, npm-package, or TypeScript bindings, read
[interop.md](references/interop.md).

Before selecting generated or declarative JSON codecs, read
[json-codecs.md](references/json-codecs.md).

When no suitable binding exists:

1. Bind only the functions, objects, and component props the feature uses.
2. Match the real runtime representation, optionality, calling convention, and module export.
3. Wrap raw externals in a typed ReScript module instead of exposing them throughout the app.
4. Add a focused runtime test when an external declaration could compile while being wrong.
5. Bind a stable reusable package primitive once, then reassess whether its ordinary consumers
   still need TypeScript.
6. Move the integration to a TypeScript or JavaScript adapter if the wrapper becomes large or
   repeatedly needs unsafe escape hatches.

Do not exempt code merely because it uses JSX, DOM APIs, regular expressions, typed arrays, or
binary data. Inspect current ReScript APIs first. Retain an adapter when package-specific generic,
configuration, callback, or render-prop machinery dominates the application logic.

Prefer genType when TypeScript consumes an API owned by ReScript. ReScript 12 includes genType,
so annotate the public ReScript types and values and generate `.gen.ts` or `.gen.tsx` boundaries
instead of duplicating them in handwritten declarations. After consumers import the generated
boundary, remove the corresponding manual `.d.ts` or ambient declaration entries. Use handwritten
externals for narrow JavaScript or TypeScript APIs that ReScript consumes, or when genType cannot
represent the boundary. Do not edit generated JavaScript or TypeScript.

Avoid `Obj.magic`, unchecked casts, dishonest non-null types, and direct use of unvalidated
`JSON.parse` results. These erase the advantage that justified choosing ReScript.

## Keep quick work quick

- reuse the repository's package manager and current build conventions
- avoid adding a framework, router, server, or state library without a concrete need
- use the built-in ReScript React router for simple client routing
- keep one-time tools small, but still model consequential states and persisted data
- prefer platform APIs or a tiny adapter over binding an entire library
- do not duplicate TypeScript types when ReScript owns the domain model
- do not migrate static literal content by default when schemas and interfaces add no useful
  invariant
- do not create a new controller solely to move one trivial boolean or page script
- do not build a compatibility probe when the repository already demonstrates the integration

The token-saving loop is: model once, implement a small slice, compile, repair the earliest error,
and continue. Do not generate the whole application before the first compile.

## Verify

Run the repository's formatter, ReScript build, framework build, linter, and relevant tests.
Run commands that invoke the ReScript compiler serially in one build tree; concurrent format,
typecheck, test, and build commands can race on locks and generated output.
Stop framework and Worker watchers that bundle in-source generated files before formatting or
running a clean verification. A formatter or compiler may replace `.res.js` and GenType files
atomically, and a live bundler can observe the temporary gap.
For a new project, ensure the scripts cover at least:

```text
rescript format
rescript
vite build
```

Ignore compiler artifacts and in-source generated JavaScript according to the selected ReScript
output configuration. Commit authored `.res` and `.resi` files, configuration, and lockfiles;
follow repository policy for any intentionally published generated output.

Verify the boundary the compiler cannot prove: use browser checks for hydration and initial URL
state; byte-for-byte parity for output deterministic in the supported runtime; semantic decoding,
signature, dimensions, chunks, and metadata when compression or encoding can vary; and the
repository's Worker harness or Wrangler with isolated persistence for routes, methods, headers,
state, and generated responses.

When persisted-schema compatibility is in scope, test both representative legacy state and a
fresh isolated state. Inspect actual columns and constraints during upgrades; a recorded migration
version alone does not prove that persisted storage has the expected shape.

Use the current official documentation when syntax or configuration may have changed:

- https://rescript-lang.org/docs/manual/introduction/
- https://rescript-lang.org/docs/manual/interop-cheatsheet/
- https://rescript-lang.org/docs/react/introduction/
- https://rescript-lang.org/docs/manual/typescript-integration/
