---
name: aps
description: Academic paper search CLI (Semantic Scholar & OpenAlex). Use when the user needs to find papers, look up citations, get paper details, search authors, or do any academic research task.
---

# aps — Academic Paper Search

`aps` (or `cmd search`) searches academic papers across Semantic Scholar (S2) and OpenAlex (OA). Both backends share a unified interface — learn one, swap the prefix.

## When to Use

- User asks to find academic papers on a topic
- User needs citation counts, references, or paper metadata
- User wants to look up a specific paper by DOI, arXiv ID, or title
- User needs author information (h-index, paper count, affiliations)
- User wants paper recommendations based on a seed paper
- User needs to search full-text passages (S2 snippets)
- User wants to aggregate/analyze publication data (OA group-by)

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

### JSON Output

Any command supports `-F json` for structured output:

```bash
aps s2 search "attention" --limit 1 -F json
aps oa search "CRISPR" --limit 1 -F json | jq '.results[0].title'
```

## Choosing Between S2 and OA

| Need | Use |
|------|-----|
| Paper recommendations | `aps s2 recommend` |
| Full-text snippets | `aps s2 snippets` |
| Exact title lookup | `aps s2 match` |
| Semantic search | `aps oa search --semantic` (better) or `aps s2 search` |
| Institution data | `aps oa institutions` |
| Topic taxonomy | `aps oa topics` |
| Aggregation/analytics | `aps oa group-by` |
| Citation intents (why cited) | `aps s2 citations` |
| Open access filtering | Both, but OA has richer `--filter` |
| General paper search | Both work well |

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

## Auth

- `SEMANTIC_SCHOLAR_API_KEY` env var → higher rate limits for S2
- `OPENALEX_API_KEY` env var → higher rate limits for OA
- Both work without keys but may hit rate limits
