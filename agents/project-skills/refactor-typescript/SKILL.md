---
name: refactor-typescript
description: Refactor TypeScript code for clarity, structure, and maintainability. Splits large modules, reduces nesting, improves domain models, then applies style conventions. Use when asked to "refactor this TypeScript", "clean up this TS module", "split this file", "simplify this TypeScript code", or `/refactor-typescript`.
---

# Refactor TypeScript

## Goal

Code should be easy to read, easy to follow, easy to review, and easy to change. But "no refactor needed" is a valid and good outcome. Don't refactor for refactoring's sake - only change code that has real readability, maintainability, or correctness pain.

## Setup

Read the project's AGENTS.md, CLAUDE.md, package.json, tsconfig files, lint config, formatter config, and nearby code before starting. Follow the project's existing framework and style instead of assuming defaults.

Preserve behavior by default. If a refactor exposes a correctness bug, isolate it, write a reproducing test, and report it to the user - don't fix it in the same pass. Ask the user before major module splits, public API changes, persisted data shape changes, route changes, or broad component rewrites.

Discover repo-specific verification first: check package manager files and scripts, then prefer the project's existing commands for format, lint, typecheck, and tests. Run relevant tests before and after when the scope has behavioral risk.

## Scope

- **Recent changes (default)**: Only refactor recently touched code - could mean uncommitted changes, commits on this branch, or the last day or two of work. Ask the user to confirm scope before starting
- **Full sweep**: When the user explicitly asks to refactor a whole module, package, app, or codebase

Infer from context: bare `/refactor-typescript` or "clean up my changes" = recent. "Refactor this codebase" or "refactor this module" = full sweep. When unsure, ask.

## Findings First

Before any refactoring, produce a findings report. Start skeptical - do not assume a refactor is justified.

- Review the code in scope and list concrete problems: maintainability issues, correctness risks, testing gaps, unclear control flow, weak data modeling, unsafe typing, framework misuse
- Every finding must point to specific code with a concrete reason - not "this file is large" or "this could be cleaner"
- Separate actionable problems from code smells. Code smells alone don't justify a refactor
- If the code is already in good shape, say so explicitly and stop
- Order findings by severity. Only proceed to Pass 1/2 for findings that warrant action

## Pass 1: Structure

Only proceed if Findings First identified structural issues that warrant action.

Spawn an agent focused exclusively on structural refactoring. Do not touch style in this pass. Clarity over brevity - explicit code is often better than overly compact.

### Data-first domain modeling

Before extracting helpers or splitting modules, identify the core domain data, invariants, and state transitions. Within the confirmed scope, refactors should usually improve the shape of the data model before optimizing control flow. Prefer reshaping data types so valid states are natural and invalid states are hard or impossible to construct, rather than spreading validation and branching across functions.

TypeScript types are erased at runtime, so pair precise static types with runtime validation at system boundaries. Parse `unknown` external data into precise internal values before business logic runs, using the project's existing parser/schema library when one exists. Avoid trusting `as`, non-null assertions, or generated API shapes as proof that runtime data is valid.

Look for places where the code is compensating for a weak model: primitive obsession, boolean state, loosely related parameters, duplicated validation, sentinel values, optional fields that only make sense in some states, impossible branches, comments that explain invariants the types do not enforce, or large prop/config objects that allow invalid combinations. Move those facts into types, constructors, parsers, and transition functions so callers must provide valid domain values and invalid combinations are difficult to express.

Refactors should consolidate the current understanding of the problem space. Code often represents several layers of past understanding: old assumptions, newer edge cases, compatibility branches, duplicated checks, and patches added after the model proved incomplete. Do not preserve that drift by adding another bandaid. If the domain model is wrong or split across competing representations, propose the fuller reshape that would make the final model explicit, simpler to follow, and easier to verify. Ask before rewriting public APIs, changing persisted formats, removing compatibility paths, or broadening beyond the requested scope. If a behavior change would make the model much simpler or more correct, explain the tradeoff and ask the user before applying it.

Prefer:
- Branded or opaque types for domain primitives (`UserId`, `EmailAddress`, `Cents`) instead of raw `string`, `number`, or `bigint`
- Value objects with private constructors, factories, or parser functions when construction has invariants
- Discriminated unions with payloads for mutually exclusive states instead of `status` fields plus flags/options
- Exhaustive switches over discriminated unions, with `never` checks when the project uses that pattern
- Aggregate methods, reducers, or transition functions for business state changes, so invariants are checked in one consistency boundary
- Parsing/validation at system boundaries, converting loose external data into precise internal types before business logic runs
- Replacing parallel historical representations with one canonical internal representation
- `readonly`, immutable updates, and stable value shapes when mutation makes state transitions hard to trace
- Typestate-like generics only when the state transition is part of the API protocol and compile-time sequencing is worth the extra type complexity

Refactoring heuristic:
1. Identify the invariant the code keeps re-checking
2. Name the domain concept that owns that invariant
3. Find older assumptions or duplicate representations that conflict with the current understanding
4. Introduce the smallest type, parser, factory, or discriminated union that can carry the proof
5. Parse into that type as early as practical
6. Move behavior and transitions onto the type, aggregate, reducer, or domain service
7. Delete now-redundant checks, flags, impossible branches, unsafe assertions, and comments that merely restate the type
8. Preserve external behavior unless the user explicitly approves a correctness or compatibility change

