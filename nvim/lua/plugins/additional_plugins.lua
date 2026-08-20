return {
    { "Vigemus/iron.nvim", cmd = "IronRepl" },
    { "kevinhwang91/nvim-bqf", event = "VeryLazy" },
    { "kamykn/spelunker.vim", event = "BufRead" },
    {
        "tpope/vim-abolish",
        event = "BufRead",
        init = function()
            vim.g.abolish_no_mappings = true
        end,
    },
    { "mg979/vim-visual-multi", event = "BufRead" },
}
