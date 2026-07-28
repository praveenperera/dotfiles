# ReScript 12 project setup

Use this reference when creating, upgrading, or debugging a ReScript build. It records details
that were verified against ReScript 12 documentation and a working ReScript 12.3 application on
2026-07-27. Recheck the migration guide when the installed major changes.

## Contents

- [Start a new project](#start-a-new-project)
- [Baseline configuration](#baseline-configuration)
- [Scripts](#scripts)
- [Development lifecycle](#development-lifecycle)
  - [Vite](#vite)
  - [Astro](#astro)
- [Generated output](#generated-output)
- [Verification order](#verification-order)

## Start a new project

Prefer the official generator:

```sh
npm create rescript-app@latest
```

For an interactive application, select Vite and React. Preserve the user's package manager.
The current official installation guide requires Node 22 or newer for new projects.

For manual integration, install `rescript` locally and use the current major's configuration
names. ReScript 12 renamed legacy keys:

```text
bs-dependencies      -> dependencies
bs-dev-dependencies  -> dev-dependencies
bsc-flags            -> compiler-flags
```

Some React documentation still shows `bs-dependencies`; use `dependencies` in a ReScript 12
`rescript.json`. Prefer the generated configuration over copied older examples.

## Baseline configuration

Adapt this baseline instead of copying it blindly:

```json
{
  "name": "example-app",
  "namespace": true,
  "sources": [
    {"dir": "src", "subdirs": true},
    {"dir": "test", "type": "dev"}
  ],
  "dependencies": ["@rescript/react"],
  "jsx": {"version": 4},
  "package-specs": {
    "module": "esmodule",
    "in-source": true
  },
  "suffix": ".res.js",
  "warnings": {
    "error": "+A"
  }
}
```

- keep `jsx.version` at `4`; it is the supported transform in ReScript 12
- use ESM for modern Vite, browser, and Node projects
- use `in-source` when the JS toolchain should import generated files beside `.res` sources
- use a namespace when avoiding cross-package module-name collisions matters
- remember that ReScript filenames must be unique within a project
- expose only intentional library APIs with `.resi` files

## Scripts

Use the ReScript 12 command names:

```json
{
  "scripts": {
    "res:build": "rescript",
    "res:watch": "rescript watch",
    "format": "rescript format",
    "format:check": "rescript format --check"
  }
}
```

ReScript 12's canonical build command is `rescript`; older material often says
`rescript build`. Run one initial build before starting the framework so imported `.res.js` files
exist. Compile before the production bundler. Add framework scripts only after choosing and
verifying its development lifecycle.

Run formatter, compiler, typecheck, test, and build scripts that invoke ReScript serially in one
build tree. Concurrent compiler processes can race on locks or let a bundler observe new source
imports before GenType has regenerated their exports.

With pnpm, compiled output imports `@rescript/runtime`. The official guide requires either adding
it as a direct dependency or configuring the documented hoist pattern.

## Development lifecycle

Do not assume that a script named `watch` stays in the foreground or that a framework dev command
owns its process for the entire session. Verify the actual lifecycle:

1. run the repository's development command
2. change a harmless `.res` or `.resi` expression
3. confirm the corresponding `.res.js` file is regenerated
4. confirm the browser receives the change
5. restore the probe and stop the development processes

### Vite

For plain Vite, run `rescript watch` alongside `vite` after confirming both processes remain
foreground jobs. Then a combined lifecycle can use:

```json
{
  "scripts": {
    "predev": "npm run res:build",
    "dev": "concurrently --kill-others \"npm run res:watch\" \"vite\"",
    "build": "npm run res:build && vite build",
    "check": "npm run format:check && npm run res:build && vite build"
  }
}
```

Use `concurrently` only when the edit probe proves both commands have compatible lifecycles and
the repository owns reliable teardown.

### Astro

Keep `predev` and production builds explicit:

```json
{
  "scripts": {
    "predev": "npm run res:build",
    "dev": "astro dev",
    "build": "npm run res:build && astro build",
    "check": "npm run format:check && npm run res:build && astro check && astro build"
  }
}
```

This `dev` command is sufficient only when Astro's Vite lifecycle or another repository-owned
runner also recompiles ReScript. If `rescript watch` or the Astro command does not remain attached
reliably, integrate compilation into that lifecycle. A Vite plugin can watch `*.res` and `.resi`,
serialize invocations of the canonical `rescript` build, reload only after a successful compile,
and surface compiler failures. Do not create competing watcher, process-identity, or teardown
paths when the repository already owns this lifecycle.

## Generated output

For in-source builds, normally ignore:

```gitignore
/lib/
/src/**/*.res.js
/test/**/*.res.js
```

Adjust the suffix and directories to the actual configuration. Do not ignore handwritten JS/TS
adapters. When GenType is enabled, also ignore its configured generated extension, such as
`src/**/*.gen.ts` or `src/**/*.gen.tsx`. Libraries that intentionally publish compiled JavaScript
may need to commit or package generated output; follow that package's release design.

Never edit generated `.res.js`. Inspect it when validating imports, exports, calling conventions,
or framework integration.

Stop framework and Worker watchers before formatting or clean verification when they bundle
in-source generated output. Formatters and compilers can transiently replace `.res.js` or GenType
files, and a live bundler may report missing modules or exports during that replacement even when
the serial build is healthy.

## Verification order

Run these steps serially:

1. run the formatter check
2. run `rescript` and fix the earliest causal error
3. run the bundler build
4. run behavior and boundary tests
5. verify live regeneration when development watcher behavior changed
6. inspect generated JS when an external or framework wrapper behaves unexpectedly
7. use the repository's Worker test harness or Wrangler with an isolated local persistence
   directory when stale development schema state is not part of the test
8. when persisted migrations are in scope, exercise a representative legacy fixture and an empty
   state, and inspect actual columns and constraints instead of trusting only a migration version

## Sources

- https://rescript-lang.org/docs/manual/installation/
- https://rescript-lang.org/docs/manual/build-configuration/
- https://rescript-lang.org/docs/manual/migrate-to-v12/
- https://rescript-lang.org/docs/react/installation/
