# Standalone HTML recap

Write one complete HTML document. It must open from disk in an ordinary
browser with no host, Planport renderer, or Codex visualize surface.

## File

- Path: `<repo>/_scratch/visual-recap/<slug>.html`
- Slug: concise ASCII, lowercase, hyphenated, from the work unit
- Create `_scratch/visual-recap/` when it does not exist
- Overwrite the same path when revising the same work unit

Read [../assets/recap.css](../assets/recap.css) and inline its full
contents in `<style>`. Do not link to the skill path; the recap must
remain readable after the skill moves.

Load no other local files. Do not use `fetch`, XHR, WebSocket, or
other API calls. Keep the document under 1 MB. Escape `<` in code
excerpts.

Allowed remote scripts, only when needed:

- `https://cdn.jsdelivr.net/npm/mermaid@11.12.1/dist/mermaid.min.js`

Do not load icon fonts, component libraries, or other CDNs.

## Document

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Recap — <short title></title>
  <style>/* inlined recap.css */</style>
</head>
<body>
  <article class="recap">
    <header>
      <p class="kicker">Recap</p>
      <h1>...</h1>
      <p class="scope">PR / branch / commit / range</p>
    </header>
    <!-- headline visual, lede, contracts, tree, key changes -->
  </article>
</body>
</html>
```

Use the recap classes below for chrome. Do not invent a second page
shell, KPI row, or card grid.

## Composition

Choose the smallest composition that makes the change visible.

- Let the headline visual carry the meaning. Do not restate it in
  adjacent prose, callouts, or metric cards.
- Prefer interaction detail over permanent toolbars, repeated
  legends, or stacked copies of the same surface.
- One mechanism per state. Do not invent search, filter, reset, or
  extra pickers.
- Show only values that explain the change. Put them on the mockup,
  diagram, table, or diff. Treat maxima as ceilings, not targets.
- Never invent qualitative scores, status cards, or secondary fact
  grids.
- Use Mermaid or a table when labeled structure is enough. Use HTML
  when the reviewer needs spatial UI, a state switch, or a diff.

## Sections

**Headline visual.** Put mockups or the architecture diagram first.

**Lede.** Optional. Use `<section class="lede">` only when recap
construction requires it.

**Contracts.** Use `<table>` for models and endpoints. Sentence-case
headers. End-align numbers.

**File tree.**

```html
<ul class="tree">
  <li data-change="added"><code>src/new.ts</code> <span class="note">creates the parser</span></li>
  <li data-change="modified"><code>src/lib.rs</code></li>
  <li data-change="removed"><code>src/old.ts</code></li>
  <li data-change="renamed"><code>src/a.ts → src/b.ts</code></li>
</ul>
```

**Key changes.** Use tabs when there are two or more excerpts:

```html
<div class="tabs">
  <div role="tablist">
    <button type="button" role="tab" aria-selected="true" aria-controls="change-1" id="tab-1">src/lib.rs</button>
    <button type="button" role="tab" aria-selected="false" aria-controls="change-2" id="tab-2">src/main.rs</button>
  </div>
  <section role="tabpanel" id="change-1" aria-labelledby="tab-1">...</section>
  <section role="tabpanel" id="change-2" aria-labelledby="tab-2" hidden>...</section>
</div>
```

Use the same tab script for key-change tabs and for mockup state
switches. Include it once after each `.tabs` or `.states` element:

```html
<script>
(() => {
  const root = document.currentScript.previousElementSibling;
  if (!root?.classList.contains("tabs") && !root?.classList.contains("states")) return;
  const tabs = [...root.querySelectorAll('[role="tab"]')];
  const select = (next) => {
    for (const tab of tabs) {
      const on = tab === next;
      tab.setAttribute("aria-selected", on ? "true" : "false");
      document.getElementById(tab.getAttribute("aria-controls")).hidden = !on;
    }
  };
  root.querySelector('[role="tablist"]').addEventListener("click", (event) => {
    const tab = event.target.closest('[role="tab"]');
    if (tab) select(tab);
  });
})();
</script>
```

Place the script immediately after the element it controls. Keep
native tab order. Do not add `tabindex`.

## Diffs and new files

Split before/after:

```html
<figure class="diff">
  <figcaption>src/lib.rs — reject empty slugs</figcaption>
  <div class="split">
    <pre><code>...</code></pre>
    <pre><code>...</code></pre>
  </div>
