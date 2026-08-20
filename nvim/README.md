# Neovim configuration

This configuration uses Neovim 0.12, AstroNvim 6, and AstroCommunity. Rust,
Swift, and Go are the primary language environments. It also supports Lua,
TypeScript, HTML, CSS, JSON, YAML, Markdown, TOML, Just, and Jinja templates.

## Install

Back up the current configuration, and then link this directory:

```sh
mv ~/.config/nvim ~/.config/nvim.bak
ln -s "$PWD/nvim" ~/.config/nvim
nvim
```

Lazy.nvim and Mason install the configured plugins and development tools on the
first start.

## External tools

Install these commands outside Neovim:

- `git`, `just`, and `rg`
- `ra-multiplex` for the Rust language server connection
- `deno` to build Peek Markdown previews

Mason manages the other language servers, formatters, linters, and debuggers.
The configuration can also use SwiftFormat and SwiftLint from Homebrew.

## Update

Run `:Lazy update`, restart Neovim, and run it again when AstroNvim reports a
new pinned plugin snapshot. Run `:MasonToolsUpdate` to update managed tools.

Harpoon 2 uses a different state format from Harpoon 1. Add the Harpoon marks
again after the first update.

## Verify

From the dotfiles repository, run:

```sh
just nvim-check
```

For more details, run `:checkhealth` and `:ConformInfo` in Neovim.

The old LSP log can be large. Close Neovim and move it to a backup when it is
no longer useful:

```sh
mv ~/.local/state/nvim/lsp.log ~/.local/state/nvim/lsp.log.old
```
