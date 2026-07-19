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
| Opus 4.8 | 7 | 8 | 5 | deliberate, polished, and collaborative; less capable than Fable on the hardest work |
| GPT-5.6 Luna | 5 | 4 | 10 | cheap, fast, and capable on repeated high-volume procedures; weak default for one-off development or ambiguous work |

The Fable and Opus scores preserve Theo Browne's published routing rubric. The Sol and Luna scores adapt the user's stated preferences, current provider positioning, and subsequent practitioner reports. Taste is especially subjective: use project-specific examples and evals when it matters.

## Fable strengths

Fable is strongest for:

- ambiguous, cross-cutting architecture
- high-level planning and task decomposition
- reconciling conflicting evidence or delegate outputs
- intent-sensitive product decisions
- public APIs, SDK shape, UI/UX, and copy where taste is part of correctness
- final simplification of a Sol implementation

Counter Fable's failure modes with explicit non-negotiable requirements, completion evidence, and what must not be omitted. Use an implementation or verification checklist when early stopping would be costly.

Anthropic positions Fable as its most capable generally available model for ambitious, long-running asynchronous work. Its July 2026 list price is $10 per million input tokens and $50 per million output tokens.

## Select GPT-5.6 Sol

Use Sol as the default Codex executor for:

- substantial bounded implementation
- difficult debugging with a clear outcome
- migrations and broad repository investigation
- work that benefits from persistence and many tool calls
- an independent code or plan review
- tasks where the Fable root has already chosen the architecture

Counter its failure modes in the prompt:

```text
Make the smallest coherent change that satisfies the objective. Preserve existing abstractions and patterns. Do not add speculative fallbacks, compatibility layers, broad rewrites, or tests that only restate implementation details. If the apparent fix expands materially beyond the owned scope, stop, explain why, and propose a smaller plan instead of piling on code.
```

Theo's GPT-5.6 review describes Sol as unusually determined and reliable while warning that it can turn a small change into a rewrite with excessive tests. The official Codex guide recommends Sol for complex, open-ended work and as the starting point when unsure. Its July 2026 API list price is $5 per million input tokens and $30 per million output tokens.

Use `high` reasoning by default for Sol work. Drop to `low` only for an easy, tightly scoped change with cheap verification. Do not use `medium` as the routine default.

## Select Opus 4.8

Use Opus for:

- high-taste review of code, APIs, UI, and copy
- interactive iteration where collaboration quality matters
- a deliberate second opinion on a Sol result
- a separate Claude pass when using another Fable-class worker would not justify the cost

Anthropic positions Opus 4.8 as a premium model for serious coding and agentic work with professional polish. Its July 2026 list price starts at $5 per million input tokens and $25 per million output tokens.

## Select GPT-5.6 Luna

Use Luna only when the prompt behaves like a function, success is cheap to check, and the work repeats at high volume or across cheap fan-out:

- classify or extract fields from many independent inputs
- inventory files, symbols, errors, or repeated patterns
- apply the same exact mechanical transform across many non-overlapping scopes
- generate branch names, titles, summaries, or other simple text
- run a cheap first-pass search that a smarter model will interpret

Do not use Luna for a one-off easy edit; if that work must be delegated, use Sol with `low` reasoning. Do not use Luna as the final authority for architecture, subtle debugging, security, taste, or broad implementation. Theo's GPT-5.6 review specifically frames Luna as a model that a smarter agent should orchestrate for bulk processing and simple outputs. OpenAI describes it as the cost-sensitive, high-volume tier. Its July 2026 API list price is $1 per million input tokens and $6 per million output tokens.

Use `low` reasoning for simple extraction and `medium` when tool use or several exact steps are required.

## Use complementary strengths

### Fable and Sol

Fable's intent inference and restraint offset Sol's tendency to overbuild, while Sol's drive offsets Fable's tendency to stop early. When evaluating Sol output, look for missed intent, abstraction drift, speculative scope, unnecessary code or tests, and overcomplicated control flow or APIs. Prefer the smallest coherent design without removing behavior or verification that protects real invariants.

### Sol and Opus

Sol's persistence and Opus's taste are useful when a difficult technical result also needs scrutiny for API shape, readability, and unnecessary code.

### Luna and a frontier model

Use Luna for cheap structured observations across repeatable work, then reserve consequential judgment for a stronger model.

## Sources

- [Theo Browne, “A proper guide to Fable 5”](https://www.youtube.com/watch?v=8GRmLR__OGQ) defines intelligence and taste and shows the original Fable, Opus, Sonnet, and GPT-5.5 ratings
- [Theo Browne, “GPT-5.6: The Review”](https://www.youtube.com/watch?v=IyoTJHLmClo) discusses Sol's persistence and overbuilding, Luna's orchestration role, and practical model selection
- [OpenAI Codex model selection](https://learn.chatgpt.com/docs/models#recommended-models) provides the current Sol, Terra, and Luna positioning
- [OpenAI GPT-5.6 Sol model page](https://developers.openai.com/api/docs/models/gpt-5.6-sol) provides capability, context, and pricing details
- [OpenAI GPT-5.6 Luna model page](https://developers.openai.com/api/docs/models/gpt-5.6-luna) provides cost-sensitive positioning and pricing details
- [Anthropic Claude Fable 5](https://www.anthropic.com/claude/fable) provides official use cases, availability, and pricing
- [Anthropic Claude Opus 4.8](https://www.anthropic.com/claude/opus) provides official use cases, availability, and pricing
- [Claire Vo's Sol and Fable comparison](https://www.lennysnewsletter.com/p/gpt-56-sol-vs-claude-fable-why-openais) is a useful counterpoint: a taste-weighted product benchmark favored Sol while finding Fable more precise and pedantic