Avoid turning every rule into type-level machinery. If the state depends on persisted/runtime data or would make ordinary code much harder to read, use a runtime discriminated union plus focused constructors and transition functions.

### Module splitting
- Use line count as a heuristic, not a trigger - large modules may warrant splitting by concern, but too many tiny files can make code harder to follow
- Split by domain boundary, side-effect boundary, or framework boundary, not by arbitrary buckets like `helpers.ts`
- Keep public entrypoints stable unless the user approved API churn
- Avoid barrel files when they obscure dependency direction, hurt tree-shaking, or create cycles; follow the project's import convention when already established

### Function extraction
- Functions doing multiple things: extract helpers
- Each function should have a single clear responsibility
- Keep parsing, authorization, domain decisions, side effects, and rendering separated when the framework allows it
- Same cohesion warning: too many tiny helpers can hurt readability more than a slightly long function

### Nesting reduction
- Early returns and guard clauses at function start
- Optional chaining and nullish coalescing when they improve clarity, not to hide missing cases
- `await` only where sequencing matters; use `Promise.all` for independent work
- Flatten deeply nested control flow
- Prefer discriminated unions and pattern checks over nested conditionals
- Sometimes an imperative loop is easier to reason about than a complex chained expression - prefer clarity

### Type design
- Make impossible states difficult or impossible - encode logic and system state in discriminated unions, branded types, and precise object shapes when it makes things clearer
- Reduce booleans - use union states and named modes instead
- Prefer `unknown` at trust boundaries and narrow deliberately; avoid `any` unless the project has no safer interop path
- Avoid non-null assertions and broad type assertions; use narrowing, parser functions, or local guards instead
- Prefer cohesive objects/modules/classes with behavior near the data they validate over freestanding utility clusters that pass the same state around
- Use generics to express real relationships between inputs and outputs, not to make call sites clever
- Prefer deriving types from canonical data or schemas when that avoids drift; avoid deriving so much that the important domain concept becomes unreadable
- Prefer `Map`/`Set` for repeated keyed lookups and membership checks when object/array scans obscure intent or cost
- Avoid mutation across distant functions; mutation is fine when local, obvious, and simpler than copying

### Framework boundaries
- Respect the framework's data-flow model. For React/Next/Svelte/Vue/server frameworks, inspect nearby code and official project conventions before moving logic across client/server, render/action, or request/background boundaries
- Do not move side effects into render paths or constructors
- Keep serialization boundaries explicit: URL params, JSON, form data, cookies, headers, server actions, RPC calls, queues, and database rows should be parsed before use
- Do not paper over hydration, cache, or async ordering issues with assertions or delayed effects

### Error handling
- Do not swallow errors or return vague `null`/`undefined` for failure unless that is the project's established API
- Use the project's error type, Result pattern, schema error handling, or exception convention consistently
- Add useful context at async and boundary layers
- Preserve error surfaces for callers unless the user approved a public API change

### Dead code and suppression audit
- Review `@ts-ignore`, `@ts-expect-error`, eslint disables, and unsafe casts - remove if genuinely unnecessary, and prefer restructuring over suppressing
- Review unused exports and compatibility paths - remove if genuinely dead, don't leave deprecated code in place
- Keep test-only helpers out of production paths

### Verify
- Run repo-specific format, lint, typecheck, and relevant test commands discovered during setup

## Pass 2: Style

Spawn a separate agent focused on style and conventions. Always runs if Pass 1 ran. May also run independently for style-only cleanup even without structural findings.

### Comments
- Inline comments (`//`, `{/* */}`): start lowercase
- Doc comments (`/** */`): start capitalized
- No trailing period on comments
- Only "why" comments, not "what"
- Remove comments that only made sense in context of an agent conversation or that reference code no longer present

### Idiomatic patterns
- Named imports over wildcard namespace imports unless the namespace carries meaning
- Avoid redundant closures: `.map(parseUser)` not `.map((value) => parseUser(value))`
- Prefer `const` over `let`; use `let` only when reassignment is meaningful
- Prefer `??` over `||` for defaulting nullable values
- Prefer `for...of` or a clear loop when chained array methods hide control flow, allocation, or early exits
- Prefer object rest/destructuring when it makes ownership and remaining props clear
- Avoid broad `Record<string, unknown>` shapes once keys are known
- Avoid spreading unvalidated external data into domain objects, props, database writes, or API responses
- Use the project's formatter and import sorter rather than hand-formatting
- Keep test names behavioral; avoid tests that only restate implementation details

### Verify
- Run repo-specific format, lint, typecheck, and relevant test commands discovered during setup

## Refactoring Discipline

- When refactoring code that calls external packages, read the package docs or source to verify behavior - use `/btx`, generated docs, local `node_modules`, or official docs as appropriate
- When code has documented assumptions, trace the data flow backwards to verify callers satisfy them
- Refactoring is an opportunity to catch correctness bugs - question the logic, not just the structure
- Avoid bandaid refactors that preserve a known-wrong model. Churn is acceptable only inside confirmed scope, with preserved external behavior, when it consolidates the domain model, removes competing representations, and leaves the code simpler, more correct, and easier to trace
