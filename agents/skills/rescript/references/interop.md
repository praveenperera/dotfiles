# JavaScript, browser, Node, and TypeScript interop

Use this reference before declaring an `external`, consuming untrusted data, or crossing a
framework boundary. These patterns were checked against ReScript 12 documentation and bindings
that compiled in a ReScript 12.3 local application on 2026-07-27.

## Contents

- [Binding rules](#binding-rules)
- [Common external shapes](#common-external-shapes)
- [Optional and nullable values](#optional-and-nullable-values)
- [Untrusted JSON](#untrusted-json)
- [Promises and async](#promises-and-async)
- [React and DOM events](#react-and-dom-events)
- [Browser and Node boundaries](#browser-and-node-boundaries)
- [TypeScript boundaries](#typescript-boundaries)
- [Failure modes](#failure-modes)

## Binding rules

Treat each `external` as a checked assertion about runtime JavaScript. The compiler proves calls
against the declared type, not against the actual package. Inspect the installed export and its
`.d.ts` or source first.

- bind only the used surface
- hide raw externals in one module
- use abstract types for opaque JavaScript objects
- verify generated JS matches the call that would be handwritten
- put binding attributes in `.resi` declarations too so externals remain zero-cost
- add a runtime test when a wrong export name, `this` binding, or nullability could still compile

## Common external shapes

Named module export:

```rescript
@module("node:path")
external dirname: string => string = "dirname"
```

Default export:

```rescript
type client
type config

@module("some-client")
external makeClient: config => client = "default"
```

Global and scoped global:

```rescript
@val external fetch: string => promise<response> = "fetch"

@scope("process") @val
external cwd: unit => string = "cwd"
```

Object method and property:

```rescript
type response

@get external status: response => int = "status"
@send external json: response => promise<JSON.t> = "json"
```

Constructor:

```rescript
type blob
type blobOptions = {type_: string}

@new
external makeBlob: (array<string>, blobOptions) => blob = "Blob"
```

Overloads should normally become multiple named externals pointing to the same JavaScript export:

```rescript
@module("node:path")
external resolve2: (string, string) => string = "resolve"

@module("node:path")
external resolve3: (string, string, string) => string = "resolve"
```

Avoid importing a whole module as a single value unless its ESM/CommonJS output has been checked.
Explicit named or default imports are easier to verify.

## Optional and nullable values

Use an optional labeled argument for an omitted JavaScript argument:

```rescript
@module("drawing")
external draw: (~x: int, ~y: int, ~border: bool=?) => unit = "draw"
```

Use `option<'a>` only when the JavaScript value is absent as `undefined`. Use
`Nullable.t<'a>` when either `null` or `undefined` can occur. Convert nullable values at the
boundary:

```rescript
value->Nullable.toOption
```

Do not declare a nullable JavaScript value as non-null merely to make a component compile.

## Untrusted JSON

Do not bind `JSON.parse` directly to the desired application record for files, storage, network
responses, or user-controlled data. Parse to `JSON.t`, then decode every required field:

```rescript
type user = {id: string, score: float}

let decodeUser = json =>
  switch json {
  | JSON.Object(dict{
      "id": JSON.String(id),
      "score": JSON.Number(score),
    }) if score->Float.isFinite =>
    Some({id, score})
  | _ => None
  }

let parseUser = text => {
  let json = try {
    Some(JSON.parseOrThrow(text))
  } catch {
  | _ => None
  }

  json->Option.flatMap(decodeUser)
}
```

Return a typed error variant instead of `option` when callers need to distinguish invalid syntax,
missing fields, unsupported versions, and invalid values. Decode once and keep raw `JSON.t` out
of the domain layer. For generated and declarative decoder choices, read
[json-codecs.md](json-codecs.md).

## Promises and async

Represent a JavaScript promise as `promise<'a>` and prefer ReScript `async`/`await`. Unlike
JavaScript, nested promises do not automatically collapse in an async function; explicitly
`await` a promise before returning its value.

```rescript
@module("node:fs/promises")
external readFile: (string, string) => promise<string> = "readFile"

let readText = async path => {
  await readFile(path, "utf8")
}
```

Handle expected failures at the boundary with `try`/`catch`. Preserve unexpected exceptions
instead of translating every failure into `None`.

## React and DOM events

Use the event type exposed by the installed `@rescript/react`. Current v12 documentation shows
`JsxEvent.Form.t`; a working `@rescript/react` 0.15 application also exposes
`ReactEvent.Form.t`. Inspect the installed `.resi` files rather than guessing across versions.

Keep event-target extraction in a small browser module:

```rescript
type inputTarget

@get
external eventTarget: ReactEvent.Form.t => inputTarget = "target"

@get
external targetValue: inputTarget => string = "value"

let inputValue = event => event->eventTarget->targetValue
```

Framework event types are a trust boundary because the runtime target can differ by event and
element. Test important form behavior in a browser.

ReScript React exposes a fixed DOM prop surface, including only selected `data-*` attributes.
Prefer a supported semantic or test prop such as `dataTestId`; otherwise add one narrow binding.
Do not keep an entire component in TypeScript or use an unsafe cast solely for one custom
attribute.

## Browser and Node boundaries

Use opaque types plus `@get`, `@set`, `@send`, `@new`, `@val`, and `@scope` for the small browser
surface actually needed. Prefer official Web API bindings if they match the installed ReScript
version; otherwise keep local bindings narrow.

Use `node:` module specifiers for built-in Node modules in ESM projects:

```rescript
@module("node:fs/promises")
external rename: (string, string) => promise<unit> = "rename"
```

Use an explicit runtime-resolvable extension when binding a local TypeScript ESM module, such as
`@module("./CustomError.ts")`. Compilation does not prove that Node or the selected bundler will
resolve an extensionless local import; exercise the generated import in the actual runtime.

For durable local files, write to a uniquely named temporary file and rename it into place.
Remove the temporary file on failure. Serialize concurrent writes when multiple optimistic UI
updates can overlap.

Environment variables are nullable. Bind the environment object opaquely and return
`Nullable.t<string>` for each property instead of declaring it present.

Use the standard `Uint8Array`, `Uint32Array`, `TypedArray`, and `ArrayBuffer` modules before
assuming binary code needs TypeScript. Bind only missing platform APIs such as compression
streams. Keep controlled unsafe indexing inside proven bounds and require byte-for-byte parity
tests for deterministic binary formats and signed bitwise algorithms. When platform compression
can vary across supported runtimes, compare decoded data and format invariants there instead of
requiring identical compressed bytes.

Create global regular expressions inside each parse call. JavaScript `RegExp` values with the
global flag carry mutable `lastIndex` state. Test repeated calls and capture ordering because the
compiler cannot prove that a capture index maps to the intended field.

## TypeScript boundaries

Use handwritten externals for a small stable surface. Use GenType when TypeScript must consume a
broader intentional ReScript API or when ReScript must import TypeScript values and generated
types improve maintenance.

Keep framework adapters mechanical:

- translate default and named exports
- translate framework-owned request, response, or component conventions
- convert external values into ReScript domain types
- do not duplicate business decisions on both sides

Bind a stable reusable component or package primitive once before declaring all of its consumers
TypeScript boundaries. Keep adapters for object-heavy, highly generic, callback-overloaded, or
render-prop APIs when binding them would expose more package machinery than application logic.

Generated `.gen.tsx` and `.res.js` files are outputs, not sources to edit.

When using GenType:

- put the public type, value declaration, and `@genType` annotation in `.resi` when the module has
  an interface; keep the implementation annotation only when there is no `.resi`
- align `gentypeconfig.moduleResolution` with TypeScript and set the intended
  `generatedFileExtension`
- ignore the configured `.gen.ts` or `.gen.tsx` artifacts unless the package deliberately
  publishes them
- after changing the generated extension or annotations, find and remove stale generated files
- normalize `null | undefined` in a compatibility adapter when an existing TypeScript API
  promises one representation, such as converting both to `null`
- remember that generated ReScript arrays are mutable TypeScript arrays; copy or wrap them when a
  public TypeScript API promises `readonly`
- annotate cross-module record values and callbacks when related nominal record shapes share field
  names; ReScript records do not structurally widen

When caught JavaScript values must retain identity across a generated boundary, unwrap the
ReScript exception with `JsExn.fromException`. Keep a JavaScript/TypeScript `Error` subclass at the
adapter when public `instanceof` behavior is part of the contract.

Generated types describe an intentional boundary; they do not replace runtime validation of
untrusted values.

## Failure modes

Check these first when code type-checks but fails at runtime:

1. named export bound as default, or default bound as named
2. method bound as a plain function instead of `@send`
3. missing `@new` for a constructor
4. `null` modeled as `option`, which only represents `undefined` at the JS boundary
5. record fields that do not match the actual runtime property names
6. promise return type or nested promise modeled incorrectly
7. stale community binding for a different upstream major
8. framework expecting a default export, directive, or file convention
9. generated JavaScript missing because the bundler started before the ReScript build
10. event target asserted more narrowly than the browser guarantees
11. local ESM binding emitted with an extension the runtime does not resolve
12. caught JavaScript error crossing as a ReScript exception wrapper
13. global regular expression reused with a stale `lastIndex`
14. TypeScript caller mutating an array that was intended to be readonly

## Sources

- https://rescript-lang.org/docs/manual/external/
- https://rescript-lang.org/docs/manual/interop-cheatsheet/
- https://rescript-lang.org/docs/manual/import-from-export-to-js/
- https://rescript-lang.org/docs/manual/bind-to-js-function/
- https://rescript-lang.org/docs/manual/bind-to-js-object/
- https://rescript-lang.org/docs/manual/null-undefined-option/
- https://rescript-lang.org/docs/manual/json/
- https://rescript-lang.org/docs/manual/async-await/
- https://rescript-lang.org/docs/manual/typescript-integration/
- https://rescript-lang.org/docs/react/events/
