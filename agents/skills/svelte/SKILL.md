---
name: svelte
description: |
  Svelte 5 and SvelteKit development with runes, runed utilities, and component patterns.
  Use when: (1) Writing or editing .svelte or .svelte.ts files, (2) Working with Svelte 5 runes ($state, $derived, $effect, $props, $bindable),
  (3) Using the runed package (Debounced, PersistedState, onClickOutside, watch, resource, etc.),
  (4) Building SvelteKit pages or layouts, (5) Creating reactive stores or component libraries,
  (6) User mentions Svelte, SvelteKit, runes, runed, or asks about Svelte 5 patterns.
---

# Svelte 5 + SvelteKit

**Stack:** SvelteKit 2 + Svelte 5 (runes) + TypeScript + Tailwind CSS

## Quick Reference

| Need | Use |
|---|---|
| Local component state | `$state` |
| Computed from other state | `$derived` / `$derived.by()` |
| Side effects, data fetching | `$effect` |
| Component props | `$props()` with `interface Props` |
| Two-way binding (sparingly) | `$bindable` |
| Global state across pages | Class-based rune store (`.svelte.ts`) |
| Shareable/bookmarkable state | URL params via `$app/state` |
| Debounced input | `Debounced` from runed |
| Persistent preferences | `PersistedState` from runed |
| Click outside detection | `onClickOutside` from runed |
| Reactive async data | `resource` from runed |
| Explicit watch with deps | `watch` from runed |
| Schema-validated URL params | `useSearchParams` from runed |

## Key Conventions

- **Callback props over `$bindable`**: Use `onchange`, `onsubmit` for child-to-parent communication. Reserve `$bindable` for form inputs where two-way binding genuinely simplifies code
- **Class-based stores**: Prefer class with `$state` fields over function-based (V8 optimizes classes). File must be `.svelte.ts`
- **URL state**: Shareable data (search query, filters, pagination) in URL params. Local-only data (loading, dropdowns) in `$state`
- **Icons**: `@lucide/svelte` - import from `@lucide/svelte/icons/icon-name`
- **Styling**: `cn()` helper (clsx + tailwind-merge) for class merging. Semantic color tokens (`bg-background`, `text-foreground`, `bg-primary`)
- **Variants**: `tailwind-variants` (`tv()`) for component variant definitions
- **Headless UI**: `bits-ui` for accessible components (Command, Dialog, etc.)
- **Children**: `{@render children?.()}` not `<slot />`
- **Event syntax**: `onclick={handler}` not `on:click`
- **Check runed first**: Before writing custom utilities for debouncing, persistence, element tracking, click-outside, etc., check if runed already provides it

## Reference Files

Read these based on what you're working on:

- **[svelte5-runes.md](references/svelte5-runes.md)** - Read when working with `$state`, `$derived`, `$effect`, `$props`, stores, or any runes patterns
- **[runed-utilities.md](references/runed-utilities.md)** - Read when you need reactive utilities (debounce, persistence, element tracking, observers, keyboard, geolocation, etc.) or before writing custom utility code
- **[component-patterns.md](references/component-patterns.md)** - Read when creating or modifying Svelte components (props, events, styling, icons, variants)
- **[gotchas.md](references/gotchas.md)** - Read when debugging effect loops, SSR issues, race conditions, or when using `$effect` with state updates
