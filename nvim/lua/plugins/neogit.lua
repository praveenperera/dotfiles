local opts = {
    disable_commit_confirmation = true,
    disable_context_highlighting = false,
    auto_refresh = true,
    integrations = {
        diffview = true,
    },
}

return {
    "TimUntersberger/neogit",
    dependencies = { "nvim-lua/plenary.nvim", "sindrets/diffview.nvim" },
    opts = opts,
    cmd = "Neogit",
}
