# OpenCode Configuration

This directory contains the OpenCode configuration.

## Bootstrap Setup

After running `bootstrap`, authenticate with providers:

```bash
# Claude Pro/Max
opencode auth login
# Select Anthropic -> Claude Pro/Max

# ChatGPT Plus/Pro
opencode auth login
# Select OpenAI -> ChatGPT Plus/Pro (Codex Subscription)
```

## Configuration

- `opencode.json` - Main OpenCode config with provider settings

The configuration needs no package manager. Local plugins are `.ts` files under
`plugins/`, and package plugins install into the OpenCode cache through
`opencode plugin add`.

## MCP Servers

| Server | Type | URL |
|--------|------|-----|
| `exa` | remote | `https://mcp.exa.ai/mcp` |
| `codemode` | remote | `http://127.0.0.1:7681/mcp` |

`codemode` is the local codemode-mcp gateway, the same server Claude Code and
Codex use.

## Agents Configured

| Agent | Model | Source |
|-------|-------|--------|
| **OmO** | Claude Opus 4.5 | Claude Pro/Max (default) |
| **oracle** | GPT 5.2 | ChatGPT Plus/Pro |
| **librarian** | Claude Sonnet 4.5 | Claude Pro/Max (default) |
| **explore** | Grok Code | OpenCode Zen (default) |
| **frontend-ui-ux-engineer** | Gemini 3 Pro | OpenCode Zen |
| **document-writer** | Gemini 3 Pro | OpenCode Zen |
| **multimodal-looker** | Gemini 2.5 Flash | OpenCode Zen |

## Model Sources

- **Claude models** (Opus 4.5, Sonnet 4.5): Uses Claude Pro/Max subscription
- **GPT models** (GPT 5.2): Uses ChatGPT Plus/Pro subscription  
- **Gemini & Other models**: Uses OpenCode Zen API (`opencode/` prefix)
