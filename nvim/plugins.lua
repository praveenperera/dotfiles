local neotree         = require("user.config.neotree")
local neogit          = require("user.config.neogit")
local treesitter      = require("user.config.treesitter")
local copilot         = require("user.config.copilot")
local cmp             = require("user.config.cmp")
local theme           = require("user.config.theme")
local mason_lspconfig = require("user.config.mason_lspconfig")

-- Configure plugins
return {
    {
        "hrsh7th/nvim-cmp",
        dependencies = { "hrsh7th/cmp-emoji" },
        opts = cmp.config,
    },
    {
        "ray-x/go.nvim",
        dependencies = { -- optional packages
            "ray-x/guihua.lua",
            "neovim/nvim-lspconfig",
            "nvim-treesitter/nvim-treesitter",
        },
        opts = {},
        event = { "CmdlineEnter" },
        ft = { "go", 'gomod' },
        build = ':lua require("go.install").update_all_sync()', -- if you need to install/update all binaries,
    },
    {
        "olexsmir/gopher.nvim",
        ft = "go",
        config = function(_, opts)
            require("gopher").setup(opts)
        end,
        build = function()
            vim.cmd [[silent! GoInstallDeps]]
        end,
    },
    {
        "IndianBoy42/tree-sitter-just",
        event = { "BufRead Justfile" },
    },
    {
        "NoahTheDuke/vim-just",
        event = { "BufRead Justfile" },
    },
    { "Vigemus/iron.nvim",              cmd = "IronRepl", },
    { "kevinhwang91/nvim-bqf",          event = "VeryLazy",               opts = {} },
    { "christoomey/vim-tmux-navigator", event = "User AstroFile" },
    { "stevearc/oil.nvim",              opts = { delete_to_trash = true } },
    { "ThePrimeagen/harpoon",           event = "User AstroFile",         opts = {}, },
    {
        "folke/todo-comments.nvim",
        dependencies = "nvim-lua/plenary.nvim",
        opts = {},
        event = "BufRead",
        cmd = { "TodoQuickFix", "TodoLocList",
            "TodoTrouble",
            "TodoTelescope",
        },
    },
    { "wakatime/vim-wakatime", event = "BufRead" },
    { "kamykn/spelunker.vim",  event = "BufRead" },
    { "AstroNvim/astrotheme",  opts = theme.config, },
    { "nvim-lua/plenary.nvim" },
    {
        "TimUntersberger/neogit",
        dependencies = { "nvim-lua/plenary.nvim", "sindrets/diffview.nvim" },
        opts = neogit.config,
        cmd = "Neogit",
    },
    {
        "zbirenbaum/copilot.lua",
        cmd = "Copilot",
        event = "InsertEnter",
        opts = copilot.config,
    },
    {
        "ggandor/leap.nvim",
        event = "BufRead",
        config = function()
            require("leap").add_default_mappings()
        end,
    },
    {
        "nvim-neo-tree/neo-tree.nvim",
        version = "v2.x",
        dependencies = {
            "nvim-lua/plenary.nvim",
            "nvim-tree/nvim-web-devicons",
            "MunifTanjim/nui.nvim",
        },
        opts = neotree.config,
    },
    { "jose-elias-alvarez/typescript.nvim" },
    {
        'mrcjkb/rustaceanvim',
        version = '^4',
        ft = { 'rust' },
    },
    { "williamboman/mason-lspconfig.nvim", opts = mason_lspconfig.config },
    {
        "saecki/crates.nvim",
        version = "v0.3.0",
        dependencies = { "nvim-lua/plenary.nvim" },
        event = "BufRead Cargo.toml",
        opts = {
            null_ls = {
                enabled = true,
                name = "crates.nvim",
            },
        },
    },
    {
        "rust-sailfish/sailfish",
        event = { "BufRead *.stpl" },
        rtp = "syntax/vim"
    },
    { "lepture/vim-jinja",               event = { "BufRead *.j2", "BufRead *.jinja", "BufRead *.html" } },
    { "nvim-treesitter/nvim-treesitter", opts = treesitter.config },
    { "towolf/vim-helm",                 event = { "BufRead *.yaml", "BufRead *.tpl" } },
    { "folke/neodev.nvim" },
    { "tpope/vim-abolish",               event = "BufRead" },
    { "mg979/vim-visual-multi",          event = "BufRead" },
    {
        "tpope/vim-eunuch",
        cmd = {
            "Remove",
            "Delete",
            "Move",
            "Chmod",
            "Mkdir",
            "Cfind",
            "Clocate",
            "Lfind",
            "Llocate",
            "Wall",
            "SudoWrite",
            "SudoEdit",
        },
    },
}