</figure>
```

Unified hunk:

```html
<figure class="diff">
  <figcaption>src/lib.rs</figcaption>
  <pre class="unified"><code><span class="ctx"> fn parse(input: &str) {</span>
<span class="del">-    let slug = input;</span>
<span class="add">+    let slug = input.trim();</span>
<span class="ctx"> }</span></code></pre>
</figure>
```

New file or large addition:

```html
<figure class="code">
  <figcaption>src/parse.rs — new parser</figcaption>
  <pre><code>...</code></pre>
</figure>
```

Prefix annotation notes with `// !` on their own line above the
relevant code, or put them in a one-line `<p class="note">` under the
figcaption. Do not wrap diffs in extra cards.

## Mockups

A mockup is inspectable product HTML, not a screenshot and not recap
chrome restyled as an app.

- Wrap each surface in `<figure class="mockup" id="unique-id"
  data-surface="browser|mobile|panel|popover">`
- **Contained:** a dialog, popover, panel, or component in its real
  footprint. **Full page:** an app shell or route at full recap width,
  still inside `.mockup`
- Put a short caption on the figure: role and what changed
- Give product windows, cards, menus, and popovers opaque backgrounds
- Scope product CSS to that mockup id (`#signin .field { ... }`)
- Never use recap tokens or classes inside the product surface
  (`--ink`, `--paper`, `.tree`, `.tabs`, `.diff`)
- Define product colors with `light-dark(<light>, <dark>)` unless the
  product has a fixed theme
- Match the product's chrome, navigation, type, and labels. Infer from
  the platform only when the product design is unavailable
- Put app-wide navigation in the product chrome and local controls in
  the changed component. Omit single-option pickers
- Use native `button`, `input`, `select`, and `a`. Do not recreate
  controls
- No decorative shadows, oversized icons, or invented dashboards
- For a flow, keep one mockup and switch states with `.states`. Put
  the tab script immediately after that `.states` element. Stack
  separate figures only when the surfaces themselves differ

```html
<figure class="mockup" id="share" data-surface="panel">
  <figcaption>Viewer — share dialog</figcaption>
  <style>
    #share .product { background: light-dark(#fff, #1b1b1b); color: light-dark(#111, #f3f3f3); }
  </style>
  <div class="states">
    <div role="tablist">
      <button type="button" role="tab" aria-selected="true" aria-controls="share-entry" id="share-tab-entry">Entry</button>
      <button type="button" role="tab" aria-selected="false" aria-controls="share-denied" id="share-tab-denied">Denied</button>
    </div>
    <section role="tabpanel" id="share-entry" aria-labelledby="share-tab-entry">
      <div class="frame"><div class="product">...</div></div>
    </section>
    <section role="tabpanel" id="share-denied" aria-labelledby="share-tab-denied" hidden>
      <div class="frame"><div class="product">...</div></div>
    </section>
  </div>
</figure>
```

## Diagrams

Use a Mermaid `flowchart`, `sequenceDiagram`, or `erDiagram` only for
architecture or data flow. Initialize after the diagrams:

```html
<pre class="mermaid">
flowchart LR
  parser --> store
</pre>
<script src="https://cdn.jsdelivr.net/npm/mermaid@11.12.1/dist/mermaid.min.js"></script>
<script>
  mermaid.initialize({
    startOnLoad: true,
    theme: window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "neutral",
    securityLevel: "strict",
  });
</script>
```

Hand-write SVG when Mermaid cannot express the layout. Size SVG from
its container. Keep labels at least 12px. Pair color with text so
meaning does not depend on color alone.

## Layout

- Design for 736px and remain readable down to 360px
- Stack split diffs, mockups, and contract tables when they no longer
  fit side by side
- No `position: fixed`, viewport-height layouts, or horizontal page
  overflow
- Visible text at least 12px
- Honor `prefers-reduced-motion`

## Verification

Open the written file. Confirm at about 736px and 360px, in light and
dark:

- the headline visual is visible without scrolling past a banner
- file-tree change flags stay distinct
- tab and mockup state switches update the visible panel
- mockup product styles do not leak into the recap chrome
- text, controls, and diffs do not overlap or clip
- Mermaid diagrams, when present, render rather than showing source

Fix broken markup before reporting the path.
