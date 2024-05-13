return {
    { "folke/neodev.nvim" },
    { "nvim-lua/plenary.nvim" },
    { "jose-elias-alvarez/typescript.nvim" },

    { "IndianBoy42/tree-sitter-just", event = { "BufRead Justfile" } },
    { "NoahTheDuke/vim-just", event = { "BufRead Justfile" } },
    { "Vigemus/iron.nvim", cmd = "IronRepl" },
    { "kevinhwang91/nvim-bqf", event = "VeryLazy" },
    { "christoomey/vim-tmux-navigator", event = "User AstroFile" },
    { "ThePrimeagen/harpoon", event = "User AstroFile" },
    { "wakatime/vim-wakatime", event = "BufRead" },
    { "kamykn/spelunker.vim", event = "BufRead" },
    {
        "lepture/vim-jinja",
        event = { "BufRead *.j2", "BufRead *.jinja", "BufRead *.html" },
    },
    {
        "towolf/vim-helm",
        event = { "BufRead *.yaml", "BufRead *.tpl" },
    },
    { "tpope/vim-abolish", event = "BufRead" },
    { "mg979/vim-visual-multi", event = "BufRead" },
}
