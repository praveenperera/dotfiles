return {
    {
        "RRethy/vim-illuminate",
        opts = {
            -- remove this override after vim-illuminate's treesitter provider works again
            -- with the current nvim-treesitter and nvim 0.12 combination
            providers = { "lsp", "regex" },
        },
    },
}
