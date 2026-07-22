---
name: deslop-writing
description: Draft, rewrite, and audit English prose to remove formulaic AI-writing patterns while preserving meaning, facts, authorial voice, dialect, and channel conventions. Use when Codex is asked to deslop or humanize writing, make prose sound less AI-generated, avoid AIisms, preserve a writer's voice during editing, diagnose formulaic prose, or produce direct and specific copy without common LLM tics.
---

# Deslop Writing

Produce specific, direct prose without replacing the writer's voice with another generic style.

## Load the catalog

Read [references/aiisms.md](references/aiisms.md) before drafting, rewriting, or auditing prose. Apply its tiered rules and final checks. Treat the catalog as editorial guidance, never as proof of authorship.

## Choose the mode

Infer the mode from the request. Ask only when the requested output would materially differ.

- **Draft**: create prose from facts, notes, or an outline
- **Rewrite**: revise supplied prose while preserving its claims and voice
- **Audit**: identify formulaic patterns without rewriting the whole piece

## Establish the brief

Identify the goal, audience, channel, intended action, length, required facts, and any supplied voice samples. Use the requested structure when it serves the content. Do not impose essay, list, or marketing-copy conventions on another format.

When source material is incomplete, do not fill gaps with generic claims. Ask for information only when it is necessary to produce a truthful result; otherwise make the narrowest reasonable edit.

## Preserve invariants

Before rewriting, record the details that must survive:

- names, numbers, dates, chronology, quotations, technical terms, and instructions
- claims, causal relationships, uncertainty, modality, and negation
- actors, motives, permissions, obligations, and intended consequences
- stance, humor, dialect, contractions, cultural markers, and intentional repetition
- required headings, calls to action, citations, and channel constraints

Do not normalize regional English toward American English. Do not add slang, typos, fragments, anecdotes, emotion, or personal experience merely to simulate humanity. Do not replace empty rhetoric with a new fact, instruction, motive, or consequence; delete it when the source provides no concrete meaning to preserve. Never promise human authorship or detector evasion.

## Edit from meaning

1. Identify the load-bearing claims and concrete evidence.
2. Draft or reconstruct the passage from those claims instead of swapping synonyms sentence by sentence.
3. Remove core formulas from the catalog unless quoting or discussing them.
4. Change contextual signals only when they are repeated, imprecise, ornamental, or wrong for the voice and format.
5. Prefer exact nouns, simple verbs, named sources, and concrete consequences.
6. Let syntax, sentence length, and formatting follow the thought. Do not manufacture randomness.
7. Compare the result with the invariants and run the catalog's final checks.

## Return the requested artifact

### Draft

Return finished copy in the requested format. Separate supplied facts from inference. Add only explanations that follow necessarily from the supplied material; do not invent operators, recovery behavior, motives, or intended uses. Omit a preamble, editing notes, source labels, and self-evaluation unless the user asks for them.

### Rewrite

Return only the revised prose by default. Preserve facts and intent rather than silently correcting or expanding them. Ask a focused question before making a change that would resolve a meaningful ambiguity or alter a claim.

### Audit

Report only actionable findings. Use a compact table with `Excerpt`, `Pattern`, `Why it weakens the prose`, and `Suggested revision`. Distinguish core formulas from contextual signals and never infer AI authorship from style alone. When conspicuous contextual features are legitimate, include a `Keep` row for each so the audit does not imply they should be removed.

## Handle conflicts

Follow explicit user, publication, brand, legal, accessibility, and genre requirements over this skill's defaults. If a required phrase matches the catalog, keep it. If preserving voice conflicts with clarity, correct only what blocks the intended reader and retain the writer's recognizable choices.
