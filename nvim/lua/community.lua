-- AstroCommunity: import any community modules here
-- We import this file in `lazy_setup.lua` before the `plugins/` folder.
-- This guarantees that the specs are processed before any user plugins.

---@type LazySpec
return {
    "AstroNvim/astrocommunity",
    { import = "astrocommunity.pack.lua" },
    { import = "astrocommunity.pack.typescript" },
    { import = "astrocommunity.pack.toml" },
    { import = "astrocommunity.pack.helm" },
    { import = "astrocommunity.pack.just" },
    { import = "astrocommunity.pack.svelte" },
    { import = "astrocommunity.pack.yaml" },
    { import = "astrocommunity.pack.zig" },
    { import = "astrocommunity.pack.elixir-phoenix" },
    { import = "astrocommunity.pack.tailwindcss" },
    { import = "astrocommunity.pack.json" },
    { import = "astrocommunity.pack.markdown" },
    { import = "astrocommunity.pack.html-css" },
}
