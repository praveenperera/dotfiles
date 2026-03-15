---
name: aps
description: Academic paper search CLI (Semantic Scholar, OpenAlex & local library). Use when the user needs to find papers, look up citations, get paper details, search authors, download papers, or do any academic research task.
---

# aps — Academic Paper Search

`aps` (or `cmd aps`) searches academic papers across Semantic Scholar (S2) and OpenAlex (OA), and manages a local paper library with PDF downloads and full-text search. Both search backends share a unified interface — learn one, swap the prefix.

**Always search both S2 and OA** for any query. They index different corpora and return different results. Run both in parallel and combine the findings.

## When to Use

- User asks to find academic papers on a topic
- User needs citation counts, references, or paper metadata
- User wants to look up a specific paper by DOI, arXiv ID, or title
- User needs author information (h-index, paper count, affiliations)
- User wants paper recommendations based on a seed paper
- User needs to search full-text passages (S2 snippets)
- User wants to aggregate/analyze publication data (OA group-by)
- User wants to download a paper PDF for local reading
- User wants to search across downloaded papers (full-text search)
- User wants to manage their local paper library

## Quick Reference

### Shared Commands (both `aps s2` and `aps oa`)

| Command | Alias | Description |
|---------|-------|-------------|
| `search <query>` | `s` | Keyword search for papers |
| `search --semantic <query>` | | Semantic/embedding-based search |
| `paper <id>` | `p` | Get paper details by ID |
| `citations <id>` | `c` | Papers that cite this paper |
| `references <id>` | `r` | Papers this paper cites |
| `author <query-or-id>` | `a` | Search or get author details |

### S2-only Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `recommend <id>` | `rec` | Paper recommendations (SPECTER embeddings) |
| `snippets <query>` | `snip` | Full-text passage search across S2ORC |
| `match <title>` | `m` | Find paper by exact title match |

### OA-only Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `institutions <query>` | `i` | Search institutions |
| `topics <query>` | `t` | Search topics |
| `group-by <field>` | `g` | Aggregate works by field |

### Shared Flags

| Flag | Description |
|------|-------------|
| `--year <YEAR>` | Year or range: `2020`, `2020-2024`, `2020-` |
| `--field <FIELD>` | Field of study (S2) or topic filter (OA) |
| `--min-citations <N>` | Minimum citation count |
| `--open-access` | Only open access papers |
| `-l, --limit <N>` | Max results (default 10) |
| `--offset <N>` | Pagination offset |
| `-F, --format plain\|json` | Output format (default plain) |

## Examples

### Search for Papers

```bash
# keyword search
aps s2 search "transformer attention" --limit 5
aps oa search "CRISPR gene editing" --limit 5

# with filters
aps s2 search "large language models" --year 2023- --field "Computer Science" --limit 10
aps oa search "climate change" --year 2020-2024 --open-access --sort cited_by_count:desc

# semantic search (embedding-based, finds conceptually related papers)
aps oa search --semantic "effects of sleep on memory consolidation" --limit 10
```

### Look Up a Specific Paper

```bash
# by arXiv ID
aps s2 paper ARXIV:1706.03762

# by DOI
aps s2 paper DOI:10.1038/s41586-020-2308-7
aps oa paper "10.1038/s41586-020-2308-7"

# by title (S2 fuzzy match)
aps s2 match "Attention Is All You Need"
```

### Citations & References

```bash
# what cites this paper?
aps s2 citations ARXIV:1706.03762 --limit 20
aps oa citations W2963403868 --limit 20

# what does this paper cite?
aps s2 references ARXIV:1706.03762
aps oa references W2963403868
```

### Authors

```bash
# search by name
aps s2 author "Geoffrey Hinton"
aps oa author "Geoffrey Hinton"

# get details by ID
aps s2 author 1695689        # S2 numeric author ID
aps oa author A5023888391    # OpenAlex author ID (starts with A)
```

### S2-only: Recommendations & Snippets

```bash
# papers similar to "Attention Is All You Need"
aps s2 recommend ARXIV:1706.03762 --limit 5 --pool recent

# full-text passage search
aps s2 snippets "backpropagation through time" --limit 5
```

### OA-only: Institutions, Topics, Group-by

```bash
# find institutions
aps oa institutions "MIT"

# find topics
aps oa topics "machine learning"

# aggregate data
aps oa group-by oa_status --filter "publication_year:2024"
aps oa group-by publication_year --filter "authorships.institutions.id:I63966007"
```

### Local Library (`aps library` / `aps lib`)

