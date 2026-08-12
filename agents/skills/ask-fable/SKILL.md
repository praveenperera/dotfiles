---
name: ask-fable
description: Ask the Fable model a question through the installed Claude CLI and return its response. Use when the user says "ask Fable," "consult Fable," "get Fable's opinion," or otherwise explicitly asks Codex to send a task to Fable.
---

# Ask Fable

Run the command from the directory that contains the relevant context. Send the prompt through standard input in the same shell call:

```sh
claude -p --model fable --effort high \
  --permission-mode plan \
  --no-session-persistence \
  <<'FABLE_PROMPT'
<prompt>
FABLE_PROMPT
```

- Never start `claude -p` before the prompt is available; print mode exits when it starts without a prompt argument or standard input
- Preserve the user's request and add only context that Fable needs to answer
- Do not steer Fable: no preferred answer, no framing that implies a conclusion, no leading questions, no "consider that X is better" style hints
- Pass facts, constraints, and paths; leave judgment and recommendation to Fable
- Use the quoted here-document delimiter so prompt text is not evaluated as shell syntax
- For repository work, add each required context directory with `--add-dir <path>`
- Add only the required directories; never grant broad filesystem access
- Return Fable's answer faithfully and identify it as Fable's response
- If the command fails, report the error and do not invent a response
