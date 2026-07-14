# APS CLI reference

Use this for stable command shapes and data contracts. Run `aps <command> --help` for the
installed binary's current flags and aliases.

## Contents

- Unified commands
- Provider commands
- Identifiers
- Scan manifests and seen-state
- JSON output
- Local library
- Authentication and storage

## Unified commands

```text
aps search <query> [filters] [-F plain|json]
aps paper <id> [-F plain|json]
aps citations <id> [filters] [-F plain|json]
aps references <id> [filters] [-F plain|json]
aps author <name-or-id> [-F plain|json]
aps scan --manifest <file> --from-date <YYYY-MM-DD> --to-date <YYYY-MM-DD> \
  --seen-file <file> [-F plain|json]
aps download <id> [--force] [--tag <tag>]...
aps login
aps status
```

Common discovery filters include exact/range year, since year, minimum citations, open access,
limit, and output format. `--year` and `--since` conflict. Unified search may also expose
`--skip-preprint`; verify it with `aps search --help` before use.

## Provider commands

S2 supports:

```text
aps s2 search <query> [--venue <venue>] [--pub-type <type>] [filters]
aps s2 paper <id>
aps s2 citations <id> [filters]
aps s2 references <id> [filters]
aps s2 author <name-or-id>
aps s2 recommend <id> [--pool <pool>] [filters]
aps s2 snippets <query> [--limit <n>]
aps s2 match <title>
```

Current source accepts `--semantic` on S2 search, but the keyword and semantic paths may be
implementation/version dependent; inspect local help and observed output before promising
embedding behavior.

OA supports:

```text
aps oa search <query> [--semantic] [--sort <field:direction>] [--work-type <type>] \
  [--institution <OA-ID>] [--source <OA-ID>] [--topic <OA-ID>] [--filter <raw>] [filters]
aps oa paper <id>
aps oa citations <id> [filters]
aps oa references <id> [filters]
aps oa author <name-or-id>
aps oa institutions <query>
aps oa topics <query>
aps oa group-by <field> [--filter <raw>]
```

Provider search filters can also include `--field`, pagination offset, open access, year, and
minimum citations. Do not transpose S2 venue/publication-type flags onto OA or OA structured
filters onto S2.

## Identifiers

- DOI: `10.1038/s41586-020-2308-7` or an accepted `DOI:`/URL form
- S2 paper ID: hexadecimal S2 ID; S2 also accepts formats such as `ARXIV:...`, `PMID:...`,
  and `CorpusId:...` where documented by its help
- OpenAlex work: `W2963403868`; author IDs begin with `A`
- arXiv: `ARXIV:1706.03762` or an accepted arXiv URL
- ChinaRxiv: `chinaxiv-202606.00025`, `202606.00025`, or a ChinaRxiv abstract URL

Use the provider's native identifier for provider-only commands. Prefer DOI when joining data.

## Scan manifests and seen-state

Manifest version 1 supports `oa_search`, `s2_search`, and `s2_recommend` jobs:

```json
{
  "version": 1,
  "jobs": [
    {
      "kind": "oa_search",
      "label": "recent-oa",
      "query": "machine unlearning",
      "institution": "I123456789",
      "sort": "publication_date:desc",
      "limit": 20
    },
    {
      "kind": "s2_search",
      "label": "recent-s2",
      "query": "machine unlearning",
      "limit": 20
    },
    {
      "kind": "s2_recommend",
      "label": "seed-neighbors",
      "paper_id": "ARXIV:2306.00000",
      "limit": 20
    }
  ]
}
```

Unknown manifest fields are rejected. `version` must be supported and `jobs` cannot be empty.
OA scan freshness uses the inclusive date window authoritatively; S2 jobs contribute unseen
candidates whose identity is reconciled by DOI, arXiv, provider IDs, and normalized title.

Seen-state version 1 is a managed object containing `entries`. Each entry records title,
optional DOI/arXiv/OA/S2 identifiers, and `last_seen_at`. A missing file starts empty. Reusing
the file suppresses already-seen identities across subsequent scans.

## JSON output

Use `-F json` and parse the emitted structure rather than scraping colored plain text:

```bash
aps search "diffusion policy robotics" -F json | jq '.'
aps oa search "climate adaptation" -F json | jq '.'
```

Do not assume every backend uses the same top-level envelope. Inspect one response before
writing a `jq` projection, and tolerate nullable identifiers and metadata.

## Local library

```text
aps lib download <doi> [--force] [--tag <tag>]...
aps lib search <query> [--mode hybrid|fts|semantic] [--tag <tag>]... [--tags a,b]
aps lib list [--tag <tag>]
aps lib info <doi>
aps lib read <doi>
aps lib open <doi>
aps lib remove <doi>
aps lib tag add <doi> <tags...>
aps lib tag remove <doi> <tags...>
aps lib tag list
aps lib reindex
aps lib optimize
aps lib config [--set-url <url>]
```

The library is DOI-keyed. Download extracts text and builds searchable chunks. Reindex repairs
metadata/text/indexes; optimize compacts the vector store. Confirm before removing papers or
changing library configuration.

## Authentication and storage

Recognized credentials are `SEMANTIC_SCHOLAR_API_KEY`, `OPENALEX_API_KEY`, and
`CHINARXIV_API_EMAIL`. `aps login` saves available values under `~/.config/aps/`; `aps status`
reports whether each value comes from the environment, config, or a default without revealing
the secret. Remote search can work without S2/OA keys but may be more constrained.

Persistent library data lives under `~/.local/share/aps/`; configuration lives under
`~/.config/aps/`. Treat both as user data.
