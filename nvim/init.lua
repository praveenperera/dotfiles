local neotree = require("user.config.neotree")
local neogit = require("user.config.neogit")
local heirline = require("user.config.heirline")
local telescope = require("user.config.telescope")
local treesitter = require("user.config.treesitter")
local rust_tools = require("user.config.treesitter")
local mason_lspconfig = require("user.config.mason_lspconfig")

local config = {
	updater = {
		remote = "origin", -- remote to use
		channel = "stable", -- "stable" or "nightly"
		version = "latest", -- "latest", tag name, or regex search like "v1.*" to only do updates before v2 (STABLE ONLY)
		commit = nil,    -- commit hash (NIGHTLY ONLY)
		pin_plugins = nil, -- nil, true, false (nil will pin plugins on stable only)
		skip_prompts = false, -- skip prompts about breaking changes
		show_changelog = true, -- show the changelog after performing an update
		auto_reload = false, -- automatically reload and sync packer after a successful update
		auto_quit = false, -- automatically quit the current session after a successful update
	},
	colorscheme = "astrotheme",
	highlights = {},
	-- set vim options here (vim.<first_key>.<second_key> = value)
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
		},
		g = {
			mapleader = " ",          -- sets vim.g.mapleader
			autoformat_enabled = true, -- enable or disable auto formatting at start (lsp.formatting.format_on_save must be enabled)
			smp_enabled = true,       -- enable completion at start
			autopairs_enabled = true, -- enable autopairs at start
			diagnostics_enabled = true, -- enable diagnostics at start
			status_diagnostics_enabled = true, -- enable diagnostics in statusline
			icons_enabled = true,     -- disable icons in the UI (disable if no nerd font is available, requires :PackerSync after changing)
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
		servers        = {},
		skip_setup     = {},
		setup_handlers = {
			rust_analyzer = function(_, opts) require("rust-tools").setup { server = opts } end
		},
		formatting     = {
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
			["<leader>sr"] = { [[:%s/\<<C-r><C-w>\>/<C-r><C-w>/gI<Left><Left><Left>]], desc = "Replace" },
			["<leader>so"] = { '<cmd>lua require("spectre").open()<CR>', desc = "Open spectre" },
			["<leader>sw"] = {
				'<cmd>lua require("spectre").open_visual({select_word=true})<CR>',
				desc = "Search current word"
			},
			["<leader>sf"] = {
				'<cmd>lua require("spectre").open_file_search({select_word=true})<CR>',
				desc =
				"Search on current file"
			},
			-- tab
			-- ["<leader>bw"] = { "<cmd>bw<CR>", desc = "Close current tab" },
			-- ["<leader>bW"] = {
			-- 	"<cmd>close<CR>",
			-- 	desc = "Close current split window",
			-- },
			-- save and replace
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
			["<leader>Y"] = { [["+Y]], desc = "Yank to system" },
			["<leader>y"] = { [["+y]], desc = "Yank to system" },
			-- resize
			["<C-Home>"] = { "<C-w>+", desc = "Resize up" },
			["<C-End>"] = { "<C-w>-", desc = "Resize down" },
			-- find
			["<leader>ff"] = { telescope.find_files, desc = "Find all files" },
			["<leader>fs"] = { function() require("telescope").extensions.aerial.aerial() end },
			["<leader>fS"] = { function() require("telescope.builtin").lsp_dynamic_workspace_symbols() end },
			-- hardmode (no arrows)
			["<Up>"] = { "<nop>" },
			["<Down>"] = { "<nop>" },
			["<Left>"] = { "<nop>" },
			["<Right>"] = { "<nop>" },
		},
		x = {
			["<leader>p"] = { [["_dP]], desc = "Paste from system" },

		},
		t = {},
		v = {
			["<leader>s"] = { name = "Find and Replace" },
			["<leader>sw"] = {
				'<cmd>lua require("spectre").open_visual({select_word=true})<CR>',
				desc = "Search current word"
			},
			["<leader>d"] = { [["_d]], desc = "Delete without register" },
			-- move lines up and down like option arrows
			["K"] = { ":m '<-2<CR>gv=gv", desc = "Move selection up" },
			["J"] = { ":m '>+1<CR>gv=gv", desc = "Move selection down" },
		},
	},
	-- Configure plugins
	plugins = {
		{ "windwp/nvim-spectre",        event = "BufRead" },
		{
			"sindrets/diffview.nvim",
			dependencies = "nvim-lua/plenary.nvim",
			cmd = { "DiffviewOpen",
				"DiffviewRefresh" }
		},
		{ "kazhala/close-buffers.nvim", cmd = { "BDelete", "BWipeout" } },
		{ "kamykn/spelunker.vim",       event = "BufRead" },
		{ "AstroNvim/astrotheme" },
		{ "rebelot/heirline.nvim",      opts = heirline.config },
		{ "nvim-lua/plenary.nvim" },
		{
			"TimUntersberger/neogit",
			dependencies = { "nvim-lua/plenary.nvim" },
			opts = neogit.config,
			cmd = "Neogit"
		},
		{ "github/copilot.vim",     event = "User AstroLspSetup" },
		{
			"ggandor/leap.nvim",
			event = "BufRead",
			dependencies = { "tpope/vim-repeat" },
			config = function()
				require('leap').add_default_mappings()
			end
		},
		{ "tpope/vim-surround",     event = "BufRead" },
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
			"nvim-neo-tree/neo-tree.nvim",
			version = "v2.x",
			dependencies = {
				"nvim-lua/plenary.nvim",
				"nvim-tree/nvim-web-devicons",
				"MunifTanjim/nui.nvim",
			},
			opts = neotree.config,
		},
		{ "jose-elias-alvarez/typescript.nvim", event = "BufRead *.ts" },
		{
			"simrat39/rust-tools.nvim",
			event = "User AstroLspSetup",
			opts = rust_tools.config
		},
		{
			"williamboman/mason-lspconfig.nvim",
			opts = mason_lspconfig.config
		},
		{
			"saecki/crates.nvim",
			version = "v0.3.0",
			dependencies = { "nvim-lua/plenary.nvim" },
			event = "BufRead Cargo.toml",
			opts = {}
		},
		{ "nvim-treesitter/nvim-treesitter",    opts = treesitter.config },
		{ "ThePrimeagen/vim-be-good",           cmd = "VimBeGood" },
		{ "towolf/vim-helm",                    event = { "BufRead *.yaml", "BufRead *.tpl" } },
		{ "folke/neodev.nvim" }
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
		vim.cmd [[highlight SpelunkerSpellBad cterm=underline ctermfg=NONE gui=underline guifg=NONE]]
	end,
}

return config
