---
name: hyperframes-registry
description: Discover, install, wire, and contribute HyperFrames registry blocks and components. Use for hyperframes catalog or add, tag-based installs, hyperframes.json paths, block sub-composition wiring, component snippet merging, registry manifests, demos, templates, and upstream registry contributions.
---

# HyperFrames Registry

Use registry items instead of recreating common compositions or effects:

- **Blocks** are standalone sub-compositions with their own dimensions, duration, and timeline. Mount them with `data-composition-src`.
- **Components** are effect snippets without independent dimensions. Merge their HTML, CSS, and optional JavaScript into a host composition.

## Workflow

1. Discover items with `npx hyperframes catalog --json` or the interactive `--human-friendly` picker.
2. Install an item or tag with `npx hyperframes add <name-or-tag>`.
3. Read the installed file and the CLI's include snippet.
4. Wire blocks as sub-compositions or merge component snippets into the host.
5. Run `npx hyperframes lint`, `npx hyperframes check`, and `npx hyperframes preview`.

```bash
npx hyperframes catalog --type block --tag transition --json
npx hyperframes catalog --human-friendly
npx hyperframes add data-chart
npx hyperframes add grain-overlay
npx hyperframes add captions                # install every block matching the tag
npx hyperframes add shimmer-sweep --dir .
npx hyperframes add data-chart --json --no-clipboard
```

`add` accepts a block or component name, or a tag that installs every matching block. It does not install examples; scaffold those with `npx hyperframes init <dir> --example <name>`.

Default install paths are `compositions/<name>.html` for blocks, `compositions/components/<name>.html` for components, and `assets/` for assets. Override them in `hyperframes.json`.

For blocks, ensure the host's `data-composition-id` matches the block's internal ID, then set `data-composition-src`, `data-start`, `data-duration`, `data-track-index`, `data-width`, and `data-height` as the host timeline requires. For components, copy only the snippet's needed HTML, styles, script, and timeline calls; resolve IDs and selectors against the host.

## Routing

| Task | Read |
| --- | --- |
| Choose items or inspect manifests | [discovery.md](references/discovery.md) |
| Configure install targets | [install-locations.md](references/install-locations.md) |
| Wire a block | [wiring-blocks.md](references/wiring-blocks.md) and [add-block.md](examples/add-block.md) |
| Merge a component snippet | [wiring-components.md](references/wiring-components.md) and [add-component.md](examples/add-component.md) |
| Understand component demos | [demo-html-pattern.md](references/demo-html-pattern.md) |
| Contribute an upstream item | [contributing.md](references/contributing.md) |
| Start from a registry template | [templates.md](references/templates.md) |

Run `npx hyperframes catalog --json` for the current manifest rather than relying on a static item list. Inspect `npx hyperframes add --help` before using release-sensitive flags.
