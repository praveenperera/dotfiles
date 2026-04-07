return {
    { "nvim-lua/plenary.nvim" },
    { "Vigemus/iron.nvim", cmd = "IronRepl" },
    { "kevinhwang91/nvim-bqf", event = "VeryLazy" },
    { "ThePrimeagen/harpoon", event = "User AstroFile" },
    { "kamykn/spelunker.vim", event = "BufRead" },
    {
        "lepture/vim-jinja",
        event = { "BufRead *.j2", "BufRead *.jinja", "BufRead *.html" },
    },
    { "tpope/vim-abolish", event = "BufRead" },
    { "mg979/vim-visual-multi", event = "BufRead" },
}
