local mappings = require("mappings")

---@type LazySpec
return {
    "AstroNvim/astrocore",
    ---@type AstroCoreOpts
    opts = {
        -- Configure core features of AstroNvim
        features = {
            large_buf = { size = 1024 * 500, lines = 10000 }, -- set global limits for large files for disabling features like treesitter
            autopairs = true, -- enable autopairs at start
            cmp = true, -- enable completion at start
            diagnostics_mode = 3, -- diagnostic mode on start (0 = off, 1 = no signs/virtual text, 2 = no virtual text, 3 = on)
            highlighturl = true, -- highlight URLs at start
            notifications = true, -- enable notifications at start
            inccommand = "split",
        },
        -- Diagnostics configuration (for vim.diagnostics.config({...})) when diagnostics are on
        diagnostics = {
            virtual_text = true,
            underline = true,
        },
        -- vim options can be configured here
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
                hlsearch = true,
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
        -- Mappings can be configured through AstroCore as well.
        -- NOTE: keycodes follow the casing in the vimdocs. For example, `<Leader>` must be capitalized
        mappings = mappings,
    },
}
