# Svelte 5 Runes Reference

## `$state` - Reactive State

```svelte
<script lang="ts">
  let results = $state<SearchResponse | null>(null);
  let isLoading = $state(false);
  let error = $state<string | null>(null);
  let items = $state<{ text: string; count: number }[]>([]);
  let copied = $state<'link' | 'nostr' | 'embed' | null>(null);
</script>
```

Always type-annotate `$state` when the type isn't obvious from the initial value.

## `$derived` - Computed Values

```svelte
<script lang="ts">
  import { page } from '$app/state';

  // from URL params
  const searchParams = $derived(page.url.searchParams);
  const query = $derived(searchParams.get('q') ?? '');
  const mode = $derived((searchParams.get('mode') as SearchMode) ?? 'hybrid');

  // from route params
  const slug = $derived(page.params.slug);

  // from other state
  const hasResults = $derived(results.length > 0);
  const filteredEpisodes = $derived(
    showTranscribedOnly ? episodes.filter((e) => e.is_transcribed) : episodes
  );
</script>
```

### `$derived.by()` for complex computations

```svelte
<script lang="ts">
  const wordPositions = $derived.by((): WordPosition[] => {
    // complex algorithm here
    return positions;
  });
</script>
```

## `$effect` - Side Effects

```svelte
<script lang="ts">
  $effect(() => {
    if (query) {
      fetchResults();
    } else {
      results = null;
    }
  });
</script>
```

**Warning:** Don't read and write the same `$state` in an effect - see [gotchas.md](gotchas.md).

## `$props` and `$bindable`

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
    value = $bindable(''),      // two-way binding
    mode = $bindable('hybrid'),
    placeholder = 'Search...',
    class: className,           // rename reserved word
    onsubmit,
    onchange
  }: Props = $props();
</script>

<!-- Parent -->
<SearchBar bind:value={searchInput} onsubmit={handleSearch} />
```

**Use `$bindable` sparingly.** Overuse makes data flow unpredictable. Prefer callback props (`onchange`, `onsubmit`) for most child-to-parent communication. Reserve `$bindable` for form inputs.

### Extending HTML element attributes

```svelte
<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';

  interface Props extends HTMLButtonAttributes {
    variant?: ButtonVariant;
    size?: ButtonSize;
  }

  let { variant = 'default', size = 'default', ...restProps }: Props = $props();
</script>

<button {...restProps}>
  {@render children?.()}
</button>
```

## `{@render children()}`

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children?: Snippet } = $props();
</script>

<div>
  {@render children?.()}
</div>
```

## Rune-Based Stores

For global state shared across components. File must be `.svelte.ts`.

### Class-based (recommended)

V8 optimizes classes better than closures with getters:

```typescript
// player.svelte.ts
class PlayerStore {
  track = $state<Track | null>(null);
  isOpen = $state(false);
  isPlaying = $state(false);

  play(track: Track) {
    this.track = track;
    this.isOpen = true;
    this.isPlaying = true;
  }

  toggle() {
    this.isPlaying = !this.isPlaying;
  }
}

export const playerStore = new PlayerStore();
```

### Function-based

Requires getters since you can't export reassignable state:

```typescript
// player.svelte.ts
function createPlayerStore() {
  let state = $state<PlayerState>({
    track: null,
    isOpen: false,
    isPlaying: false,
  });

  return {
    get track() { return state.track; },
    get isOpen() { return state.isOpen; },
    get isPlaying() { return state.isPlaying; },

    play(track: Track) {
      state.track = track;
      state.isOpen = true;
      state.isPlaying = true;
    },
    toggle() {
      state.isPlaying = !state.isPlaying;
    },
  };
}

export const playerStore = createPlayerStore();
```

### SSR Warning

Global state persists across requests on the server, which can leak data between users. For SSR-safe state, use SvelteKit's `event.locals` or Svelte context. Client-only state (like audio player) is safe as a global.

## When to Use What

| Need | Use |
|---|---|
| Local component state | `$state` |
| Computed from other state | `$derived` |
| Side effects, data fetching | `$effect` |
| Global state across pages | Rune-based store (`.svelte.ts`) |
| Shareable/bookmarkable state | URL params |
