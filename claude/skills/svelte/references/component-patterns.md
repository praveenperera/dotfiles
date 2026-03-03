# Component Patterns

## Props Interface

```svelte
<script lang="ts">
  interface Props {
    value?: string;
    mode?: SearchMode;
    class?: string;
    onsubmit?: (query: string, mode: SearchMode) => void;
    onchange?: (query: string) => void;
  }

  let {
    value = $bindable(''),
    mode = $bindable('hybrid'),
    class: className,
    onsubmit,
    onchange,
  }: Props = $props();
</script>
```

## Callback Props vs `$bindable`

**Prefer callback props** for child-to-parent communication:

```svelte
<SearchBar value={query} onchange={(v) => query = v} />
```

**Use `$bindable`** only when two-way binding genuinely simplifies code (form inputs):

```svelte
<SearchBar bind:value={query} />
```

### Callback naming

- Lowercase: `onsubmit`, `onchange`, `onmodechange`, `onplay`
- Invoke with optional chaining: `onsubmit?.(value, mode)`

## Extending HTML Attributes

```svelte
<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    variant?: ButtonVariant;
    size?: ButtonSize;
  }

  let { variant = 'default', size = 'default', children, ...restProps }: Props = $props();
</script>

<button {...restProps}>
  {@render children?.()}
</button>
```

## Module-Level Exports

Use `<script lang="ts" module>` for exports that don't depend on component instance:

```svelte
<script lang="ts" module>
  export const buttonVariants = tv({
    base: 'inline-flex items-center justify-center rounded-md',
    variants: {
      variant: {
        default: 'bg-primary text-primary-foreground',
        outline: 'border border-input bg-background',
      },
      size: {
        default: 'h-10 px-4 py-2',
        sm: 'h-9 px-3',
        lg: 'h-11 px-8',
      },
    },
    defaultVariants: { variant: 'default', size: 'default' },
  });

  export type ButtonVariant = VariantProps<typeof buttonVariants>['variant'];
</script>

<script lang="ts">
  // component logic
</script>
```

## Class Merging with `cn()`

```typescript
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

Usage:

```svelte
<div class={cn(
  'w-full px-3 py-2 rounded',
  isActive && 'bg-accent',
  className
)}>
```

## Tailwind Variants

Use `tailwind-variants` for component variant definitions:

```typescript
import { tv, type VariantProps } from 'tailwind-variants';

const badge = tv({
  base: 'inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold',
  variants: {
    variant: {
      default: 'bg-primary text-primary-foreground',
      secondary: 'bg-secondary text-secondary-foreground',
      destructive: 'bg-destructive text-destructive-foreground',
      outline: 'text-foreground border',
    },
  },
  defaultVariants: { variant: 'default' },
});
```

## Semantic Colors

```svelte
<div class="bg-background text-foreground">
<div class="bg-primary text-primary-foreground">
<div class="bg-secondary text-secondary-foreground">
<div class="bg-muted text-muted-foreground">
<div class="bg-destructive text-destructive-foreground">
<div class="border-border">
<input class="border-input focus:ring-ring">
```

## Icons

Use Lucide from `@lucide/svelte`. Browse at https://lucide.dev/icons.

```svelte
<script lang="ts">
  import Search from '@lucide/svelte/icons/search';
  import Play from '@lucide/svelte/icons/play';
</script>

<Search class="w-4 h-4" />
<Play class="w-4 h-4" fill="currentColor" />
```

Do not create custom SVG icons when a Lucide icon exists.

## bits-ui Headless Components

Use `bits-ui` for accessible headless components:

```svelte
<script lang="ts">
  import { Command } from 'bits-ui';
</script>

<Command.Root>
  <Command.Input placeholder="Search..." />
  <Command.List>
    <Command.Group>
      <Command.GroupHeading>Results</Command.GroupHeading>
      <Command.Item>Item 1</Command.Item>
    </Command.Group>
  </Command.List>
</Command.Root>
```

## CSS Gotcha: Flexbox + Absolute Overlay

An absolute overlay inside a flex child stretches beyond the expected bounds:

```svelte
<!-- BUG: overlay taller than image -->
<div class="flex gap-4">
  <div class="relative">
    <img class="h-20 w-20" />
    <div class="absolute inset-0 bg-black/40" />
  </div>
  <div class="flex-1"><!-- taller content --></div>
</div>

<!-- FIX: self-start prevents stretch -->
<div class="flex gap-4">
  <div class="relative self-start">
    <img class="h-20 w-20" />
    <div class="absolute inset-0 bg-black/40" />
  </div>
  ...
</div>
```

## View Transitions

```typescript
import { onNavigate } from '$app/navigation';

onNavigate((navigation) => {
  if (!document.startViewTransition) return;
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

  return new Promise((resolve) => {
    document.startViewTransition(async () => {
      resolve();
      await navigation.complete;
    });
  });
});
```
