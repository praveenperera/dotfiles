---
name: aps
description: Search and manage academic literature with the local `aps` CLI across unified Semantic Scholar, OpenAlex, and ChinaRxiv discovery; paper, citation, reference, and author lookup; repeatable scan manifests; PDF download; and a local full-text/vector library. Use for scholarly literature discovery, citation-graph research, paper identifiers, author metrics, systematic update scans, or local paper-corpus work. Do not use for ordinary web search or non-academic sources.
---

# aps — Academic Paper Search

Use `aps` as the execution surface. Start with `aps --help`, then read the selected
subcommand's `--help` before relying on a flag: installed binaries can lag the source or this
skill. Do not infer flags from examples.

## Route the task

1. Use unified `aps search` for general discovery.
2. Use unified `aps paper`, `citations`, `references`, or `author` for ordinary lookups.
3. Switch to `aps s2` or `aps oa` only for a provider-specific capability or filter.
4. Use `aps scan` for repeatable, date-windowed discovery with persistent seen-state.
5. Use `aps download` or `aps lib` when the user wants PDFs or local-corpus search.
6. Add `-F json` when parsing, joining, deduplicating, or automating results.

Read [references/cli.md](references/cli.md) for command syntax, identifier formats, scan
manifests, JSON output, authentication, and local-library operations. Read
[references/research-strategy.md](references/research-strategy.md) for query design,
provider selection, citation traversal, and evidence handling.

## Default discovery

Run a focused unified search first:

```bash
aps search "retrieval augmented generation evaluation" --since 2024
```

Unified search merges Semantic Scholar (S2), OpenAlex (OA), and ChinaRxiv. ChinaRxiv is a
source inside unified commands, not an `aps chinarxiv` command. By default, preprints remain
eligible. When supported by the installed binary, `--skip-preprint` (and its documented
aliases) excludes ChinaRxiv plus arXiv-like S2/OA records.

Do not claim that every filter applies equally to every source. In the current implementation:

- year/since filters can constrain all three search sources
- citation filters apply to S2/OA; a positive minimum-citation filter omits ChinaRxiv
- open-access filtering maps to source-specific availability signals
- unified ranking and deduplication can hide provider ordering

If the installed `aps search --help` lacks a required unified option, report the mismatch and
use available backend commands rather than pretending the option worked.

## Provider capabilities

| Need | Command | Why |
| --- | --- | --- |
| broad discovery | `aps search` | merged S2/OA/ChinaRxiv results |
| similar papers | `aps s2 recommend` | S2 recommendation graph/model |
| passage search | `aps s2 snippets` | S2ORC snippets |
| title match | `aps s2 match` | S2 title matching |
| venue/publication type | `aps s2 search` | S2-specific filters |
| institution/source/topic/raw filters | `aps oa search` | OA structured filters |
| institutions or topics | `aps oa institutions`, `aps oa topics` | OA entities |
| aggregations | `aps oa group-by` | OA group-by endpoint |
| ChinaRxiv paper detail | `aps paper <china-id>` | unified lookup only |

Unified citations, references, and authors combine S2 and OA; ChinaRxiv does not supply those
unified graph/author results. Use more than one provider when ranking or coverage matters,
because corpus coverage, metadata, identifiers, and citation counts differ.

## Paper identifiers

Prefer DOI for cross-provider work. Preserve provider prefixes when ambiguity matters.
Unified paper lookup detects DOI, S2 ID, OA work ID, arXiv ID, and ChinaRxiv ID. Accepted
ChinaRxiv forms in the current implementation include `chinaxiv-202606.00025`, the bare
`202606.00025`, and an `https://chinaxiv.org/abs/...` URL.

Top-level download resolves DOI-backed DOI/S2/OA/arXiv identifiers before adding a paper to
the local library. Current ChinaRxiv records do not expose a DOI to that downloader; use
`aps paper <id>` to obtain source/PDF URLs and explain this limitation.

## Repeatable scans

Use a scan when the user wants recurring discovery rather than an ad hoc query:

```bash
aps scan \
  --manifest scans/topic.json \
  --from-date 2026-07-01 \
  --to-date 2026-07-13 \
  --seen-file .cache/aps/topic-seen.json \
  -F json
```

The date window is inclusive. The manifest and seen-state are versioned JSON. Reuse the same
seen file for incremental scans; choose a different file for an independent history. `aps`
creates a missing seen file and atomically updates it after successful output. Do not hand-edit
seen-state unless repairing it deliberately.

## Local library

Use `aps lib search` for already-downloaded papers:

- `hybrid` (default): normal concept-plus-keyword questions
- `fts`: exact names, phrases, or identifiers
- `semantic`: paraphrases and vocabulary mismatch

Search without tags first; add tag filters only when unrelated material overwhelms results.
Use `aps lib read <doi>` for full context after passage search identifies a candidate. Keep
local-library claims separate from remote search claims: a missing local hit only means the
paper is absent from the downloaded/indexed corpus.

## Research integrity

- Treat search results as leads, not evidence that a paper supports a claim.
- Open details or full text before summarizing methods, results, or limitations.
- Preserve DOI/provider IDs when deduplicating and cite the paper, not the search result.
- Distinguish no result from source failure, authentication failure, or rate limiting.
- Never invent metadata when providers disagree; report the disagreement or verify it.
- For absence claims, search synonyms and adjacent terminology, then qualify the conclusion.

## Authentication and failures

Run `aps status` before diagnosing coverage or throttling. `aps login` persists available
credentials from the documented environment variables. Never print secret values. Provider
failure is not evidence of no literature; unified search can return partial results while
warning that one source failed. If all relevant sources fail, surface the error and next step.
