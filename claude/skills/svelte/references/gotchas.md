# Svelte 5 Gotchas

## `effect_update_depth_exceeded`

An `$effect` reads and writes the same `$state`, creating an infinite loop.

### First: Avoid Effects for State Updates

Effects are escape hatches for side effects (DOM, analytics), **not** state synchronization:

1. **Use `$derived`** for computed values:
   ```svelte
   let doubled = $derived(count * 2);  // not $effect + $state
   ```

2. **Use callback props** instead of syncing state:
   ```svelte
   <input oninput={(e) => onchange(e.target.value)} />
   ```

3. **Use regular `let`** for non-reactive tracking variables

### When Effects Are Necessary

#### Use regular `let` instead of `$state`

If only needed for comparison/tracking:

```svelte
let lastKey = '';  // not $state - won't trigger effect

$effect(() => {
  const currentKey = getKey(query);
  if (currentKey !== lastKey) {
    lastKey = currentKey;
    doSomething();
  }
});
```

#### Use `untrack()` for the read

```svelte
import { untrack } from 'svelte';

let prevKey = $state('');

$effect(() => {
  const currentKey = getKey(query);
  const lastKey = untrack(() => prevKey);
  if (currentKey !== lastKey) {
    prevKey = currentKey;
    doSomething();
  }
});
```

#### Add guards to prevent unnecessary writes

```svelte
$effect(() => {
  const current = untrack(() => localValue);
  if (urlValue !== current) {
    localValue = urlValue;
  }
});
```

### Decision table

| Scenario | Solution |
|---|---|
| Tracking previous value for comparison | Regular `let` |
| Syncing external state to local | `untrack()` + guard |
| Auto-selecting based on results | `untrack()` for current state |
| Version counters for async | Regular `let` |

---

## `onMount` vs `$effect` for Data Loading

### Use `onMount` for initial data loads

```svelte
// CORRECT: runs once on mount
onMount(async () => {
  podcasts = await api.listPodcasts();
});

// WRONG: re-runs on unrelated state changes, race conditions
$effect(() => {
  api.listPodcasts().then(p => { podcasts = p; });
});
```

### Use `$effect` for reactive data fetching

When data should refetch based on changing params:

```svelte
$effect(() => {
  const currentSlug = slug;
  if (currentSlug) {
    fetchSpeaker(currentSlug);
  }
});
```

### Quick reference

| Pattern | Use |
|---|---|
| Initial page data (no dependencies) | `onMount` |
| Data that refetches on param change | `$effect` with explicit dependency |
| Data that refetches on user input | `$effect` with debounced input |
| Processing data after async load | Do it in `onMount` after `await` |

---

## Race Conditions

### The problem

`$effect` processes data that's still loading:

```svelte
// BUG: filterPodcasts runs before podcasts loads
onMount(async () => {
  podcasts = await api.listPodcasts();
});

$effect(() => {
  if (query) filterPodcasts(query); // podcasts is empty!
});
```

**Fix:** Process in `onMount` after data loads:

```svelte
onMount(async () => {
  podcasts = await api.listPodcasts();
  if (searchInput) filterPodcasts(searchInput);
});
```

### Version counter pattern

For concurrent async operations, use a version counter to discard stale results:

```svelte
let searchVersion = 0;

$effect(() => {
  const version = ++searchVersion;
  const q = debouncedQuery.current;
  if (!q) return;

  fetchResults(q).then((results) => {
    if (version === searchVersion) {
      searchResults = results;
    }
  });
});
```

---

## SSR State Leak

Global stores persist across requests on the server, leaking data between users.

**Safe:** Client-only state (audio player, UI preferences)

**Unsafe:** User-specific data in global stores

**Fix:** Use `event.locals` or Svelte context for SSR-safe state:

```svelte
<!-- +layout.svelte -->
<script lang="ts">
  import { setContext } from 'svelte';

  let { data, children } = $props();
  setContext('user', data.user);
</script>
```

---

## Prefer `$derived` Over Effect + State

```svelte
// BAD: unnecessary effect
let doubled = $state(0);
$effect(() => { doubled = count * 2; });

// GOOD: derived
const doubled = $derived(count * 2);
```

If the value is purely computed from other reactive values, always use `$derived`.
