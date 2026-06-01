---
name: refine-plan-html
description: Deep plan refinement that writes an HTML spec with all unanswered questions collected at the bottom for batch answers
---

# Refine Plan HTML

Create or refine an implementation spec as a readable HTML file. Gather requirements with the same depth as `refine-plan`, but do not run an interactive interview unless the user explicitly asks for one. Instead, inspect evidence, write the clearest spec possible, and collect every unresolved question in a bottom section so the user can answer everything at once.

## Evidence Before Questions

Before adding a question to the HTML spec, decide whether the answer should already exist in code, docs, tests, config, or plan artifacts.

- Inspect the current repo first with targeted file reads and searches
- If the plan depends on an external repo, upstream project, or library source, use `btx` to inspect that code before asking about its behavior
- Treat existing implementation details, public APIs, config defaults, test coverage, and architectural conventions as evidence to investigate, not hypotheticals for the user
- Ask only for intent, product decisions, scope boundaries, preference tradeoffs, or missing context that cannot be answered from source
- When source evidence answers part of a question, write the finding into the spec and ask only for the unresolved decision

## Output Contract

Produce an `.html` spec file unless the user names another target path.

- If refining an existing plan/spec, preserve its decisions and convert or update it into an HTML spec
- If no path is provided, create a clearly named HTML spec beside the relevant plan or in the current working directory
- Use Tailwind utility classes directly on the HTML elements so the spec is easy to read without a separate stylesheet
- Include the Tailwind browser CDN in `<head>` for normal viewing, and keep any extra inline CSS limited to small print or textarea fixes
- Prefer durable sections such as Overview, Goals, Non-Goals, Current Evidence, Requirements, Implementation Plan, Edge Cases, Verification, Risks, and Questions
- Include concrete file paths, APIs, commands, configs, and constraints discovered during inspection
- Mark uncertain details as questions instead of silently assuming product intent
- Do not include unresolved questions in the requirements as accepted decisions

## Styling

Make the generated HTML comfortable to read and answer.

- Use a restrained Tailwind layout: centered `max-w-5xl`, generous vertical spacing, readable line length, light borders, and clear section hierarchy
- Use inline Tailwind classes on every major element instead of relying on custom class names for core styling
- Use neutral colors with subtle accent colors for status, risks, and questions
- Style code paths, commands, and identifiers with compact monospace treatment
- Style question cards distinctly from confirmed requirements so unresolved decisions are visually obvious
- Make answer areas full width with enough height for detailed answers
- Avoid decorative gradients, oversized hero treatments, or dense card nesting
- Keep the HTML usable when printed by adding only minimal inline print CSS if needed

Recommended document shell:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="spec-status" content="refined">
    <script src="https://cdn.tailwindcss.com"></script>
    <title>Implementation Spec</title>
  </head>
  <body class="bg-zinc-50 text-zinc-900 antialiased">
    <main class="mx-auto max-w-5xl px-6 py-10">
      <!-- spec content -->
    </main>
  </body>
</html>
```

## Questions Section

Place all questions at the bottom of the HTML document in a dedicated section with `id="questions"`.

- Title the section `Questions For User`
- Group related questions under short subheadings when helpful
- Number each question so answers can refer to stable IDs
- Include a short context note for each question when source evidence explains why it matters
- Provide an answer area for each question, such as an empty `<textarea>` or clearly labeled blank answer block
- Make the questions comprehensive enough that the user can answer them all in one pass
- Do not ask `AskUserQuestion` questions for items already captured in the HTML, unless the user explicitly requests an interactive follow-up

Example question markup:

```html
<section id="questions" class="mt-12 border-t border-zinc-200 pt-8">
  <h2 class="text-2xl font-semibold tracking-tight text-zinc-950">Questions For User</h2>
  <div class="mt-6 space-y-5">
    <article class="rounded-lg border border-amber-200 bg-amber-50/70 p-5">
      <h3 class="text-base font-semibold text-zinc-950">Q1. Which retry behavior should failed uploads use?</h3>
      <p class="mt-2 text-sm leading-6 text-zinc-700">The current uploader retries network failures but not validation failures.</p>
      <label class="mt-4 block text-sm font-medium text-zinc-800">
        Answer
        <textarea name="q1" rows="5" class="mt-2 w-full rounded-md border border-zinc-300 bg-white p-3 text-sm leading-6 text-zinc-900 shadow-sm focus:border-zinc-500 focus:outline-none focus:ring-2 focus:ring-zinc-200"></textarea>
      </label>
    </article>
  </div>
</section>
```

## Topics to Cover

- Technical implementation details
- UI and UX considerations
- Concerns and edge cases
- Tradeoffs and alternatives
- Error states and recovery paths
- Security implications and attack surfaces
- Data lifecycle and cleanup
- Performance, scalability, and maintenance concerns
- Test strategy and verification commands

## Workflow

1. Inspect available source evidence for likely answers before drafting questions
2. Identify the target HTML spec path or choose a conservative path from nearby plan artifacts
3. Draft or update the HTML spec with confirmed evidence and decisions
4. Add every unresolved decision to the bottom `Questions For User` section
5. If a follow-up, idea, risk, or adjacent task is worth preserving but should not block the current refinement, add it to `next.md` beside the HTML spec instead of expanding the active scope
6. Report the spec path and summarize the highest-impact open questions

## Spec Status Marker

When refining a spec file, mark it as refined so later agents can tell the refinement pass has already touched it.

- In HTML specs, add or update `<meta name="spec-status" content="refined">` inside `<head>`
- If converting from Markdown with YAML front matter, preserve useful metadata in visible spec content or HTML metadata where appropriate
- Do not add the status marker to `next.md` or deferred follow-up files

## Deferred Follow-Ups

Create or update `next.md` when useful to capture deferred work that should survive the refinement session without becoming part of the active plan. Use it for follow-up questions, later investigation, adjacent feature ideas, cleanup tasks, risks to revisit, or decisions intentionally postponed.

Keep `next.md` concise and actionable:

- Include enough context for the item to make sense later
- Group related follow-ups under short headings when helpful
- Mark items as deferred, blocked, or candidate follow-ups rather than treating them as accepted requirements
- Do not move an item from `next.md` into the working HTML spec unless the user confirms it belongs in the current scope

## Important

- Keep the HTML spec current if the user provides answers and asks for refinement
- Do not ask the user to speculate about behavior that can be checked in the current repo or an inspectable external source
- Use `next.md` for deferred follow-ups so the current spec does not accumulate unresolved side tasks
- Keep questions at the bottom so the user can answer them in one pass
- Let the caller decide what to do with the gathered information
