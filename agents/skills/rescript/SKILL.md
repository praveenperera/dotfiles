---
name: rescript
description: Use for new or substantially extended JavaScript-targeting apps without an explicit language choice, or for ReScript development and evaluation. Default suitable work to ReScript; preserve existing languages for small edits.
---

# ReScript

Default to ReScript for JavaScript-targeting work when its compiler can own meaningful application
logic. Optimize for delivery cost, not language purity: bindings and scaffolding can cost more than
the compiler saves. Type safety ends at an incorrectly typed JavaScript boundary.

## Choose the language boundary

Honor explicit language and framework choices. Preserve the language of small changes in existing
projects; do not migrate a project merely because ReScript would suit a new implementation.

| Work | Default |
| --- | --- |
| new React app, dashboard, form, interactive tool, or reusable domain logic | ReScript; Vite + ReScript React for UI |
| static page without meaningful behavior | HTML/CSS; no compiler |
| tiny snippet, bookmarklet, configuration, or disposable DOM script | JavaScript |
| Svelte, Vue, Solid, or Astro templates | native TypeScript; ReScript can own separate domain logic |
| content site with substantial interactive React islands | Astro/TypeScript shell with ReScript React islands |
| Node service or JavaScript-targeting Worker | ReScript where bindings stay narrow; a thin JS/TS adapter otherwise |
| explicit Rust or existing Rust Worker | workers-rs; do not introduce TypeScript only for Durable Objects |
| binary, raster, or typed-array algorithm | ReScript when platform bindings stay narrow; verify runtime output |
| dependency-heavy integration with missing or highly generic bindings | a small TypeScript boundary exposed to ReScript |

ReScript is useful when state transitions, nullable/error states, untrusted input, persisted data,
or transformations carry meaningful invariants. Choose JS/TS when setup or binding code outweighs
application logic, or code must be pasted into an existing runtime. Do not use a fixed line-count
threshold. JSX, DOM APIs, regular expressions, and binary data do not by themselves require JS/TS.

For new interactive apps, prefer the official `create-rescript-app` Vite/React template and
`@rescript/react`. Use the built-in ReScript React router for simple client routing. Add Next.js or
another metaframework only when rendering, routing, content, or deployment requirements justify it.
For a local durable tool, use a small Node API for filesystem/database access, with ReScript domain
logic and a thin JS/TS server adapter only when bindings justify one.

## Read the relevant reference

Load only the guidance needed by the current task:

| Task | Reference |
| --- | --- |
| create or upgrade a project; change build scripts, watchers, or generated output | [project-setup.md](references/project-setup.md) |
| choose a framework/package or integrate Astro islands | [frameworks-and-packages.md](references/frameworks-and-packages.md) |
| design or refactor nontrivial state, ownership, transitions, or persisted formats | [domain-modeling.md](references/domain-modeling.md) |
| add browser, Node, npm, TypeScript, genType, or binary-data bindings | [interop.md](references/interop.md) |
| choose declarative JSON codecs or validate wire formats | [json-codecs.md](references/json-codecs.md) |
| evaluate a migration or claim a ReScript payoff | [evaluating-rescript.md](references/evaluating-rescript.md) |

## Implementation constraints

- Model states with records, variants, `option`, and `result`. Preserve origin, authority, lifecycle,
  and transition information when behavior depends on them. Prefer exhaustive switches over closed
  variants, and parse external data once at its owning boundary
- Let inference remove routine annotations. Use explicit types at public APIs, domain boundaries,
  recursive values, and places where inference would communicate the wrong contract
- Check installed versions and real package exports/types before writing bindings. Bind only the
  used surface, then wrap externals in a typed owner. A declaration can compile while being wrong;
  verify uncertain runtime representations and calling conventions
- Bind stable package primitives once and keep ordinary consumers in ReScript. Use genType when
  TypeScript consumes ReScript-owned APIs; do not duplicate those types by hand or edit generated
  files. Keep a small adapter when package-specific machinery dominates
- Avoid `Obj.magic`, unchecked casts, dishonest non-null types, and unvalidated `JSON.parse` results
- Reuse project conventions. Do not add a framework, server, controller, or schema without a useful
  role, migrate static literals without an invariant, or create compatibility probes where the
  repository already demonstrates the integration
- Compile after each meaningful slice and fix the first causal type error before downstream
  errors. Do not generate the entire application before the first compile

## Verify

Run the repository's formatter, ReScript build, framework build, linter, and relevant tests.
Run compiler-related commands serially within one build tree. Stop framework or Worker watchers
that bundle in-source generated files before formatting or clean verification; they can observe
files between replacement steps. Follow the generated-output and lifecycle rules in
[project-setup.md](references/project-setup.md) when changing that setup.

Verify boundaries the compiler cannot prove: browser hydration and initial URL state, binary
parity or semantic decoding, Worker routes and responses, and legacy/fresh persisted state when
those behaviors are in scope. The corresponding references above contain the focused checks.
Use current official ReScript documentation when syntax or configuration may have changed.
