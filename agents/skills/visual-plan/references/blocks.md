# Planport block components

Read this before authoring structured Planport MDX. These are the local renderer
components this installation can render. Use the canonical tags for new plans;
legacy and alias tags are tolerated only so old plans still open.

## Canonical authoring map

| Conceptual block | Authoring tag / shape |
| --- | --- |
| `rich-text` | `<RichText id="..." markdown="..." />` or children |
| `annotated-code` | `<AnnotatedCode id="..." filename="..." language="..." code={...} annotations={[...]} />` |
| `code` | `<Code id="..." filename="..." language="..." code={...} />` |
| `diff` | `<Diff id="..." filename="..." language="..." before={...} after={...} mode="split" annotations={[...]} />` |
| `file-tree` | `<FileTree id="..." files={[{ path, change, note }]} />` |
| `data-model` | `<DataModel id="..." entities={[...]} relations={[...]} />` |
| `api-endpoint` | `<Endpoint id="..." method="GET" path="/api/..." params={[...]} request={{...}} responses={[...]} examples={[...]} />...description...</Endpoint>` |
| `question-form` | `<QuestionForm id="..." title="Open Questions" questions={[...]} />` |
| `custom-html` | `<CustomHtml id="..." html="..." />` for bounded fragments only |
| `json-explorer` | `<Json id="..." label="..." value={{...}} />` |
| `diagram` | `<Diagram id="..." data={{ html, css }} />` |
| `mermaid` | `<Mermaid id="..." source={`graph TD\nA-->B`} />` or children |
| `columns` | `<Columns id="..."><Column label="Before">...</Column><Column label="After">...</Column></Columns>` |
| `tabs` | `<TabsBlock id="..." tabs={[{ id, label, blocks: [{ id, type, summary, data }] }]} />` |
| `table` | `<Table id="..." columns={[...]} rows={[...]} />` |
| `checklist` | `<Checklist id="..." items={[{ id, label, checked }]} />` |
| `callout` | `<Callout id="..." tone="decision">...</Callout>` |
| `wireframe` | `<WireframeBlock id="..."><Screen surface="browser" html={`<div>...</div>`} /></WireframeBlock>` |
| `canvas` | `canvas.mdx` with `<DesignBoard>`, `<Section>`, `<Artboard>`, `<Annotation>`, and `<Connector>` |
| `prototype` | `prototype.mdx` with `<Prototype>` and `<PrototypeScreen>` |

## Canvas tags

Use these in `canvas.mdx` for the top static review surface:

- `<DesignBoard id="..." title="...">...</DesignBoard>` wraps the canvas.
- `<Section id="..." title="...">...</Section>` groups related frames.
- `<Artboard id="..." surface="desktop|browser|mobile|panel|popover">...</Artboard>` holds one inspectable frame. It can contain `<Screen ... />` or wireframe HTML.
- `<Annotation id="..." targetId="..." placement="right|left|top|bottom">...</Annotation>` is a plain designer note anchored to a target frame/control.
- `<Connector id="..." from="..." to="..." label="..." />` draws intentional flow or dependency links between canvas items.

Keep canvas annotations and connectors out of document prose. Use document
`diagram`, `data-model`, `api-endpoint`, and `file-tree` blocks for technical
evidence below the visual surface.

## Prototype tags

Use these in `prototype.mdx` when the reviewer needs to operate a flow:

- `<Prototype id="..." title="...">...</Prototype>` wraps the interactive surface.
- `<PrototypeScreen id="..." surface="browser|desktop|mobile|panel|popover" html={`...`} />` defines one screen.
- Controls inside prototype screen HTML may use `data-goto="screen-id"` to move to another screen.

Keep prototype labels, ids, and visible states aligned with the canvas artboards
when both surfaces exist.

## Nested tab block types

`TabsBlock` does not have nested `<Tab>` elements. Its `tabs` prop is one array;
each tab has `blocks`, and each nested block has a lowercase `type`. Supported
nested types are `api-endpoint`, `annotated-code`, `code`, `custom-html`,
`data-model`, `diff`, `file-tree`, `json-explorer`, `question-form`,
`rich-text`, `table`, and `wireframe`.

## Required shape notes

- Every component-style block needs a stable `id` unique across the plan folder.
- `Endpoint` is the canonical API tag. Do not author `ApiEndpoint`.
- `TabsBlock` is the canonical tabs tag. Do not author `Tabs`.
- `Json` is the canonical JSON explorer tag. Do not author `JsonExplorer`.
- `CustomHtml` is the canonical custom HTML tag. Do not author `CustomHTML`.
- `QuestionForm` questions need `id`, `title`, and `mode`; options need `id` and `label`.
- `Checklist` items need `id` and `label`.
- `Endpoint` prose description belongs in children, not in a `description` prop. Examples should be single parseable JSON strings.
- `Screen html` should be a quoted string or static template literal, not a dynamic variable.

## Legacy compatibility

Planport can render these old names for existing plans, but new plans should not
emit them:

- `<CodeTabs>`: use `TabsBlock` with nested `code`, `diff`, or `annotated-code` blocks.
- `<ImplementationMap>`: use `FileTree` plus nearby prose or `AnnotatedCode`.
- `<LegacyWireframe>` / `<Wireframe>` with kit-tree `screen` arrays: use `WireframeBlock` and `Screen html`.
- Alias tags `<ApiEndpoint>`, `<Tabs>`, `<JsonExplorer>`, and `<CustomHTML>`: use their canonical tags above.
