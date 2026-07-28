# Domain modeling in ReScript

Use this reference before designing or refactoring nontrivial application state. Shape data so
valid states are natural, invalid states are difficult or impossible to construct, and invariants
have one clear owner.

## Contents

- [Start from invariants](#start-from-invariants)
- [Recognize a weak model](#recognize-a-weak-model)
- [Choose the ReScript type](#choose-the-rescript-type)
- [Hide valid construction](#hide-valid-construction)
- [Model mutually exclusive states](#model-mutually-exclusive-states)
- [Preserve semantic state](#preserve-semantic-state)
- [Model numbers deliberately](#model-numbers-deliberately)
- [Own transitions](#own-transitions)
- [Separate external and domain data](#separate-external-and-domain-data)
- [Consolidate historical representations](#consolidate-historical-representations)
- [Refactor safely](#refactor-safely)
- [Avoid over-modeling](#avoid-over-modeling)

## Start from invariants

Before extracting helpers or reorganizing files:

1. identify the fact the code repeatedly checks
2. name the domain concept that owns that fact
3. list valid states and transitions
4. ask whether behavior depends on origin, authority, lifecycle, provenance, or transition history
5. find duplicate, older, or conflicting representations
6. introduce the smallest type that carries the proof
7. parse or construct that type as early as practical
8. move transitions into the owning module
9. delete redundant flags, checks, and impossible branches

Preserve external behavior unless the user approves a correctness, compatibility, persisted-data,
or public-API change.

## Recognize a weak model

Look for:

- raw strings, numbers, or IDs that represent different domain concepts
- multiple booleans describing one state
- `status` plus fields that are valid only for some statuses
- sentinel values such as `""`, `0`, `-1`, or magic dates
- records where most fields are `option`
- loosely related parameters repeatedly passed together
- duplicated validation in UI, API, persistence, and export code
- comments that explain an invariant the types do not enforce
- wildcard branches that claim a closed domain case is impossible
- parallel legacy and current representations

Repeated fixes in one area usually mean the model is incomplete. Do not add another caller-level
condition until checking whether the owner type should change.

## Choose the ReScript type

| Domain shape | ReScript model |
| --- | --- |
| fixed group of named fields | immutable record |
| exactly one of several states | variant |
| state with state-specific data | variant with record payloads |
| value may legitimately be absent | `option<'a>` |
| operation can fail in expected ways | `result<'value, 'error>` with an error variant |
| validated primitive with meaningful identity | abstract module type plus constructor/parser |
| open string set from an external system | explicit known cases plus an unknown payload |
| UI request lifecycle | `Idle | Loading | Loaded(data) | Failed(error)` |

Use polymorphic variants mainly for structural composition or JavaScript interop. Prefer regular
variants for owned domain concepts because they have one explicit definition.

Keep records immutable unless mutation is required for a measured hot path or a tightly scoped
imperative boundary. Immutable updates make transitions visible:

```rescript
let rename = (account, name) => {...account, name}
```

## Hide valid construction

A type alias alone does not protect a validated primitive. Hide its representation with a
`.resi` interface and expose only construction and conversion.

`UserId.res`:

```rescript
type t = string
type error = Empty

let parse = raw => {
  let value = raw->String.trim
  if value == "" {
    Error(Empty)
  } else {
    Ok(value)
  }
}

let toString = value => value
```

`UserId.resi`:

```rescript
type t
type error = Empty

let parse: string => result<t, error>
let toString: t => string
```

Outside `UserId`, callers cannot substitute an arbitrary string for `UserId.t`. Use this for
concepts whose confusion or invalid construction would matter: account IDs, validated email
addresses, nonempty names, normalized categories, money, or persisted version identifiers.

Do not wrap every primitive. The abstraction must remove repeated validation, prevent confusion,
or own meaningful behavior.

## Model mutually exclusive states

Replace status flags and state-dependent optional fields with variants carrying only the data
valid in each state.

Avoid:

```rescript
type review = {
  status: string,
  category: option<category>,
  amount: option<money>,
  isDenied: bool,
}
```

Prefer:

```rescript
type pendingReview = {
  category: option<category>,
  amount: option<money>,
}

type approvedReview = {
  category: category,
  amount: money,
}

type deniedReview = {
  reason: option<string>,
}

type review =
  | Pending(pendingReview)
  | Approved(approvedReview)
  | Denied(deniedReview)
```

Now `Approved` cannot exist without a category and amount, and denied-only data cannot leak into
another state. Pattern matching forces callers to handle every review state.

Apply the same pattern to remote data, optimistic saves, authentication, uploads, payments, and
multi-step forms. Avoid separate `loading`, `data`, and `error` values when only a few
combinations are valid.

## Preserve semantic state

A typed translation of an incomplete model remains incomplete. Before porting state, ask whether
the current value omits information that changes later behavior:

- origin: where did the value come from?
- authority: is it a default, system value, persisted preference, or explicit user choice?
- lifecycle: has it merely loaded, been validated, been saved, or been acknowledged?
- transition: which actions are legal from this state?

For example, `Light | Dark` cannot explain whether a theme follows the operating system or was
chosen by the user:

```rescript
type theme = Light | Dark

type preference =
  | System(theme)
  | Explicit(theme)
```

Now a media-query change can update `System(_)` while preserving `Explicit(_)`. Use variants with
payloads when behavior depends on provenance. Do not add history that has no behavioral meaning.

## Model numbers deliberately

ReScript `int` has signed 32-bit semantics and truncates when necessary. It is suitable for
bounded counts, indexes, small identifiers, and intentional bitwise fields. It is not a safe
default for every JavaScript number.

Use `float` at a JavaScript or JSON boundary for Unix timestamps, epoch milliseconds, and integer
values that may exceed the signed 32-bit range. Before treating an incoming `float` as an integer,
verify that it is finite, `Number.isSafeInteger`, and within any domain-specific bounds. Before
calling `Float.toInt`, also verify the value lies between `-2_147_483_648` and `2_147_483_647`.

Keep large safe integers as `float` behind an abstract domain type when arithmetic must preserve
integrality. Use `bigint` only when values can exceed JavaScript's safe-integer range and the
project accepts ReScript's currently experimental bigint support.

Boundary tests should include fractional values, signed bounds, values just outside those bounds,
unsafe JavaScript integers, and post-2038 timestamps. A compiler-approved `int` record does not
prove that its decoder validated a JavaScript number correctly.

Validate the complete numeric string before conversion. JavaScript `Number.parseInt` accepts a
valid prefix followed by junk, so a successful conversion does not prove that the whole input
matched the intended decimal or hexadecimal grammar.

## Own transitions

Put business transitions in the module that owns the aggregate instead of reconstructing records
in UI event handlers, API routes, and persistence code.

```rescript
type approveError =
  | MissingCategory
  | MissingAmount

let approve = (pending: pendingReview) =>
  switch (pending.category, pending.amount) {
  | (Some(category), Some(amount)) => Ok(Approved({category, amount}))
  | (None, _) => Error(MissingCategory)
  | (_, None) => Error(MissingAmount)
  }
```

Prefer functions that return a new aggregate or a typed error. Keep transition preconditions and
derived values in one place. UI code should request `approve`; it should not know how an approved
record is assembled.

Use separate input types when a transition protocol benefits from compile-time sequencing. Avoid
typestate-like proliferation when runtime data determines the state or ordinary pattern matching
is clearer.

Separate deterministic plans from effect execution. Page starts, ranges, batching, concurrency
caps, refresh stages, missing-work selection, and output ordering belong in typed domain
functions even when `fetch`, SQL, cache, or Durable Object calls remain in a framework adapter.

## Separate external and domain data

Treat JSON, CSV, database rows, form strings, environment variables, and JavaScript package
objects as external representations. They may be permissive, nullable, stringly typed, or
versioned. Convert them into precise domain values before business logic.

Use this flow:

```text
external bytes or object
  -> syntax parse
  -> boundary DTO / JSON.t
  -> field and invariant validation
  -> canonical domain type
  -> domain transition
  -> explicit encoder
```

Keep optional fields and compatibility aliases in the decoder, not in the canonical model. Keep
raw `JSON.t` and arbitrary dictionaries out of domain functions. See [interop.md](interop.md) for
safe parsing and binding patterns.

Do not rely on a variant's incidental generated JavaScript representation for durable storage.
Encode and decode persisted states explicitly so refactoring an internal constructor does not
silently change the file or API format.

## Consolidate historical representations

Code accumulates old assumptions, newer edge cases, compatibility branches, and patches. When
several representations describe the same concept:

1. choose one canonical internal representation
2. keep legacy formats at boundary-specific migration functions
3. migrate immediately after parsing
4. write only the current format
5. test every supported migration
6. remove obsolete internal branches after compatibility is proven

Ask before removing a compatibility path, changing a persisted schema, rewriting a public API, or
broadening beyond the requested scope. If a behavior change would make the model substantially
safer, explain the tradeoff instead of hiding it inside a refactor.

## Refactor safely

Before changing types, produce concrete findings. A large file or a subjective preference is not
enough. Identify the repeated invariant, impossible combination, duplicated validation, or
correctness risk.

Then:

1. add tests around current parsing, transitions, and persisted behavior
2. introduce the canonical type and constructors
3. migrate one boundary or caller at a time
4. compile after each slice and follow the first causal type error
5. move behavior to the owning module
6. delete superseded checks only after all callers use the new type
7. run format, compile, build, and behavior tests

The compiler exposes every caller affected by a model change. Use that feedback as the migration
map rather than weakening the type to make errors disappear.

## Avoid over-modeling

- do not create a module and abstract type for every string
- do not encode runtime facts in elaborate types when a variant and constructor are clearer
- do not preserve known-wrong representations merely to minimize the diff
- do not split one cohesive aggregate across many tiny files
- do not expose mutable record fields to avoid writing transition functions
- do not replace readable branching with advanced type machinery solely for cleverness

No model change is a valid outcome when the current data already makes invariants clear and
invalid combinations unrepresentable.

## Sources

- https://rescript-lang.org/docs/manual/primitive-types/
- https://rescript-lang.org/docs/manual/shared-data-types/
- https://rescript-lang.org/docs/manual/module/
- https://rescript-lang.org/docs/manual/import-export/
- https://rescript-lang.org/docs/manual/record/
- https://rescript-lang.org/docs/manual/variant/
- https://rescript-lang.org/docs/manual/pattern-matching-destructuring/
