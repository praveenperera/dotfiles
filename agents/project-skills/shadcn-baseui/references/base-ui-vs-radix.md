# Base UI vs Radix UI Migration Guide

## Migration Command

```bash
npx shadcn@latest migrate radix
```

Migrates from individual `@radix-ui/react-*` packages to Base UI.
Recommended approach: gradual component-by-component, not big bang.

## Key Differences

| Aspect | Radix UI | Base UI |
|--------|----------|---------|
| Packages | Multiple `@radix-ui/react-*` | Single `@base-ui/react` |
| Composition | `asChild` prop | `render` prop |
| Open state | `data-state="open"` | `[data-open]` (boolean) |
| Closed state | `data-state="closed"` | `[data-closed]` (boolean) |
| CSS variables | `--radix-*` | `--base-ui-*` |
| Animation | `data-state` selectors | `[data-entering]` / `[data-exiting]` |

## Composition Pattern

### Radix: `asChild`

```tsx
<Dialog.Trigger asChild>
  <Button variant="outline">Open</Button>
</Dialog.Trigger>

<Button asChild>
  <a href="/about">About</a>
</Button>
```

### Base UI: `render`

```tsx
<Dialog.Trigger render={<Button variant="outline" />}>
  Open
</Dialog.Trigger>

<Button render={<a href="/about" />}>
  About
</Button>
```

`render` takes an **element** (`<Link />`) not a **component** (`Link`).
Event handlers merge automatically.

## State Attribute Selectors

### Radix

```css
[data-state="open"] { opacity: 1; }
[data-state="closed"] { opacity: 0; }
```

### Base UI

```css
[data-open] { opacity: 1; }
[data-closed] { opacity: 0; }
```

Boolean attributes — no string matching needed.

## Animation Pattern

### Radix

```css
.dialog-content[data-state="open"] {
  animation: fadeIn 200ms ease;
}
.dialog-content[data-state="closed"] {
  animation: fadeOut 200ms ease;
}
```

### Base UI

```css
.dialog-content {
  transition: opacity 200ms, transform 200ms;
  opacity: 0;
  transform: scale(0.95);
}
.dialog-content[data-open] {
  opacity: 1;
  transform: scale(1);
}
/* entering/exiting states for transition timing */
.dialog-content[data-entering] {
  opacity: 0;
  transform: scale(0.95);
}
```

Base UI provides `[data-entering]` and `[data-exiting]` attributes for
fine-grained transition control during mount/unmount animations.

## CSS Variable Prefix Changes

| Radix | Base UI |
|-------|---------|
| `--radix-accordion-content-height` | `--base-ui-accordion-content-height` |
| `--radix-collapsible-content-height` | `--base-ui-collapsible-content-height` |
| `--radix-popper-available-width` | `--base-ui-popper-available-width` |
| `--radix-popper-available-height` | `--base-ui-popper-available-height` |

Find and replace `--radix-` with `--base-ui-` in CSS/Tailwind config.

## Import Path Changes

### Radix

```tsx
import * as Dialog from '@radix-ui/react-dialog'
import * as Select from '@radix-ui/react-select'
import * as Tooltip from '@radix-ui/react-tooltip'
```

### Base UI

```tsx
import { Dialog } from '@base-ui/react/Dialog'
import { Select } from '@base-ui/react/Select'
import { Tooltip } from '@base-ui/react/Tooltip'
```

Named exports from subpath, not namespace imports.

## Positioning Side Values

Base UI adds logical positioning values for RTL support:

| Radix | Base UI |
|-------|---------|
| `side="left"` | `side="inline-start"` |
| `side="right"` | `side="inline-end"` |

Physical values (`left`, `right`, `top`, `bottom`) still work.
Use logical values for RTL-aware layouts.
