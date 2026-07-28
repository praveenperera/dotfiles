# Declarative JSON codecs

Use this reference when external JSON is large enough that handwritten pattern matching would
repeat structure or obscure validation. Recheck package versions, maintenance, runtime
constraints, and ReScript compatibility before installation.

## Contents

- [Choose the boundary](#choose-the-boundary)
- [Check before installing](#check-before-installing)
- [Sury PPX](#sury-ppx)
- [PPX Spice](#ppx-spice)
- [ReScript JSON Combinators](#rescript-json-combinators)
- [Protect the domain](#protect-the-domain)

## Choose the boundary

| Need | Default |
| --- | --- |
| substantial validation, transformations, JSON Schema, or reusable contracts | Sury PPX when its current ReScript release is acceptable |
| direct Serde-like encoder and decoder generation | PPX Spice |
| explicit stable decoder without PPX or runtime code generation | ReScript JSON Combinators |
| one small payload | built-in `JSON.t`, pattern matching, and `JSON.Decode` |

These tools are JSON-specific; they are not a format-independent trait ecosystem like Rust Serde.
Do not bind `JSON.parse` directly to an application record and mistake a type assertion for
validation.

## Check before installing

1. compare the package's ReScript peer or tested version with the project
2. inspect recent commits, releases, unresolved compatibility issues, and maintainer count
3. check whether the package or selected version is stable, prerelease, or `0.x`
4. confirm that CSP and the target runtime allow its code-generation strategy
5. pin unstable versions exactly and preserve the lockfile
6. compile a representative record, variant, optional field, and custom codec

Treat download counts and stars as adoption signals, not correctness guarantees. Each reviewed
option had one principal maintainer as of 2026-07-27, so keep the codec boundary narrow enough to
replace.

## Sury PPX

Sury can generate a bidirectional schema from a type:

```rescript
@schema
type status =
  | @as("active") Active
  | @as("disabled") Disabled

@schema
type user = {
  @as("user_id")
  id: string,
  tags: @s.default([]) array<string>,
  status: status,
}
```

The PPX generates `<typeName>Schema`. Use it for parsing, serialization, transformations,
refinements, error paths, and JSON Schema generation.

Sury was highly active and materially adopted at the maintenance snapshot, including substantial
downloads of the ReScript-specific `sury-ppx` package. However, ReScript 12 support required Sury
11 alpha while the last stable Sury 10 release targeted ReScript 11. Until a stable compatible
release exists:

- pin `sury` and `sury-ppx` to the same exact version
- read the release notes before upgrading
- protect codecs with boundary and round-trip tests
- expect API or wire-behavior changes during upgrades

Sury compiles optimized parsers with `new Function`. Verify the production runtime and CSP before
choosing it. Keep a TypeScript adapter or another decoder when dynamic code generation is
prohibited.

## PPX Spice

Spice generates direct `<typeName>_encode` and `<typeName>_decode` functions:

```rescript
@spice
type user = {
  @spice.key("user_id")
  id: string,
  name: string,
}
```

Use the current `@mununki/ppx-spice` package; `@greenlabs/ppx-spice` was the previous namespace.
At the maintenance snapshot, Spice had a stable ReScript 12-compatible `0.4.1` release and recent
feature work, but much smaller public adoption than Sury. Because it remains `0.x`, pin it exactly
for consequential persisted or public formats.

Prefer Spice when generated JSON codecs are the complete requirement. Test generic types,
nullable and optional fields, custom field names, and variant payload representations actually
used by the application.

## ReScript JSON Combinators

`@glennsl/rescript-json-combinators` provides explicit, composable encoders and decoders without
PPX or runtime `new Function` generation. It had a ReScript 12-compatible release and modest
ongoing adoption at the maintenance snapshot.

Prefer it when transparency and a stable explicit wire contract matter more than avoiding a
second schema declaration. It is a conservative choice for persisted configuration and formats
that must remain readable across compiler or PPX changes.

## Protect the domain

- keep wire DTOs separate when JSON names, optionality, or states differ from the domain
- validate numeric integrality and range rather than accepting every JSON number
- define the unknown-field policy explicitly
- version persisted formats and migrate immediately after decoding
- test malformed syntax, missing fields, wrong types, `null`, unknown fields, numeric boundaries,
  variants, and supported historical versions
- round-trip only where encode/decode symmetry is part of the contract

Generated codecs remove boilerplate; they do not decide domain invariants or compatibility policy.

## Sources

- https://rescript-lang.org/docs/manual/json/
- https://github.com/DZakh/sury
- https://github.com/DZakh/sury/blob/main/packages/sury-ppx/README.md
- https://github.com/mununki/ppx_spice
- https://forum.rescript-lang.org/t/ann-green-labs-ppx-spice-has-moved-to-mununki-ppx-spice/7104
- https://www.npmjs.com/package/@glennsl/rescript-json-combinators
