---
name: blast-radius
description: Find what a code or configuration change could break outside its direct diff, and prove the main safety assumption with executable evidence when practical. Use for compatibility risk, migration risk, suspicious small diffs, or requests about what to test before merging.
---

# Blast radius

Follow effects that symbol search does not show. This is a read-only review unless the user also asks for fixes.

## Find the safety claim

Read the diff, surrounding code, and stated intent. Identify the one or two facts that make the change safe. Examples include a wire field remaining compatible, a cleanup call being idempotent, or a state being unreachable.

## Look beyond direct callers

Check the affected boundaries that apply:

- serialized data, database schemas, caches, and migrations
- APIs, events, queues, protocols, and other language consumers
- lifecycle order, retries, cancellation, cleanup, and partial failure
- feature flags, configuration, deployment order, and old clients
- library behavior at the pinned version
- user flows that share state without sharing a call path

Rank risks by likelihood and consequence. Clear a risk only with evidence.

## Prove the main fact

Prefer the cheapest direct proof that uses the real code:

1. a focused script or existing test
2. a real integration path
3. the running application

If direct proof is too expensive or unavailable, cite the strongest source evidence and mark the fact unproven. A successful compile is not proof of runtime compatibility.

## Return

- **Change:** what now behaves differently
- **Safety fact:** the main assumption and how far it was proven
- **Risks:** confirmed or credible breakage paths with evidence
- **Cleared:** suspected risks that the evidence ruled out
- **Before merge:** the smallest check that catches the highest-cost failure

Do not produce a long list of hypothetical callers. Keep only risks with a concrete path to failure.
