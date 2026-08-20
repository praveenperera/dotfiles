-- AstroCommunity: import any community modules here
-- We import this file in `lazy_setup.lua` before the `plugins/` folder.
-- This guarantees that the specs are processed before any user plugins.

---@type LazySpec
return {
    "AstroNvim/astrocommunity",
    { import = "astrocommunity.editing-support.conform-nvim" },
    { import = "astrocommunity.lsp.nvim-lint" },
    { import = "astrocommunity.search.grug-far-nvim" },
    { import = "astrocommunity.pack.lua" },
    { import = "astrocommunity.pack.rust" },
    { import = "astrocommunity.pack.swift" },
    { import = "astrocommunity.pack.go" },
    { import = "astrocommunity.pack.typescript" },
    { import = "astrocommunity.pack.toml" },
    { import = "astrocommunity.pack.just" },
    { import = "astrocommunity.pack.yaml" },
    { import = "astrocommunity.pack.json" },
    { import = "astrocommunity.pack.markdown" },
    { import = "astrocommunity.pack.html-css" },
    { import = "astrocommunity.pack.prettier" },
}
