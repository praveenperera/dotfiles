require("user.config.heirline")
require("user.config.neotree")
require("user.config.neogit")

local function configurePolish()
    require("neogit").setup(NeogitConfig())
end

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
    -- Set colorscheme to use
    colorscheme = "default_theme",
    -- Add highlight groups in any theme
    highlights = {
        -- init = { -- this table overrides highlights in all themes
        --   Normal = { bg = "#000000" },
        -- }
        -- duskfox = { -- a table of overrides/changes to the duskfox theme
        --   Normal = { bg = "#000000" },
        -- },
    },
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
    header = {
        " █████  ███████ ████████ ██████   ██████",
        "██   ██ ██         ██    ██   ██ ██    ██",
        "███████ ███████    ██    ██████  ██    ██",
        "██   ██      ██    ██    ██   ██ ██    ██",
        "██   ██ ███████    ██    ██   ██  ██████",
        " ",
        "    ███    ██ ██    ██ ██ ███    ███",
        "    ████   ██ ██    ██ ██ ████  ████",
        "    ██ ██  ██ ██    ██ ██ ██ ████ ██",
        "    ██  ██ ██  ██  ██  ██ ██  ██  ██",
        "    ██   ████   ████   ██ ██      ██",
    },
    -- Default theme configuration
    default_theme = {
        -- Modify the color palette for the default theme
        colors = {
            fg = "#abb2bf",
            bg = "#1e222a",
        },
        highlights = function(hl) -- or a function that returns a new table of colors to set
            local C = require "default_theme.colors"

            hl.Normal = { fg = C.fg, bg = C.bg }

            -- New approach instead of diagnostic_style
            hl.DiagnosticError.italic = true
            hl.DiagnosticHint.italic = true
            hl.DiagnosticInfo.italic = true
            hl.DiagnosticWarn.italic = true

            return hl
        end,
        -- enable or disable highlighting for extra plugins
        plugins = {
            aerial = true,
            beacon = false,
            bufferline = true,
            cmp = true,
            dashboard = true,
            highlighturl = true,
            hop = false,
            indent_blankline = true,
            lightspeed = false,
            ["neo-tree"] = true,
            notify = true,
            ["nvim-tree"] = false,
            ["nvim-web-devicons"] = true,
            rainbow = true,
            symbols_outline = false,
            telescope = true,
            treesitter = true,
            vimwiki = false,
            ["which-key"] = true,
        },
    },
    -- Diagnostics configuration (for vim.diagnostics.config({...})) when diagnostics are on
    diagnostics = {
        virtual_text = false,
        underline = true,
    },
    -- Extend LSP configuration
    lsp = {
        -- enable servers that you already have installed without mason
        servers = {},
        formatting = {
            -- control auto formatting on save
            format_on_save = {
                enabled = true,     -- enable or disable format on save globally
                allow_filetypes = { -- enable format on save for specified filetypes only
                    -- "go",
                },
                ignore_filetypes = { -- disable format on save for specified filetypes
                    -- "python",
                },
            },
            disabled = { -- disable formatting capabilities for the listed language servers
                -- "sumneko_lua",
            },
            timeout_ms = 1000, -- default format timeout
            -- filter = function(client) -- fully override the default formatting function
            --   return true
            -- end
        },
        -- easily add or disable built in mappings added during LSP attaching
        mappings = {
            n = {},
        },
        ["server-settings"] = {},
    },
    -- Mapping data with "desc" stored directly by vim.keymap.set().
    mappings = {
        n = {
            -- tab
            ["<leader>bb"] = { "<cmd>tabnew<cr>", desc = "New tab" },
            ["<leader>bc"] = { "<cmd>BufferLinePickClose<cr>", desc = "Pick to close" },
            ["<leader>bj"] = { "<cmd>BufferLinePick<cr>", desc = "Pick to jump" },
            ["<leader>bt"] = { "<cmd>BufferLineSortByTabs<cr>", desc = "Sort by tabs" },
            ["<leader>bw"] = { "<cmd>bw<CR>", desc = "Close current tab" },
            ["<leader>bW"] = { "<cmd>close<CR>", desc = "Close current split window" },
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
            -- hardmode (no arrows)
            ["<Up>"] = { "<nop>" },
            ["<Down>"] = { "<nop>" },
            ["<Left>"] = { "<nop>" },
            ["<Right>"] = { "<nop>" },
        },
        x = {
            ["<leader>p"] = { [["_dP]], desc = "Paste without register" },
        },
        t = {
        },
        v = {
            ["<leader>d"] = { [["_d]], desc = "Delete without register" },
            -- move lines up and down like option arrows
            ["K"] = { ":m '<-2<CR>gv=gv", desc = "Move selection up" },
            ["J"] = { ":m '>+1<CR>gv=gv", desc = "Move selection down" },
        },
    },
    -- Configure plugins
    plugins = {
        init = {
            { 'TimUntersberger/neogit', requires = 'nvim-lua/plenary.nvim', },
            { "github/copilot.vim" },
            { 'justinmk/vim-sneak' },
            { 'mg979/vim-visual-multi' },
            { "tpope/vim-surround" },
            {
                'xbase-lab/xbase',
                run = 'make install',
                requires = {
                    "nvim-lua/plenary.nvim",
                    "nvim-telescope/telescope.nvim",
                    "neovim/nvim-lspconfig"
                },
                config = function()
                    require 'xbase'.setup()
                end
            },
            {
                "simrat39/rust-tools.nvim",
                after = "mason-lspconfig.nvim", -- make sure to load after mason-lspconfig
                config = function()
                    require("rust-tools").setup {
                        server = astronvim.lsp.server_settings "rust_analyzer", -- get the server settings and built in capabilities/on_attach
                    }
                end,
            },
            {
                'saecki/crates.nvim',
                tag = 'v0.3.0',
                requires = { 'nvim-lua/plenary.nvim' },
                config = function()
                    require('crates').setup()
                end
            },
            { "towolf/vim-helm" },
        },
        ["null-ls"] =
            function(config)  -- overrides `require("null-ls").setup(config)`
                config.sources = {}
                return config -- return final config table
            end,
        treesitter = {},
        ["mason-lspconfig"] = {
            ensure_installed = { "rust_analyzer" }, -- install rust_analyzer
        },
        ["mason-null-ls"] = {},
        ["mason-nvim-dap"] = {},
        ["neo-tree"] = NeotreeConfig,
        heirline = HeirlineConfig,
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
    -- Modify which-key registration (Use this with mappings table in the above.)
    ["which-key"] = {
        register = {
            n = {
                ["<leader>"] = {
                    -- group name in which-key top level menu
                    ["b"] = { name = "Buffer" },
                },
            },
        },
    },
    -- This function is run last and is a good place to configuring
    -- augroups/autocommands and custom filetypes also this just pure lua so
    -- anything that doesn't fit in the normal config locations above can go here
    polish = configurePolish
}

return config
