---
name: ask-grok
description: Ask Grok through the installed Grok CLI and return its response. Use when the user says "ask Grok," "consult Grok," "get Grok's opinion," or otherwise explicitly asks to send a task to Grok. Also use for X.com / Twitter lookups, posts, accounts, threads, and live discussion.
---

# Ask Grok

Run the command from the directory that contains the relevant context. Grok headless mode does not read piped stdin as the prompt. Pass the prompt with `-p` and a quoted here-document inside command substitution:

```sh
grok -m grok-4.6 --effort high --always-approve \
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
- `--disallowed-tools` keeps this consult read-only; do not drop it or add write tools
- Preserve the user's request and add only context that Grok needs to answer
- Do not steer Grok: no preferred answer, no framing that implies a conclusion, no leading questions, no "consider that X is better" style hints
- Pass facts, constraints, and paths; leave judgment and recommendation to Grok
- Use the quoted here-document delimiter so prompt text is not evaluated as shell syntax
- Return Grok's answer faithfully and identify it as Grok's response
- If the command fails, report the error and do not invent a response
- For repository implementation, use delegate-grok instead of this skill

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
