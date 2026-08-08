# Model routing reference

## Treat ratings as a local policy

The routing table is a qualitative working rubric, not a provider benchmark. Recalibrate it from observed outputs when a model, harness, subscription, or workload changes.

The three axes intentionally measure different things:

- **intelligence** measures the difficulty and ambiguity a model can handle without supervision
- **taste** measures restraint and quality in UI/UX, code, APIs, and copy
- **cost efficiency** measures practical affordability for this workflow, including subscription limits and total task consumption rather than API list price alone

Higher is better on every axis.

| Model | Intelligence | Taste | Cost efficiency | Character |
| --- | ---: | ---: | ---: | --- |
| Fable 5 | 9 | 9 | 2 | strongest intent inference and taste; expensive; can be lazy or pursue perceived intent over literal instructions |
| GPT-5.6 Sol | 8 | 7 | 8 | relentless and efficient; drives hard to completion; can overbuild instead of stepping back |
| Opus 5 | 8 | 8 | 6 | near-Fable on benchmarks at Opus price, but launch reports show early stopping and weaker adherence to long instruction sets; taste strong but unproven relative to Fable |
| GPT-5.6 Luna | 5 (7 at `max`) | 4 | 10 | very cheap after the July 30, 2026 80% price cut; `max` reasoning reaches near Sol-`medium` capability, so it owns easy bounded delegations; still weak for ambiguous or taste-sensitive work |

The Fable score preserves Theo Browne's published routing rubric; the Opus 5 scores are a local recalibration from Anthropic's launch benchmarks and positioning together with early practitioner reports, since Theo's video predates Opus 5. The Sol and Luna scores adapt the user's stated preferences, current provider positioning, and subsequent practitioner reports. Taste is especially subjective: use project-specific examples and evals when it matters.

## Root modes

This workflow supports two Claude roots. Capability scores stay the same; default ownership moves with the root.

### Fable 5 root

- Fable owns orchestration, ambiguous architecture, intent-sensitive design, high-taste surface work, and final simplification in the root thread.
- Sol is the default delegated implementer.
- Opus 5 is the default high-taste second opinion and the "use opus" implementation substitute.
- Prefer not to spawn Fable as a subagent of itself for work the root can finish.

### Opus 5 root

- Opus owns orchestration, long-horizon agentic implementation, and complex debugging in the root thread.
- Sol remains the default delegated implementer.
- **Fable is the default model for high-taste review, UI/UX/API/copy judgment, intent-sensitive surface design, and final simplification.** Invoke Fable through the Agent tool with model `fable` and `high` effort.
- Do not treat the Opus root as the final taste authority on its own implementation. A cheap self-check is fine; consequential taste and simplification go to Fable.
- When "use opus" is active, Opus implements (root or isolated Agent scope). Take independent review from Fable for taste and from Sol when cross-vendor inventory scrutiny matters.

Detect the root from the session model or an explicit user directive. Do not silently switch roots mid-session unless the user asks.

## Fable strengths

Fable is strongest for:

- ambiguous, cross-cutting architecture
- high-level planning and task decomposition
- reconciling conflicting evidence or delegate outputs
- intent-sensitive product decisions
- public APIs, SDK shape, UI/UX, and copy where taste is part of correctness
- final simplification of a Sol or Opus implementation
- high-taste review when Opus is the root (or when an independent Fable pass is worth the cost)

When Fable is root, do that work in-thread. When Opus is root, route the taste, review, and simplification rows above to a Fable subagent rather than leaving them on Opus.

Counter Fable's failure modes with explicit non-negotiable requirements, completion evidence, and what must not be omitted. Use an implementation or verification checklist when early stopping would be costly. For Fable subagent prompts, keep the same short contract shape used for Opus: hard authority, scope, and verification; soft style; progressive references rather than skill dumps.

Anthropic positions Fable as its most capable generally available model for ambitious, long-running asynchronous work. Its July 2026 list price is $10 per million input tokens and $50 per million output tokens.

## Select GPT-5.6 Sol

Use Sol as the default Codex executor for:

- substantial bounded implementation
- difficult debugging with a clear outcome
- migrations and broad repository investigation
- work that benefits from persistence and many tool calls
- an independent code or plan review
- tasks where the Claude root (Fable or Opus) has already chosen the architecture
- under an Opus root, adversarial inventory-style review that benefits from cross-vendor scrutiny

Counter its failure modes in the prompt:

```text
Make the smallest coherent change that satisfies the objective. Preserve existing abstractions and patterns. Do not add speculative fallbacks, compatibility layers, broad rewrites, or tests that only restate implementation details. If the apparent fix expands materially beyond the owned scope, stop, explain why, and propose a smaller plan instead of piling on code.
```

