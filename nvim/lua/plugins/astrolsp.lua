local emmet_filetypes = {
    "astro",
    "css",
    "eruby",
    "html",
    "htmlangular",
    "htmldjango",
    "javascriptreact",
    "jinja",
    "jinja_html",
    "less",
    "pug",
    "sass",
    "scss",
    "svelte",
    "templ",
    "typescriptreact",
    "vue",
}

local servers = {
    "cssls",
    "dockerls",
    "emmet_ls",
    "gopls",
    "html",
    "jsonls",
    "just",
    "lua_ls",
    "marksman",
    "sourcekit",
    "taplo",
    "vtsls",
    "yamlls",
}

---@type LazySpec
return {
    "AstroNvim/astrolsp",
    ---@param opts AstroLSPOpts
    opts = function(_, opts)
        opts.features = vim.tbl_deep_extend("force", opts.features or {}, {
            codelens = true,
            inlay_hints = false,
            semantic_tokens = true,
        })

        opts.servers =
            require("astrocore").list_insert_unique(opts.servers or {}, servers)

        opts.config = opts.config or {}

        local rust_analyzer = opts.config.rust_analyzer or {}
        rust_analyzer.settings = rust_analyzer.settings or {}
        rust_analyzer.settings["rust-analyzer"] = rust_analyzer.settings["rust-analyzer"]
            or {}
        rust_analyzer.settings["rust-analyzer"].check = { command = "check" }
        opts.config.rust_analyzer = rust_analyzer

        opts.config.taplo =
            vim.tbl_deep_extend("force", opts.config.taplo or {}, {
                cmd_env = { RUST_LOG = "error" },
                settings = {
                    evenBetterToml = {
                        formatter = {
                            columnWidth = 120,
                            arrayAutoExpand = true,
                            arrayAutoCollapse = false,
                            compactArrays = true,
                        },
                    },
                },
            })

        opts.config.emmet_ls = vim.tbl_deep_extend(
            "force",
            opts.config.emmet_ls or {},
            { filetypes = emmet_filetypes }
        )

        opts.autocmds = opts.autocmds or {}
        opts.autocmds.lsp_document_highlight = {
            cond = "textDocument/documentHighlight",
            {
                event = { "CursorHold", "CursorHoldI" },
                desc = "Highlight document references",
                callback = vim.lsp.buf.document_highlight,
            },
            {
                event = { "CursorMoved", "CursorMovedI", "BufLeave" },
                desc = "Clear document references",
                callback = vim.lsp.buf.clear_references,
            },
        }

        opts.mappings = opts.mappings or {}
        opts.mappings.n = opts.mappings.n or {}
        opts.mappings.n.gl = {
            vim.diagnostic.open_float,
            desc = "Hover diagnostics",
        }
        opts.mappings.n["<Leader>lI"] = false
    end,
}
