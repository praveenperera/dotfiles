---
name: shadcn-baseui
description: >
  shadcn/ui component catalog, CLI commands, patterns, and migration guide.
  ALWAYS use Base UI (@base-ui/react) as the primitive library — Radix UI is
  legacy and should not be used. Use this skill when working with shadcn/ui,
  adding shadcn components, initializing new shadcn projects, or building UI
  with @base-ui/react. Triggers on: shadcn, shadcn/ui, Base UI, @base-ui/react,
  or shadcn component names (Dialog, Select, Popover, etc.) in React/Next.js.
---

# shadcn/ui with Base UI

**IMPORTANT: Always use Base UI (`@base-ui/react`) as the primitive library, NOT Radix UI.**
Radix UI (`@radix-ui/react-*`) is the legacy option. Base UI is the modern replacement.
When initializing projects, adding components, or writing code — always choose Base UI.
Do NOT use `asChild`, `@radix-ui/*` imports, `data-state`, or `--radix-*` CSS variables.

Base UI (MUI team, v1.0 Dec 2025) ships as a single `@base-ui/react` package.
Full shadcn/ui Base UI docs shipped Jan 2026, all blocks for both libraries Feb 2026.

## CLI Quick Reference

```bash
# new project — select a Base UI style (base-vega, base-nova, etc.)
npx shadcn create

# existing project — initialize with Base UI
npx shadcn@latest init

# add components (uses Base UI if components.json style is base-*)
npx shadcn add button dialog select
npx shadcn add --all

# browse available components
npx shadcn list
npx shadcn search [query]

# migrate existing Radix project to Base UI
npx shadcn@latest migrate radix

# add RTL support
npx shadcn@latest migrate rtl

# change icon library
npx shadcn@latest migrate icons
```

### Ensuring Base UI in `components.json`

The `style` field in `components.json` controls which primitive library is used.
**Base UI styles start with `base-`**. Radix styles do NOT have this prefix.

```jsonc
// Base UI (correct)
{ "style": "base-vega" }

// Radix (legacy — do not use)
{ "style": "new-york" }
```

Available Base UI styles: `base-vega`, `base-nova`, `base-maia`, `base-lyra`, `base-mira`.

If a project has a non-`base-*` style, run `npx shadcn@latest migrate radix` to switch to Base UI,
or manually change the `style` field to a `base-*` value and re-add components.

## Visual Styles

Selected during `npx shadcn create`. Can also be set during `init`.

| Style | Character |
|-------|-----------|
| Vega  | Classic shadcn/ui look |
| Nova  | Compact, reduced spacing |
| Maia  | Soft, rounded, generous spacing |
| Lyra  | Sharp, boxy, monospace-friendly |
| Mira  | Dense interface |

## Base UI Core Pattern: `render` Prop

Base UI replaces Radix's `asChild` with a `render` prop for component composition.

**Radix (old):**
```tsx
<Button asChild>
  <a href="/about">About</a>
</Button>
```

**Base UI (new):**
```tsx
<Button render={<a href="/about" />}>
  About
</Button>
```

**Important:** `render` takes a React **element** (`<Link />`) not a **component** (`Link`).
Event handlers merge automatically — no need to forward them manually.

## Import Pattern

All imports use subpath exports from the single `@base-ui/react` package:

```tsx
import { Dialog } from '@base-ui/react/Dialog'
import { Select } from '@base-ui/react/Select'
import { Tooltip } from '@base-ui/react/Tooltip'
```

## Component Selection Guide

| Need | Component |
|------|-----------|
| Modal / popup | Dialog |
| Side panel | Sheet, Drawer |
| Dropdown menu | Dropdown Menu |
| Right-click menu | Context Menu |
| Form select | Select |
| Searchable select | Combobox |
| Command palette | Command |
| Tooltip | Tooltip |
| Notification | Toast (Sonner) |
| Collapsible sections | Accordion |
| Tab navigation | Tabs |
| Date selection | Date Picker |
| Form with validation | Field (new Oct 2025) |
| Loading indicator | Spinner (new Oct 2025) |
| Keyboard shortcut display | Kbd (new Oct 2025) |
| Empty state | Empty (new Oct 2025) |
| Grouped buttons | Button Group (new Oct 2025) |
| Input with addons | Input Group (new Oct 2025) |
| List/card item | Item (new Oct 2025) |

For the full 70+ component catalog with import paths and part names,
see [references/components.md](references/components.md).

## Common Gotchas

- `render` takes an element not a component: `render={<Link />}` not `render={Link}`
- Styling states use boolean data attributes: `[data-open]` instead of `data-state="open"`
- CSS variables use `--base-ui-*` prefix instead of `--radix-*`
- Animations use native CSS transitions. Style `[data-open]` / `[data-closed]` / `[data-entering]` / `[data-exiting]` attributes
- The CLI auto-converts `asChild` to `render` prop when installing — manual copy/paste requires manual conversion
- Positioning components (Tooltip, Popover, Select, etc.) support `inline-start` / `inline-end` for RTL

## Documentation

- shadcn/ui docs: https://ui.shadcn.com/docs
- Component docs: https://ui.shadcn.com/docs/components/[name]
- Base UI docs: https://base-ui.com/react/overview
- Blocks: https://ui.shadcn.com/blocks
- CLI docs: https://ui.shadcn.com/docs/cli
- Changelog: https://ui.shadcn.com/docs/changelog

## Reference Files

- [references/components.md](references/components.md) — full component catalog with categories, imports, and part names
- [references/base-ui-vs-radix.md](references/base-ui-vs-radix.md) — detailed migration guide and API differences (read when migrating or comparing)
