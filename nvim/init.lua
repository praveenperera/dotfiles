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
        config = {
            tailwindcss = function()
                return {
                    filetypes = { "html", "css", "scss", "javascript", "javascriptreact", "typescript",
                        "typescriptreact", "vue", "jinja" },

                    init_options = {
                        userLanguages = {
                            html = "html",
                            css = "css",
                            scss = "scss",
                            javascript = "javascript",
                            javascriptreact = "javascript",
                            typescript = "typescript",
                            typescriptreact = "typescript",
                            vue = "vue",
                            jinja = "html"
                        },
                    },
                }
            end,
            emmet_language_server = function()
                return {
                    filetypes = { "css", "eruby", "html", "javascript", "javascriptreact", "less", "sass", "scss",
                        "svelte", "pug", "typescriptreact", "vue", "jinja", "heex", "elixir" },

                    init_options = {
                        --- @type table<string, any> https://docs.emmet.io/customization/preferences/
                        preferences = {},
                        --- @type "always" | "never" defaults to `"always"`
                        showexpandedabbreviation = "always",
                        --- @type boolean defaults to `true`
                        showabbreviationsuggestions = true,
                        --- @type boolean defaults to `false`
                        showsuggestionsassnippets = false,
                        --- @type table<string, any> https://docs.emmet.io/customization/syntax-profiles/
                        syntaxprofiles = {},
                        --- @type table<string, string> https://docs.emmet.io/customization/snippets/#variables
                        variables = {},
                        --- @type string[]
                        excludelanguages = {},
                    },
                }
            end,
        },
        servers = {},
        skip_setup = {},
        setup_handlers = {
            tsserver = function(_, opts)
                require("typescript").setup({ server = opts })
            end,
        },
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
        },
    },
    -- LuaSnip Options
    luasnip = {
        filetype_extend = {},
        vscode = {
            paths = {},
        },
    },
    -- Run after everything is loaded
    polish = function()
        -- Set filetype for terraform files
        vim.cmd("au BufRead,BufNewFile *.tfvars set filetype=terraform")

        -- Set filetype for jinja
        vim.cmd("au BufNewFile,BufRead *.j2,*.jinja set ft=jinja")

        vim.g.rustaceanvim = require("user.config.rustaceanvim").config()
    end,
}

return config
