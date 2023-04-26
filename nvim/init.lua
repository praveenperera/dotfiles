local neotree = require("user.config.neotree")
local neogit = require("user.config.neogit")
local heirline = require("user.config.heirline")
local telescope = require("user.config.telescope")
local treesitter = require("user.config.treesitter")
local rust_tools = require("user.config.rust_tools")
local copilot = require("user.config.copilot")
local theme = require("user.config.theme")
local mason_lspconfig = require("user.config.mason_lspconfig")
local toggleterm = require("user.config.toggleterm")

local config = {
    updater = {
        remote = "origin", -- remote to use
        channel = "stable", -- "stable" or "nightly"
        version = "latest", -- "latest", tag name, or regex search like "v1.*" to only do updates before v2 (STABLE ONLY)
        commit = nil, -- commit hash (NIGHTLY ONLY)
        pin_plugins = nil, -- nil, true, false (nil will pin plugins on stable only)
        skip_prompts = false, -- skip prompts about breaking changes
        show_changelog = true, -- show the changelog after performing an update
        auto_reload = false, -- automatically reload and sync packer after a successful update
        auto_quit = false, -- automatically quit the current session after a successful update
    },
    colorscheme = "astrotheme",
    options = {
        opt = {
            -- set to true or false etc.
            relativenumber = true,
            number = true,
            spell = false,
            spelloptions = "camel",
            signcolumn = "auto",
            -- swap
            swapfile = false,
            backup = false,
            undodir = os.getenv("HOME") .. "/.vim/undodir",
            undofile = true,
            --
            wrap = false,
            -- colorcolumn = "80",
            termguicolors = true,
            expandtab = true,
            scrolloff = 5,
            -- indent
            tabstop = 4,
            smartindent = true,
            softtabstop = 4,
            shiftwidth = 4,
            -- highlight
            hlsearch = false,
            incsearch = true,
            -- hardmode ( no mouse )
            mouse = nil,
            -- disable autochange
            autochdir = false,
        },
        g = {
            mapleader = " ", -- sets vim.g.mapleader
            autoformat_enabled = true, -- enable or disable auto formatting at start (lsp.formatting.format_on_save must be enabled)
            smp_enabled = true, -- enable completion at start
            autopairs_enabled = true, -- enable autopairs at start
            diagnostics_enabled = true, -- enable diagnostics at start
            status_diagnostics_enabled = true, -- enable diagnostics in statusline
            icons_enabled = true, -- disable icons in the UI (disable if no nerd font is available, requires :PackerSync after changing)
            ui_notifications_enabled = true, -- disable notifications when toggling UI elements
            heirline_bufferline = false, -- enable new heirline based bufferline (requires :PackerSync after changing)
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
        skip_setup = {},
        setup_handlers = {
            tsserver = function(_, opts)
                require("typescript").setup({ server = opts })
            end,

            rust_analyzer = function(client, opts)
                require("rust-tools").setup(
                    rust_tools.config(client, { server = opts })
                )
            end,
        },
        formatting = {
            format_on_save = {
                enabled = true,
                allow_filetypes = {},
                ignore_filetypes = {},
            },
            disabled = { -- disable formatting capabilities for the listed language servers },
                timeout_ms = 1000, -- default format timeout
            },
            mappings = {
                n = {},
            },
            ["server-settings"] = {},
        },
    },
    -- Mapping data with "desc" stored directly by vim.keymap.set().
    mappings = {
        n = {
            -- spectre
            ["<leader>s"] = { name = "Find and Replace" },
            ["<leader>sr"] = {
                [[:%s/\<<C-r><C-w>\>/<C-r><C-w>/gI<Left><Left><Left>]],
                desc = "Replace",
            },
            ["<leader>so"] = {
                '<cmd>lua require("spectre").open()<CR>',
                desc = "Open spectre",
            },
            ["<leader>sw"] = {
                '<cmd>lua require("spectre").open_visual({select_word=true})<CR>',
                desc = "Search current word",
            },
            ["<leader>sf"] = {
                '<cmd>lua require("spectre").open_file_search({select_word=true})<CR>',
                desc = "Search on current file",
            },
            -- Harpoon
            ["<leader>h"] = { name = "Harpoon" },
            ["<leader>ha"] = {
                function()
                    require("harpoon.mark").add_file()
                end,
                desc = "Add file",
            },
            ["<leader>he"] = {
                function()
                    require("harpoon.ui").toggle_quick_menu()
                end,
                desc = "Toggle quick menu",
            },
            ["<leader>h1"] = {
                function()
                    require("harpoon.ui").nav_file(1)
                end,
                desc = "Go to file 1",
            },
            ["<leader>h2"] = {
                function()
                    require("harpoon.ui").nav_file(2)
                end,
                desc = "Go to file 2",
            },
            ["<leader>h3"] = {
                function()
                    require("harpoon.ui").nav_file(3)
                end,
                desc = "Go to file 3",
            },
            ["<leader>h4"] = {
                function()
                    require("harpoon.ui").nav_file(4)
                end,
                desc = "Go to file 4",
            },
            ["<leader>h5"] = {
                function()
                    require("harpoon.ui").nav_file(5)
                end,
                desc = "Go to file 5",
            },
            ["<leader>h6"] = {
                function()
                    require("harpoon.ui").nav_file(6)
                end,
                desc = "Go to file 6",
            },
            ["g1"] = {
                function()
                    require("harpoon.ui").nav_file(1)
                end,
                desc = "Go to file 1",
            },
            ["g2"] = {
                function()
                    require("harpoon.ui").nav_file(2)
                end,
                desc = "Go to file 2",
            },
            ["g3"] = {
                function()
                    require("harpoon.ui").nav_file(3)
                end,
                desc = "Go to file 3",
            },
            ["g4"] = {
                function()
                    require("harpoon.ui").nav_file(4)
                end,
                desc = "Go to file 4",
            },
            ["g5"] = {
                function()
                    require("harpoon.ui").nav_file(5)
                end,
                desc = "Go to file 5",
            },
            ["g6"] = {
                function()
                    require("harpoon.ui").nav_file(6)
                end,
                desc = "Go to file 6",
            },
            -- quick save
            ["<C-s>"] = { "<cmd>w!<cr>", desc = "Save File" },
            -- window navigation
            ["<leader>1"] = { "1<C-w>w", desc = "Go to window 1" },
            ["<leader>2"] = { "2<C-w>w", desc = "Go to window 2" },
            ["<leader>3"] = { "3<C-w>w", desc = "Go to window 3" },
            ["<leader>4"] = { "4<C-w>w", desc = "Go to window 4" },
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
            -- toggleterm
            ["<leader>th"] = {
                "<cmd>ToggleTerm direction=horizontal<cr>",
                desc = "Toggle horizontal terminal",
            },
            ["<leader>tv"] = {
                "<cmd>ToggleTerm direction=vertical<cr>",
                desc = "Toggle vertical terminal",
            },
            ["<leader>t1"] = {
                "<cmd>ToggleTerm 1<cr>",
                desc = "ToggleTerm 1st Window",
            },
            ["<leader>t2"] = {
                "<cmd>ToggleTerm 2<cr>",
                desc = "ToggleTerm 2nd Window",
            },
            ["<leader>t3"] = {
                "<cmd>ToggleTerm 3<cr>",
                desc = "ToggleTerm 3rd Window",
            },
            ["<leader>t4"] = {
                "<cmd>ToggleTerm 4<cr>",
                desc = "ToggleTerm 4th Window",
            },
            -- system yank
            ["<leader>d"] = { [["_d]], desc = "Delete without register" },
            ["<leader>Y"] = { [["+Y]], desc = "Yank to system" },
            ["<leader>y"] = { [["+y]], desc = "Yank to system" },
            -- resize
            ["<C-Home>"] = { "<C-w>+", desc = "Resize up" },
            ["<C-End>"] = { "<C-w>-", desc = "Resize down" },
            -- find
            ["<leader>fi"] = {
                "<cmd>Telescope current_buffer_fuzzy_find case_mode=ignore_case<CR>",
                desc = "Find in Buffer",
            },
            ["<leader>ft"] = { "<cmd>TodoTelescope<CR>", desc = "Find TODOs" },
            ["<leader>ff"] = { telescope.find_files, desc = "Find all files" },
            ["<leader>fs"] = {
                function()
                    require("telescope").extensions.aerial.aerial()
                end,
                desc = "Search document symbols",
            },
            ["<leader>fS"] = {
                function()
                    require("telescope.builtin").lsp_dynamic_workspace_symbols()
                end,
                desc = "Search project symbols",
            },
            -- undo
            ["<leader>U"] = { vim.cmd.UndotreeToggle, desc = "Undo tree" },
            -- hardmode (no arrows)
            ["<Up>"] = { "<nop>" },
            ["<Down>"] = { "<nop>" },
            ["<Left>"] = { "<nop>" },
            ["<Right>"] = { "<nop>" },
        },
        x = {
            ["<leader>p"] = { [["_dP]], desc = "Paste from system" },
        },
        t = {
            ["<esc><esc>"] = { [[<C-\><C-n>]], desc = "Normal mode" },
        },
        v = {
            ["<leader>s"] = { name = "Find and Replace" },
            ["<leader>sw"] = {
                '<cmd>lua require("spectre").open_visual({select_word=true})<CR>',
                desc = "Search current word",
            },
            ["<leader>d"] = { [["_d]], desc = "Delete without register" },
            -- move lines up and down like option arrows
            ["K"] = { ":m '<-2<CR>gv=gv", desc = "Move selection up" },
            ["J"] = { ":m '>+1<CR>gv=gv", desc = "Move selection down" },
        },
    },
    -- Configure plugins
    plugins = {
        { "AstroNvim/astrocommunity" },
        {
            "ThePrimeagen/harpoon",
            opts = {},
            event = "User AstroFile",
        },
        { "mbbill/undotree", event = "User AstroFile" },
        { "windwp/nvim-spectre", event = "BufRead" },
        {
            "folke/todo-comments.nvim",
            dependencies = "nvim-lua/plenary.nvim",
            opts = {},
            event = "BufRead",
            cmd = {
                "TodoQuickFix",
                "TodoLocList",
                "TodoTrouble",
                "TodoTelescope",
            },
        },
        { "wakatime/vim-wakatime", event = "BufRead" },
        {
            "sindrets/diffview.nvim",
            dependencies = "nvim-lua/plenary.nvim",
            cmd = { "DiffviewOpen", "DiffviewRefresh" },
        },
        { "kazhala/close-buffers.nvim", cmd = { "BDelete", "BWipeout" } },
        { "kamykn/spelunker.vim", event = "BufRead" },
        {
            "AstroNvim/astrotheme",
            opts = theme.config,
        },
        { "rebelot/heirline.nvim", opts = heirline.config },
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
        { "tpope/vim-surround", event = "BufRead" },
        { "mg979/vim-visual-multi", event = "BufRead" },
        {
            "xbase-lab/xbase",
            run = "make install",
            event = "BufRead *.swift",
            opts = {},
            dependencies = {
                "nvim-lua/plenary.nvim",
                "nvim-telescope/telescope.nvim",
                "neovim/nvim-lspconfig",
            },
        },
        {
            "akinsho/toggleterm.nvim",
            version = "*",
            opts = toggleterm.config,
            cmd = {
                "ToggleTerm",
                "ToggleTermToggleAll",
            },
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
        {
            "jose-elias-alvarez/typescript.nvim",
            event = "BufRead *.ts",
        },
        { "simrat39/rust-tools.nvim" },
        {
            "williamboman/mason-lspconfig.nvim",
            opts = mason_lspconfig.config,
        },
        {
            "saecki/crates.nvim",
            version = "v0.3.0",
            dependencies = { "nvim-lua/plenary.nvim" },
            event = "BufRead Cargo.toml",
            opts = {},
        },
        { "nvim-treesitter/nvim-treesitter", opts = treesitter.config },
        { "ThePrimeagen/vim-be-good", cmd = "VimBeGood" },
        {
            "towolf/vim-helm",
            event = { "BufRead *.yaml", "BufRead *.tpl" },
        },
        { "folke/neodev.nvim" },
        {
            "rust-sailfish/sailfish",
            rtp = "syntax/vim",
            event = "BufRead *.stpl",
        },
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
    polish = function() end,
}

return config