Theo's GPT-5.6 review describes Sol as unusually determined and reliable while warning that it can turn a small change into a rewrite with excessive tests. The official Codex guide recommends Sol for complex, open-ended work and as the starting point when unsure. Its July 2026 API list price is $5 per million input tokens and $30 per million output tokens.

Use `high` reasoning for all Sol work. Do not run Sol at `low` or `medium`: since the July 2026 Luna price cut, an easy, tightly scoped change with cheap verification routes to Luna with `max` reasoning instead, which reaches near the same capability at a small fraction of the cost.

## Select Opus 5

Use Opus 5 for:

- root orchestration when the session root is Opus
- long-horizon agentic implementation, including multi-step terminal work, broad refactors, and workflow automation
- complex debugging and root-cause analysis
- under a Fable root: a deliberate second opinion on a Sol or Fable result, and high-taste review when spending another Fable pass is not justified
- under a Fable root: full delegated implementation when the user directs "use opus"
- interactive iteration where collaboration quality matters

Do **not** use Opus as the primary taste/review/simplification authority when Opus is already the root and produced or directed the implementation. Prefer Fable for that pass.

Anthropic positions Opus 5 as near Fable 5 intelligence at Opus speed and cost, which works out to roughly half Fable's cost per task. Its model id is `claude-opus-5`, and its July 2026 list price is $5 per million input tokens and $25 per million output tokens, unchanged from Opus 4.8. Fast mode runs 2.5x faster at 2x the base price when latency matters more than cost.

Counter its failure modes in the prompt. Launch-era reports describe a model that stops before the work is finished, reports unfinished work as done, argues with explicit instructions, and follows large instruction-dense skill files less reliably than short focused ones:

```text
Finish the entire objective before reporting. Do not stop at a partial result, and do not report success while any part of the success condition is unmet. Follow the explicit requirements above even where you would choose a different approach; if you believe a requirement is wrong, say so in the final report and still satisfy it. Stop early only for one of the listed stop conditions, and name which one.
```

Send Opus 5 a focused self-contained prompt rather than pointing it at a large bundle of instruction files, and use `high` effort by default, as with Sol. For how to write that prompt—judgment over hard style rules, interfaces over examples, progressive disclosure, rich references, and a short completion rider—read [opus5-prompting.md](opus5-prompting.md).

Sol remains the default delegated implementer under both roots. When the user directs "use opus", usually because Sol usage limits are running low, Opus 5 owns delegated implementation for that session. The launch benchmarks support the substitution: Opus 5 matches or beats Sol on FrontierCode (53.4 vs 47.5), terminal coding (43.3 vs 34.4), and AutomationBench (26.0 vs 18.1), and trails only on DeepSWE (68.8 vs 72.7). Under this substitution:

- from a Fable root, take independent review from Sol (cross-vendor) and keep Fable on final taste/simplification when needed
- from an Opus root, take taste/review/simplification from Fable and inventory-style cross-checks from Sol
- verify every completion claim against the success condition and the diff before integrating

The near-Fable framing above comes from Anthropic; the first outside reports temper it, and precedent from the GPT-5.1 and GPT-5.2 launches is that early impressions of an awkward model can improve as prompting adapts. Recalibrate this section from observed runs after a few weeks of use.

## Select GPT-5.6 Luna

Use Luna when success is cheap to check and either the prompt behaves like a function repeated at high volume, or the task is an easy one-off delegation that would once have gone to Sol at `low`:

- make an easy, tightly scoped one-off change with cheap verification (`max` reasoning)
- classify or extract fields from many independent inputs
- inventory files, symbols, errors, or repeated patterns
- apply the same exact mechanical transform across many non-overlapping scopes
- generate branch names, titles, summaries, or other simple text
- run a cheap first-pass search that a smarter model will interpret

Do not use Luna as the final authority for architecture, subtle debugging, security, taste, or broad implementation. Theo's GPT-5.6 review specifically frames Luna as a model that a smarter agent should orchestrate for bulk processing and simple outputs; the July 30, 2026 price cut extends that to easy bounded implementation at `max` reasoning. OpenAI cut Luna's API list price by 80% on that date, to $0.20 per million input tokens and $1.20 per million output tokens, while Sol stayed at $5 and $30.

Choose reasoning effort by workload shape, not difficulty alone. Use `low` for simple extraction and `medium` when tool use or several exact steps are required; at bulk volume, `max` wastes tokens and latency. Use `max` for a one-off easy delegated change: Artificial Analysis measures Luna at `max` near Sol-`medium` capability, but its output tokens rise roughly 9x over no-reasoning, so reserve `max` for one-off or low-count work.

## Use complementary strengths

### Fable and Sol

