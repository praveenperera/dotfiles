# Planport local workflow

Use Planport as the local review surface for visual plans and recaps. It serves
local MDX files, binds to the LAN, and stores review feedback beside the source.
There is no external Plan UI, remote database, or sharing by default. Planport
does not make the coding model local; use a local or otherwise approved model
when that privacy boundary matters.

## Artifact contract

- Use `plans/<slug>/` for a repo-owned artifact. Use a repo-ignored or temporary
  path such as `_scratch/plans/<slug>/` or `/tmp/planport-plans/<slug>/` for
  private scratch.
- Store the document in `plan.mdx`. Add `canvas.mdx`, `prototype.mdx`, and
  `.plan-state.json` only when needed. Planport writes `comments.json` beside
  `plan.mdx`.
- For a recap, set `kind: "recap"` and `localOnly: true` in the supported
  frontmatter or state.
- Keep planning and recap work scoped to these artifact and feedback files.
  Source inspection is read-only. Do not change implementation, tests, config,
  generated source, or dependencies unless the user separately requests that
  work.
- Treat the structured artifact as the source of truth. Update it when scope or
  decisions change; do not leave the real update only in chat.

## Serve and verify

Run:

```bash
env -u PORT planport serve <plan-dir> --open
```

Planport chooses a random available port by default and prints a tokenized LAN
URL. Report that URL as the primary review surface. Use `PORT` or `--port` only
when the user explicitly requests a fixed port. Never commit the tokenized URL
or post it publicly without explicit direction.

If `planport` is unavailable, run `cmd release planport` and retry. When a
browser surface is available, open the printed URL and smoke-test the render.
For headless verification, fetch the printed URL's API endpoint:

```bash
curl '<lan-or-local-url>/api/plan?token=<token>'
```

Confirm the response contains the expected title and files. Use the response to
diagnose a concrete render error when the UI cannot load.

## Feedback loop

Read `comments.json` before editing, after review, after a long pause, and before
handoff. A pasted Planport `Copy` payload is equivalent feedback. Treat its
file, line, text anchor, and comment body as the exact target.

Patch the relevant MDX or state file surgically, preserve unaffected blocks and
visual surfaces, keep serving the same folder, and reload the same review URL.
Feedback authorizes recap or plan artifact edits only. If a comment proposes an
implementation change, capture it in the plan or report it for approval unless
the user has separately requested implementation.
