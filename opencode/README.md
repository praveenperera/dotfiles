# OpenCode Configuration

This directory contains the OpenCode configuration with oh-my-opencode plugin.

## Bootstrap Setup

After running `bootstrap`, you need to:

1. Install dependencies:
   ```bash
   cd ~/.config/opencode
   bun install
   ```

2. Authenticate with providers:
   ```bash
   # Claude Pro/Max
   opencode auth login
   # Select Anthropic -> Claude Pro/Max
   
   # ChatGPT Plus/Pro
   opencode auth login
   # Select OpenAI -> ChatGPT Plus/Pro (Codex Subscription)
   ```

## Configuration

- `opencode.json` - Main OpenCode config with plugins and provider settings
- `oh-my-opencode.json` - oh-my-opencode plugin configuration
- `package.json` - Plugin dependencies (uses hotfix branch for openai-codex-auth)
- `bun.lock` - Dependency lockfile

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