Fable's intent inference and restraint offset Sol's tendency to overbuild, while Sol's drive offsets Fable's tendency to stop early. When evaluating Sol output, look for missed intent, abstraction drift, speculative scope, unnecessary code or tests, and overcomplicated control flow or APIs. Prefer the smallest coherent design without removing behavior or verification that protects real invariants. This pairing is the default under a Fable root and still applies under an Opus root when Fable is the taste/simplification subagent after a Sol implementation.

### Opus and Fable

Under an Opus root, Opus supplies orchestration and persistence while Fable supplies taste, restraint, and intent-sensitive surface judgment. Typical shapes:

- Opus plans and implements (or Sol implements); Fable reviews API/UI/copy taste and simplifies
- Opus debugs a hard failure; Fable judges whether the resulting surface still fits the product
- Sol implements under Opus orchestration; Fable does the final simplification pass

Under a Fable root, invert only the second-opinion direction: Fable shapes the design; Opus can review when an independent Claude pass is useful and another Fable spend is not.

Do not run Fable and Opus as overlapping writers on the same files. One implements or simplifies; the other reviews read-only unless ownership is explicitly transferred.

### Sol and Opus

Either model can implement while the other reviews for correctness and missed cases. With Sol implementing under a Fable root, Opus 5 can supply a deliberate second opinion on API shape, readability, and unnecessary code. With Opus 5 implementing (root or "use opus"), Sol supplies the cross-vendor review pass, where its persistence is well suited to re-deriving inventories and chasing missed cases. Under an Opus root, prefer Fable over Opus for the high-taste leg of that review, and Sol for the inventory leg. Check an Opus 5 review for coverage as well as verdicts: the same tendency to stop early can truncate a review before it reaches the whole diff.

### Luna and a frontier model

Use Luna for cheap structured observations across repeatable work, then reserve consequential judgment for a stronger model. Since the price cut, Luna at `max` also serves as the implementer for easy bounded changes under frontier-model orchestration, with the orchestrator verifying the diff.

## Sources

- [Theo Browne, “A proper guide to Fable 5”](https://www.youtube.com/watch?v=8GRmLR__OGQ) defines intelligence and taste and shows the original Fable, Opus, Sonnet, and GPT-5.5 ratings; it predates Opus 5
- [Theo Browne, “GPT-5.6: The Review”](https://www.youtube.com/watch?v=IyoTJHLmClo) discusses Sol's persistence and overbuilding, Luna's orchestration role, and practical model selection
- [OpenAI Codex model selection](https://learn.chatgpt.com/docs/models#recommended-models) provides the current Sol, Terra, and Luna positioning
- [OpenAI GPT-5.6 Sol model page](https://developers.openai.com/api/docs/models/gpt-5.6-sol) provides capability, context, and pricing details
- [OpenAI GPT-5.6 Luna model page](https://developers.openai.com/api/docs/models/gpt-5.6-luna) provides cost-sensitive positioning and pricing details
- [CNBC on the July 30, 2026 GPT-5.6 price cuts](https://www.cnbc.com/2026/07/30/open-ai-price-cut-gpt.html) reports the 80% Luna cut to $0.20/$1.20 per million tokens, the 20% Terra cut, and unchanged Sol pricing
- [Augmented Mind on Luna's effort-dependent capability](https://augmentedmind.substack.com/p/gpt-56-luna-is-80-cheaper) grounds the `max`-reasoning numbers: Artificial Analysis index 26.6 without reasoning to 51.2 at `max`, with roughly 9x output tokens and 13.5x evaluation cost
- [Anthropic Claude Fable 5](https://www.anthropic.com/claude/fable) provides official use cases, availability, and pricing
- [Claude Opus 5 announcement](https://www.anthropic.com/news/claude-opus-5) provides official use cases, availability, pricing, and the launch benchmark card that grounds the split criteria above
- [Dan Shipper's Opus 5 day-zero vibe check](https://x.com/danshipper/status/2080700057892815114) is the source for the early stopping, instruction arguing, and large-skill-file observations; it reports one week of Every's testing on launch day and is explicitly provisional
- [Thariq Shihipar on context engineering for Claude 5 models](https://x.com/trq212/status/2080710971228918066) and the [Claude blog post](https://claude.com/blog/the-new-rules-of-context-engineering-for-claude-5-generation-models) are the source for Opus 5 prompting defaults in [opus5-prompting.md](opus5-prompting.md): unhobble style rules, design interfaces instead of examples, progressive disclosure, and rich references
- [Claire Vo's Sol and Fable comparison](https://www.lennysnewsletter.com/p/gpt-56-sol-vs-claude-fable-why-openais) is a useful counterpoint: a taste-weighted product benchmark favored Sol while finding Fable more precise and pedantic
