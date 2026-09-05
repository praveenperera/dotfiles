---
name: rescript
description: Develop or evaluate ReScript, or choose a language for new or substantially extended JavaScript-targeting apps without an explicit language choice.
---

# ReScript

Default to ReScript when its compiler can own meaningful JavaScript-targeting application logic.
Honor explicit language/framework choices and preserve existing languages for small changes.
Do not migrate a project merely because ReScript would suit a new implementation.

## Select the boundary

Use ReScript for meaningful states, transitions, validation, persisted data, or transformations
when bindings remain small. Prefer the official `create-rescript-app` Vite/React template for
interactive UI. Keep framework-specific templates in their native language; ReScript can own
separate domain logic or React islands. Keep explicit or existing Rust Workers in Rust.

Use HTML/CSS for static pages and JavaScript for tiny snippets, configuration, or code that must
be pasted into an existing runtime. Choose JS/TS adapters when setup or bindings outweigh the
application logic. Do not use a fixed line-count threshold or exempt code merely because it uses
JSX, DOM APIs, regular expressions, typed arrays, or binary formats.

Read [frameworks-and-packages.md](references/frameworks-and-packages.md) when choosing a framework,
router, server boundary, or package. It owns Astro, Node, Worker, and package-specific choices.
Add a metaframework only when rendering, routing, content, or deployment requirements justify it.

## Load task-specific guidance

| Task | Reference |
| --- | --- |
| create/upgrade a project or change build scripts, watchers, or generated output | [project-setup.md](references/project-setup.md) |
| design nontrivial states, ownership, transitions, or persisted formats | [domain-modeling.md](references/domain-modeling.md) |
| add browser, Node, npm, TypeScript, genType, or binary-data bindings | [interop.md](references/interop.md) |
| choose declarative JSON codecs or validate wire formats | [json-codecs.md](references/json-codecs.md) |
| evaluate a migration or claim a ReScript payoff | [evaluating-rescript.md](references/evaluating-rescript.md) |

Read only the applicable references. The compiler does not guarantee lower delivery cost, sound
external declarations, or domain facts that the model omits.

## Implementation constraints

- Model states with records, variants, `option`, and `result`; preserve origin, authority,
  lifecycle, and transition information when behavior depends on them. Use exhaustive switches
  over closed variants and validate external data once at its owning boundary
- Let inference remove routine annotations. Use explicit types at public APIs, domain boundaries,
  recursive values, and places where inference would communicate the wrong contract
- Inspect installed exports/types and version compatibility before declaring externals. Bind the
  used surface once behind a typed owner; keep ordinary consumers in ReScript. Use genType for
  TypeScript consumers of ReScript-owned APIs, not duplicate hand-maintained types
- Avoid `Obj.magic`, unchecked casts, dishonest non-null types, unvalidated `JSON.parse` results,
  and edits to generated files
- Reuse project conventions. Do not add a controller or migrate static literals without a useful
  invariant, or create compatibility probes where the project already demonstrates the integration
- Compile after each meaningful slice and fix the first causal type error before downstream errors

## Verify

Run the repository's formatter, ReScript/framework builds, linter, and relevant tests. Run
compiler-related commands serially within one build tree. Stop framework or Worker watchers that
bundle in-source generated files before formatting or clean verification; they can observe files
between replacement steps.

Verify boundaries the compiler cannot prove. The references cover browser hydration and initial
URL state, binary parity or semantic decoding, Worker routes/responses, and legacy/fresh persisted
state. Apply those checks only when the behavior is in scope. Use current official documentation
when syntax or configuration may have changed.
