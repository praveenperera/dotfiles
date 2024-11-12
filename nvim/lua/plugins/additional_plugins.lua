return {
    { "folke/neodev.nvim" },
    { "nvim-lua/plenary.nvim" },
    { "jose-elias-alvarez/typescript.nvim" },
    { "Vigemus/iron.nvim", cmd = "IronRepl" },
    { "kevinhwang91/nvim-bqf", event = "VeryLazy" },
    { "ThePrimeagen/harpoon", event = "User AstroFile" },
    { "wakatime/vim-wakatime", event = "BufRead" },
    { "kamykn/spelunker.vim", event = "BufRead" },
    {
        "lepture/vim-jinja",
        event = { "BufRead *.j2", "BufRead *.jinja", "BufRead *.html" },
    },
    { "tpope/vim-abolish", event = "BufRead" },
    { "mg979/vim-visual-multi", event = "BufRead" },
}
