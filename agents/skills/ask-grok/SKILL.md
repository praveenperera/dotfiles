---
name: ask-grok
description: Use Grok through the installed Grok CLI only to search and read X.com / Twitter posts, accounts, threads, and replies. Use when the user asks to read an X post or asks Grok to look something up on X. Grok only fetches and summarizes; the caller does any analysis.
---

# Ask Grok

Grok only searches and reads X. Never ask it to analyze, compare, review, read a repository, or apply what it finds; that wastes tokens. Ask it to fetch the posts and return their content or a short summary with post URLs, then do any analysis yourself.

If you are running in Codex, use the `$hbd` skill.

Otherwise, run the command from a new empty temporary directory so Grok cannot read the repository. Grok headless mode does not read piped stdin as the prompt. Pass the prompt with `-p` and a quoted here-document inside command substitution:

```sh
grok -m grok-4.5 --effort medium --always-approve \
  --disallowed-tools "search_replace,run_terminal_cmd" \
  --verbatim \
  -p "$(cat <<'GROK_PROMPT'
<prompt>
GROK_PROMPT
)"
```

- Never start `grok -p` without the prompt argument; headless mode ignores stdin and exits after the turn
- Use `--prompt-file <path>` instead of `-p` when the prompt is already on disk
- `--always-approve` is required so X search and web search can run without a TTY prompt
- `--disallowed-tools` keeps this read-only; do not drop it or add write tools
- Keep the prompt to the search: what to find or fetch, and the constraints below. Do not add repository context, paths, or follow-up questions
- Use the quoted here-document delimiter so prompt text is not evaluated as shell syntax
- Return Grok's answer faithfully and identify it as Grok's response
- If the command fails, report the error and do not invent a response

## X.com searches

Prefer this skill over local web search for posts, accounts, threads, or discussion on X.com / Twitter. Grok has native X search. In the prompt, tell Grok to search X, not only the web, and include every known constraint:

- handles
- keywords or exact phrases
- date bounds
- post URLs or IDs
- latest vs top

Tell Grok to use its X tools and to cite post URLs. Do not invent posts.

- `x_keyword_search` for operators, dates, engagement, media, and latest vs top
- `x_semantic_search` for meaning-based discovery
- `x_user_search` to resolve a handle or profile
- `x_thread_fetch` to pull a post and its replies
