---
name: ask-gemini
description: Use the Gemini CLI to search the web for current information. Use this skill whenever the user explicitly asks to "ask gemini" or wants to query Gemini with web search capabilities.
---

# Ask Gemini

This skill uses the `gemini` CLI tool to search the web and get current information using Google's Gemini model with web search capabilities.

## When to Use

Use this skill when the user:
- Explicitly asks to "ask gemini" about something
- Wants to search the web using Gemini
- Needs current information that requires web search
- Mentions using Gemini for a query

## How to Use

### Basic Usage

The `gemini` CLI accepts positional arguments for one-shot queries. To ensure web search is used, prepend an explicit instruction:

```bash
# basic web search query
gemini "Use web search to find current information about: [user's question]"
```

### Example Workflow

1. User asks to "ask gemini about X"
2. Run gemini with explicit web search instruction:
   ```bash
   gemini "Use web search to find current information about: X"
   ```
3. Present the results to the user

### Query Format Examples

```bash
# weather query
gemini "Use web search to find current information about: weather in San Francisco"

# technical query
gemini "Use web search to find current information about: latest Rust async features"

# news query
gemini "Use web search to find current information about: recent AI developments"
```

## Output Format

The tool outputs text responses directly to stdout. The responses:
- Include information gathered from web search
- Are formatted as natural language
- May include citations or references to sources

## Notes

- Gemini CLI should be available in your PATH
- Authentication is handled by the CLI configuration
- The explicit "Use web search to find current information about:" prefix ensures Gemini performs a web search rather than relying solely on training data
- For complex queries, the user's exact wording is preserved after the prefix