```bash
# download a paper by DOI (tries OA sources first, Sci-Hub fallback)
aps lib dl "10.1145/3442188.3445922"
aps lib dl "https://doi.org/10.1145/3442188.3445922"  # URL prefixes auto-stripped
aps lib dl --tag ml --tag nlp "10.1145/3442188.3445922"  # download with tags

# full-text search across all downloaded papers
aps lib search "language model" --limit 5
aps lib search "attention" --tag ml  # search within tagged papers only

# list all downloaded papers
aps lib ls
aps lib ls --tag ml  # filter by tag

# manage tags
aps lib tag add "10.1145/3442188.3445922" ml transformers  # add tags
aps lib tag rm "10.1145/3442188.3445922" ml                # remove a tag
aps lib tag ls                                              # list all tags with counts

# show paper details and text extraction stats (includes tags)
aps lib info "10.1145/3442188.3445922"

# open PDF in default viewer
aps lib open "10.1145/3442188.3445922"

# remove a paper from the library (cascade-deletes tags)
aps lib rm "10.1145/3442188.3445922"

# configure Sci-Hub base URL
aps lib config --set-url https://sci-hub.se
aps lib config  # show current config
```

| Command | Alias | Description |
|---------|-------|-------------|
| `download <doi>` | `dl` | Download PDF, resolve metadata, extract text, index |
| `search <query>` | `s` | Full-text search across all papers |
| `list` | `ls` | List all downloaded papers |
| `open <doi>` | `o` | Open PDF in default viewer |
| `info <doi>` | `i` | Show paper details + text stats + tags |
| `remove <doi>` | `rm` | Delete paper from DB + disk (cascade-deletes tags) |
| `read <doi>` | `r` | Output extracted paper text to stdout (for piping) |
| `tag add <doi> <tags...>` | | Add tag(s) to a paper |
| `tag rm <doi> <tags...>` | | Remove tag(s) from a paper |
| `tag ls` | | List all tags with paper counts |
| `reindex` | | Re-extract text for all papers using pdftotext |
| `config` | | Show/set Sci-Hub base URL |

Flags: `dl --tag <TAG>` (repeatable), `search --tag <TAG>`, `ls --tag <TAG>`, `dl --force`

Data stored at `~/.local/share/aps/` (SQLite DB + PDFs). Config at `~/.config/aps/`.

### Local Library: When to Use What

**`aps lib search <query>`** — search across your library
- Full-text search across title, authors, and extracted text of ALL downloaded papers
- Returns ranked results with highlighted snippets showing where the match occurred
- Use when: looking for a concept/term across multiple papers, finding which downloaded papers discuss a topic
- Papers must already be downloaded with `dl` to appear in search results

**`aps lib read <doi>`** — read one paper's full text
- Outputs the entire extracted text of a single paper to stdout
- Auto-downloads the paper if not already in library (no `dl` needed first)
- Use when: need to read/analyze one specific paper in detail, or pipe its text to another tool

**`dl` + `search` vs `read` — breadth vs depth**
- **Breadth** (`dl` then `search`): download several papers on a topic, then search across all of them to find which ones discuss a specific concept. Good for literature surveys and finding relevant passages across a corpus
- **Depth** (`read`): get the full text of one known paper for detailed analysis. Good when you already know which paper you want and need its complete content

```bash
# breadth: build a library, then search across it
aps lib dl "10.1145/3442188.3445922"
aps lib dl "10.48550/arXiv.2005.14165"
aps lib dl "10.48550/arXiv.2303.08774"
aps lib search "alignment" --limit 5

# depth: read one paper in full (auto-downloads if needed)
aps lib read "10.48550/arXiv.2303.08774"
```

### JSON Output

Any command supports `-F json` for structured output:

```bash
aps s2 search "attention" --limit 1 -F json
aps oa search "CRISPR" --limit 1 -F json | jq '.results[0].title'
```

## Always Use Both

Always run both `aps s2` and `aps oa` for any search. They have different corpora and ranking — combining results gives better coverage. For commands only available on one backend, use that backend.

| Only on S2 | Only on OA |
|------------|------------|
| `recommend` (SPECTER embeddings) | `institutions` |
| `snippets` (full-text passages) | `topics` |
| `match` (exact title lookup) | `group-by` (aggregation) |

## S2 Paper ID Formats

S2 accepts multiple identifier formats:
- S2 ID: `649def34f8be52c8b66281af98ae884c09aef38b`
- DOI: `DOI:10.1038/nrn3241`
- arXiv: `ARXIV:2106.09685`
- PubMed: `PMID:19872477`
- Corpus ID: `CorpusId:37220927`
- URL: `https://arxiv.org/abs/2106.09685`

## OA Filter Syntax

The `--filter` flag accepts raw OpenAlex filter strings:

```bash
# combine filters with commas (AND)
aps oa search "deep learning" --filter "is_oa:true,language:en,type:article"

# OR within a filter using pipe
aps oa search "neural" --filter "publication_year:2023|2024"
```

Key filters: `publication_year`, `is_oa`, `oa_status`, `type`, `language`, `has_fulltext`, `cited_by_count`

## Rate Limits

- **S2**: 1 RPS — the client automatically throttles across invocations via a tmp file
- **OA**: 10 RPS, no delay needed for typical usage. Semantic search limited to 1 RPS and max 50 results

## Auth

- `SEMANTIC_SCHOLAR_API_KEY` env var → higher rate limits for S2
- `OPENALEX_API_KEY` env var → higher rate limits for OA
- Both work without keys but may hit rate limits
