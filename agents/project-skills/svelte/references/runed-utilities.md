# Runed Utilities

Reactive utilities from [runed.dev](https://runed.dev). Check for runed solutions before writing custom utility code.

```bash
npm install runed
```

## Table of Contents

- [State Management](#state-management) - Debounced, PersistedState, Previous, StateHistory, FiniteStateMachine, Context
- [Reactivity](#reactivity) - watch, watchOnce, resource, useSearchParams
- [Debounce & Throttle](#debounce--throttle) - useDebounce, useThrottle
- [Element & DOM](#element--dom) - ElementSize, ElementRect, ScrollState, TextareaAutosize, IsFocusWithin, IsInViewport, activeElement, onClickOutside
- [Observers](#observers) - useResizeObserver, useIntersectionObserver, useMutationObserver
- [Browser & Sensors](#browser--sensors) - useEventListener, useGeolocation, PressedKeys, IsDocumentVisible, IsIdle
- [Animation](#animation) - AnimationFrames
- [Lifecycle & Helpers](#lifecycle--helpers) - IsMounted, useInterval, boolAttr, onCleanup

---

## State Management

### `Debounced`

Reactive debounced state wrapper. Access debounced value via `.current`.

```typescript
import { Debounced } from 'runed';

let search = $state('');
const debounced = new Debounced(() => search, 500);

// debounced.current - the debounced value
// debounced.cancel() - cancel pending update
// debounced.setImmediately(value) - set now, cancel pending
// debounced.updateImmediately() - flush pending now
```

### `PersistedState`

Persist state to localStorage/sessionStorage. Syncs across tabs.

```typescript
import { PersistedState } from 'runed';

const compactMode = new PersistedState('search-compact-mode', false);
compactMode.current = !compactMode.current; // auto-persists

// with options
const prefs = new PersistedState('user-prefs', { theme: 'dark' }, {
  storage: 'session',      // 'local' (default) or 'session'
  syncTabs: true,           // cross-tab sync (default: true)
  serializer: {             // custom serialization
    serialize: superjson.stringify,
    deserialize: superjson.parse,
  },
});

// connection control
prefs.disconnect(); // keep in memory only
prefs.connect();    // re-persist to storage
```

Plain objects/arrays are deeply reactive. Class instances require reassignment.

### `Previous`

Track the previous value of a reactive getter.

```typescript
import { Previous } from 'runed';

let count = $state(0);
const prev = new Previous(() => count);
// count = 0 => prev.current = undefined
// count = 1 => prev.current = 0
```

### `StateHistory`

Undo/redo for reactive state.

```typescript
import { StateHistory } from 'runed';

let count = $state(0);
const history = new StateHistory(() => count, (c) => (count = c));

count = 1; count = 2; count = 3;
history.undo(); // count = 2
history.redo(); // count = 3
history.canUndo; // boolean
history.canRedo; // boolean
history.log;     // array of { snapshot, timestamp }
```

### `FiniteStateMachine`

Strongly-typed state machine with lifecycle hooks.

```typescript
import { FiniteStateMachine } from 'runed';

type States = 'idle' | 'loading' | 'success' | 'error';
type Events = 'fetch' | 'resolve' | 'reject' | 'reset';

const fsm = new FiniteStateMachine<States, Events>('idle', {
  idle: { fetch: 'loading' },
  loading: { resolve: 'success', reject: 'error' },
  success: { reset: 'idle' },
  error: { reset: 'idle' },
});

fsm.send('fetch');             // transitions to 'loading'
fsm.debounce(5000, 'reset');   // delayed event
```

Supports conditional transitions (return `undefined` to cancel), `_enter`/`_exit` lifecycle hooks, and `"*"` wildcard state.

### `Context`

Type-safe wrapper around Svelte's Context API.

```typescript
import { Context } from 'runed';

// define (shared file)
export const themeCtx = new Context<'light' | 'dark'>('theme');

// set (parent component, during init)
themeCtx.set('dark');

// get (child component)
const theme = themeCtx.get();           // throws if not set
const theme = themeCtx.getOr('light');  // fallback if missing
themeCtx.exists();                       // boolean
```

---

## Reactivity

### `watch` / `watch.pre` / `watchOnce`

Explicit dependency control, unlike `$effect` which auto-tracks everything.

```typescript
import { watch } from 'runed';

// single source
let count = $state(0);
watch(() => count, (current, previous) => {
  console.log(`Changed from ${previous} to ${current}`);
});

// multiple sources
watch([() => age, () => name], ([age, name], [prevAge, prevName]) => {
  console.log(`${name} is now ${age}`);
});

// objects - use $state.snapshot for deep comparison
let user = $state({ name: 'bob', age: 20 });
watch(() => $state.snapshot(user), () => {
  console.log(`${user.name} is ${user.age}`);
});
```

Options: `{ lazy: true }` delays initial execution. `watchOnce()` fires once then stops.

### `resource`

Async data fetching with loading/error state. Built on `watch`.

```typescript
import { resource } from 'runed';

let id = $state(1);

const post = resource(
  () => id,
  async (id, prevId, { signal }) => {
    const res = await fetch(`/api/posts?id=${id}`, { signal });
    return res.json();
  },
  { debounce: 300 }
);

post.current;    // fetched data
post.loading;    // boolean
post.error;      // Error | null
post.refetch();  // manually refetch
post.mutate(v);  // directly set value
```

Options: `lazy`, `once`, `initialValue`, `debounce`, `throttle`.

### `useSearchParams`

Reactive, schema-validated URL search params for SvelteKit.

```typescript
import { useSearchParams } from 'runed';
import { z } from 'zod';

const schema = z.object({
  q: z.string().default(''),
  page: z.coerce.number().default(1),
  sort: z.enum(['date', 'relevance']).default('relevance'),
});

const params = useSearchParams(schema, {
  debounce: 500,
  pushHistory: true,
});

// read: params.q, params.page
// write: params.q = 'search term'
// batch: params.update({ q: 'new', page: 1 })
// reset: params.reset()
// for API calls: params.toURLSearchParams()
```

Supports Zod, Valibot, Arktype, or built-in `createSearchParamsSchema`.

**Limitations**: Manual `goto()` + `Debounced` is better when URL updates only on explicit submit, you need dual-debounce patterns, or complex multi-step URL updates.

---

## Debounce & Throttle

### `useDebounce`

Debounced callback function (not state).

```typescript
import { useDebounce } from 'runed';

const debouncedLog = useDebounce(
  () => console.log(count),
  () => 1000  // can be reactive
);

debouncedLog();               // schedule
debouncedLog.cancel();        // cancel
debouncedLog.runScheduledNow(); // flush
debouncedLog.pending;         // boolean
```

### `useThrottle`

Throttled callback function.

```typescript
import { useThrottle } from 'runed';

const throttled = useThrottle(
  () => { throttledSearch = search; },
  () => 1000
);
```

---

## Element & DOM

### `ElementSize`

Reactive element dimensions (width/height).

```typescript
import { ElementSize } from 'runed';

let el = $state<HTMLElement>();
const size = new ElementSize(() => el);
// size.width, size.height
```

### `ElementRect`

Full reactive bounding rect (like `getBoundingClientRect()`).

```typescript
import { ElementRect } from 'runed';

let el = $state<HTMLElement>();
const rect = new ElementRect(() => el);
// rect.width, rect.height, rect.top, rect.left, rect.right, rect.bottom, rect.x, rect.y
```

### `ScrollState`

Track scroll position, direction, edges, with programmatic scrolling.

```typescript
import { ScrollState } from 'runed';

let el = $state<HTMLElement>();
const scroll = new ScrollState({
  element: () => el,
  idle: 200,
  behavior: 'smooth',
});

scroll.x; scroll.y;          // get/set position
scroll.directions;            // active scroll directions
scroll.arrived;               // edge detection
scroll.progress;              // percentage on x/y
scroll.scrollToTop(); scroll.scrollToBottom();
```

### `TextareaAutosize`

Auto-resize textarea to fit content.

```typescript
import { TextareaAutosize } from 'runed';

let el = $state<HTMLTextAreaElement>(null!);
let value = $state('');

new TextareaAutosize({
  element: () => el,
  input: () => value,
  styleProp: 'height',    // or 'minHeight' for grow-only
  maxHeight: 300,
});
```

### `IsFocusWithin`

Track whether focus is inside a container.

```typescript
import { IsFocusWithin } from 'runed';

let form = $state<HTMLFormElement>();
const focusWithin = new IsFocusWithin(() => form);
// focusWithin.current - boolean
```

### `IsInViewport`

Track element visibility (IntersectionObserver).

```typescript
import { IsInViewport } from 'runed';

let el = $state<HTMLElement>();
const inViewport = new IsInViewport(() => el, { once: true });
// inViewport.current - boolean
```

### `activeElement`

Reactive `document.activeElement` with Shadow DOM support.

```typescript
import { activeElement } from 'runed';
// activeElement.current - Element | null
```

### `onClickOutside`

Detect clicks outside an element.

```typescript
import { onClickOutside } from 'runed';

let menuRef = $state<HTMLElement>()!;
let isOpen = $state(false);

// always active
onClickOutside(() => menuRef, () => { isOpen = false; });

// controlled
const handler = onClickOutside(() => menuRef, () => { isOpen = false; }, {
  immediate: false,
});
handler.start(); handler.stop(); // handler.enabled
```

---

## Observers

### `useResizeObserver`

```typescript
import { useResizeObserver } from 'runed';

let el = $state<HTMLElement | null>(null);
const { stop } = useResizeObserver(() => el, (entries) => {
  const { width, height } = entries[0].contentRect;
});
```

### `useIntersectionObserver`

```typescript
import { useIntersectionObserver } from 'runed';

let target = $state<HTMLElement | null>(null);
const observer = useIntersectionObserver(() => target, (entries) => {
  if (entries[0]?.isIntersecting) { /* visible */ }
}, { once: true });

observer.pause(); observer.resume(); observer.stop();
```

### `useMutationObserver`

```typescript
import { useMutationObserver } from 'runed';

let el = $state<HTMLElement | null>(null);
const { stop } = useMutationObserver(() => el, (mutations) => {
  mutations.forEach((m) => console.log(m.attributeName));
}, { attributes: true });
```

---

## Browser & Sensors

### `useEventListener`

Typed event listener with automatic cleanup.

```typescript
import { useEventListener } from 'runed';

useEventListener(() => document.body, 'click', () => clicks++);
```

### `useGeolocation`

```typescript
import { useGeolocation } from 'runed';

const location = useGeolocation({ immediate: true });
// location.position.coords, location.error, location.isPaused
// location.pause(), location.resume()
```

### `PressedKeys`

Track currently pressed keys.

```typescript
import { PressedKeys } from 'runed';

const keys = new PressedKeys();
const isCmdK = $derived(keys.has('meta', 'k'));

keys.onKeys(['meta', 'k'], () => {
  console.log('Command palette');
});
```

### `IsDocumentVisible`

Track tab visibility.

```typescript
import { IsDocumentVisible } from 'runed';

const visible = new IsDocumentVisible();
// visible.current - true when tab is visible
```

### `IsIdle`

Track user inactivity.

```typescript
import { IsIdle } from 'runed';

const idle = new IsIdle({ timeout: 60000 });
// idle.current - boolean
// idle.lastActive - timestamp
```

---

## Animation

### `AnimationFrames`

Declarative `requestAnimationFrame` with FPS limiting.

```typescript
import { AnimationFrames } from 'runed';

const anim = new AnimationFrames(
  ({ delta }) => { /* called each frame */ },
  { fpsLimit: () => 60 }
);
// anim.fps, anim.running, anim.delta
```

---

## Lifecycle & Helpers

### `IsMounted`

```typescript
import { IsMounted } from 'runed';

const mounted = new IsMounted();
// mounted.current - false initially, true after mount
```

### `useInterval`

Reactive `setInterval` with pause/resume/reset.

```typescript
import { useInterval } from 'runed';

const interval = useInterval(() => 1000, {
  immediate: true,
  callback: (count) => console.log(`Tick #${count}`),
});
// interval.counter, interval.isActive
// interval.pause(), interval.resume(), interval.reset()
```

### `boolAttr`

Convert boolean to HTML attribute value (`true` -> `""`, `false` -> `undefined`).

### `onCleanup`

Register cleanup functions within reactive contexts.

---

## When NOT to Use Runed

- Simple local state -> `$state`
- Computed values -> `$derived`
- One-time data fetch on mount -> `onMount` + async
- Global shared state -> rune-based store pattern
