local neotree = require("user.config.neotree")
local neogit = require("user.config.neogit")
local heirline = require("user.config.heirline")
local telescope = require("user.config.telescope")

local config = {
    updater = {
        remote = "origin",     -- remote to use
        channel = "stable",    -- "stable" or "nightly"
        version = "latest",    -- "latest", tag name, or regex search like "v1.*" to only do updates before v2 (STABLE ONLY)
        commit = nil,          -- commit hash (NIGHTLY ONLY)
        pin_plugins = nil,     -- nil, true, false (nil will pin plugins on stable only)
        skip_prompts = false,  -- skip prompts about breaking changes
        show_changelog = true, -- show the changelog after performing an update
        auto_reload = false,   -- automatically reload and sync packer after a successful update
        auto_quit = false,     -- automatically quit the current session after a successful update
    },
    colorscheme = "astrodark",
    highlights = {},
    -- set vim options here (vim.<first_key>.<second_key> = value)
    options = {
        opt = {
            -- set to true or false etc.
            relativenumber = true, -- sets vim.opt.relativenumber
            number = true,         -- sets vim.opt.number
            spell = true,          -- sets vim.opt.spell
            signcolumn = "auto",   -- sets vim.opt.signcolumn to auto
            wrap = false,          -- sets vim.opt.wrap
            tabstop = 4,
            softtabstop = 4,
            shiftwidth = 4,
            expandtab = true,
            smartindent = true,
            -- hardmode ( no mouse )
            mouse = nil,
        },
        g = {
            mapleader = " ",                   -- sets vim.g.mapleader
            autoformat_enabled = true,         -- enable or disable auto formatting at start (lsp.formatting.format_on_save must be enabled)
            smp_enabled = true,                -- enable completion at start
            autopairs_enabled = true,          -- enable autopairs at start
            diagnostics_enabled = true,        -- enable diagnostics at start
            status_diagnostics_enabled = true, -- enable diagnostics in statusline
            icons_enabled = true,              -- disable icons in the UI (disable if no nerd font is available, requires :PackerSync after changing)
            ui_notifications_enabled = true,   -- disable notifications when toggling UI elements
            heirline_bufferline = false,       -- enable new heirline based bufferline (requires :PackerSync after changing)
        },
    },
    -- Diagnostics configuration (for vim.diagnostics.config({...})) when diagnostics are on
    diagnostics = {
        virtual_text = true,
        underline = true,
    },
    -- Extend LSP configuration
    lsp = {
        servers = {},
        skip_setup = { "rust_analyzer" },
        formatting = {
            format_on_save = {
                enabled = true,
                allow_filetypes = {},
                ignore_filetypes = {},
            },
            disabled = {           -- disable formatting capabilities for the listed language servers },
                timeout_ms = 1000, -- default format timeout
            },
            mappings = {
                n = {},
            },
            ["server-settings"] = {},
        }
    },
    -- Mapping data with "desc" stored directly by vim.keymap.set().
    mappings = {
        n = {
            -- tab
            ["<leader>bb"] = { "<cmd>tabnew<cr>", desc = "New tab" },
            ["<leader>bc"] = {
                "<cmd>BufferLinePickClose<cr>",
                desc = "Pick to close",
            },
            ["<leader>bj"] = {
                "<cmd>BufferLinePick<cr>",
                desc = "Pick to jump",
            },
            ["<leader>bt"] = {
                "<cmd>BufferLineSortByTabs<cr>",
                desc = "Sort by tabs",
            },
            ["<leader>bw"] = { "<cmd>bw<CR>", desc = "Close current tab" },
            ["<leader>bW"] = {
                "<cmd>close<CR>",
                desc = "Close current split window",
            },
            -- window
            ["<leader>sv"] = { "<cmd>vsp<cr>", desc = "Split vertically" },
            ["<leader>sh"] = { "<cmd>hsp<cr>", desc = "Split horizontally" },
            -- quick save
            ["<C-s>"] = { "<cmd>w!<cr>", desc = "Save File" },
            -- window navigation
            ["<leader>1"] = { "1<C-w>w", desc = "Go to window 1" },
            ["<leader>2"] = { "2<C-w>w", desc = "Go to window 2" },
            ["<leader>3"] = { "3<C-w>w", desc = "Go to window 3" },
            ["<leader>4"] = { "4<C-w>w", desc = "Go to window 4" },
            ["<leader>5"] = { "5<C-w>w", desc = "Go to window 5" },
            ["<leader>6"] = { "6<C-w>w", desc = "Go to window 6" },
            -- git
            ["<leader>gs"] = { "<cmd>Neogit <CR>", desc = "Git status" },
            -- move
            ["<C-d>"] = { "<C-d>zz", desc = "Half page up" },
            ["<C-u>"] = { "<C-u>zz", desc = "Half page down" },
            -- random
            ["U"] = { "<cmd>redo<cr>" },
            ["J"] = { "mzJ`z" },
            ["n"] = { "nzzzv" },
            ["N"] = { "Nzzzv" },
            ---
            ["<leader>d"] = { [["_d]], desc = "Delete without register" },
            -- system yank
            ["<leader>Y"] = { [["+Y]], desc = "Yank to system register" },
            ["<leader>y"] = { [["+y]], desc = "Yank to system register" },
            -- find
            ["<leader>ff"] = { telescope.find_files, desc = "Find all files" },
            -- hardmode (no arrows)
            ["<Up>"] = { "<nop>" },
            ["<Down>"] = { "<nop>" },
            ["<Left>"] = { "<nop>" },
            ["<Right>"] = { "<nop>" },
        },
        x = {
            ["<leader>p"] = { [["_dP]], desc = "Paste without register" },
        },
        t = {},
        v = {
            ["<leader>d"] = { [["_d]], desc = "Delete without register" },
            -- move lines up and down like option arrows
            ["K"] = { ":m '<-2<CR>gv=gv", desc = "Move selection up" },
            ["J"] = { ":m '>+1<CR>gv=gv", desc = "Move selection down" },
        },
    },
    -- Configure plugins
    plugins = {
        { "AstroNvim/astrotheme" },
        { "rebelot/heirline.nvim", opts = heirline.config },
        { "nvim-lua/plenary.nvim", lazy = false },
        {
            "TimUntersberger/neogit",
            dependencies = { "nvim-lua/plenary.nvim" },
            config = function()
                require("neogit").setup(neogit.config())
            end
        },
        { "github/copilot.vim" },
        { "justinmk/vim-sneak" },
        { "mg979/vim-visual-multi" },
        { "tpope/vim-surround" },
        {
            "xbase-lab/xbase",
            run = "make install",
            dependencies = {
                "nvim-lua/plenary.nvim",
                "nvim-telescope/telescope.nvim",
                "neovim/nvim-lspconfig",
            },
            config = function()
                require("xbase").setup()
            end,
        },
        {
            "nvim-neo-tree/neo-tree.nvim",
            verision = "v2.x",
            dependencies = {
                "nvim-lua/plenary.nvim",
                "nvim-tree/nvim-web-devicons",
                "MunifTanjim/nui.nvim",
            },
            config = neotree.config
        },
        {
            "simrat39/rust-tools.nvim",
            event = "User AstroLspSetup",
            opts = function()
                return {
                    server = require("astronvim.utils.lsp").config("rust_analyzer")
                }
            end
        },
        {
            "williamboman/mason-lspconfig.nvim",
            opts = {
                ensure_installed = { "rust_analyzer" },
            },
        },
        {
            "saecki/crates.nvim",
            version = "v0.3.0",
            dependencies = { "nvim-lua/plenary.nvim" },
            config = function()
                require("crates").setup()
            end,
        },
        { "ThePrimeagen/vim-be-good" },
        { "towolf/vim-helm" },
    },
    -- LuaSnip Options
    luasnip = {
        filetype_extend = {},
        vscode = {
            paths = {},
        },
    },
    -- CMP Source Priorities
    cmp = {
        source_priority = {
            nvim_lsp = 1000,
            luasnip = 750,
            buffer = 500,
            path = 250,
        },
    },
    -- Run after everything is loaded
    polish = function()
    end,
}

return config
