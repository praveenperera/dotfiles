-- AstroLSP allows you to customize the features in AstroNvim's LSP configuration engine
-- Configuration documentation can be found with `:h astrolsp`
-- NOTE: We highly recommend setting up the Lua Language Server (`:LspInstall lua_ls`)
--       as this provides autocomplete and documentation while editing

---@type LazySpec
return {
    "AstroNvim/astrolsp",
    ---@type AstroLSPOpts
    opts = {
        -- Configuration table of features provided by AstroLSP
        features = {
            autoformat = true, -- enable or disable auto formatting on start
            codelens = true, -- enable/disable codelens refresh on start
            inlay_hints = false, -- enable/disable inlay hints on start
            semantic_tokens = true, -- enable/disable semantic token highlighting
        },
        -- customize lsp formatting options
        formatting = {
            -- control auto formatting on save
            format_on_save = {
                enabled = true, -- enable or disable format on save globally
                allow_filetypes = { -- enable format on save for specified filetypes only
                    -- "go",
                },
                ignore_filetypes = { -- disable format on save for specified filetypes
                    -- "python",
                },
            },
            disabled = {},
            timeout_ms = 1000, -- default format timeout
            -- filter = function(client) -- fully override the default formatting function
            --   return true
            -- end
        },
        -- enable servers that you already have installed without mason
        servers = {
            "sourcekit",
        },
        -- customize language server configuration options passed to `lspconfig`
        ---@diagnostic disable: missing-fields
        config = {
            tailwind = function()
                return {
                    filetypes = {
                        "html",
                        "css",
                        "scss",
                        "javascript",
                        "javascriptreact",
                        "typescript",
                        "typescriptreact",
                        "vue",
                        "jinja",
                    },

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
                            jinja = "html",
                        },
                    },
                }
            end,
            emmet_language_server = function()
                return {
                    filetypes = {
                        "css",
                        "eruby",
                        "html",
                        "javascript",
                        "javascriptreact",
                        "less",
                        "sass",
                        "scss",
                        "svelte",
                        "pug",
                        "typescriptreact",
                        "vue",
                        "jinja",
                        "heex",
                        "elixir",
                    },

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
        -- Configure buffer local auto commands to add when attaching a language server
        autocmds = {
            -- first key is the `augroup` to add the auto commands to (:h augroup)
            lsp_document_highlight = {
                -- Optional condition to create/delete auto command group
                -- can either be a string of a client capability or a function of `fun(client, bufnr): boolean`
                -- condition will be resolved for each client on each execution and if it ever fails for all clients,
                -- the auto commands will be deleted for that buffer
                cond = "textDocument/documentHighlight",
                -- cond = function(client, bufnr) return client.name == "lua_ls" end,
                -- list of auto commands to set
                {
                    -- events to trigger
                    event = { "CursorHold", "CursorHoldI" },
                    -- the rest of the autocmd options (:h nvim_create_autocmd)
                    desc = "Document Highlighting",
                    callback = function()
                        vim.lsp.buf.document_highlight()
                    end,
                },
                {
                    event = { "CursorMoved", "CursorMovedI", "BufLeave" },
                    desc = "Document Highlighting Clear",
                    callback = function()
                        vim.lsp.buf.clear_references()
                    end,
                },
            },
        },
        -- mappings to be set up on attaching of a language server
        mappings = {
            n = {
                gl = {
                    function()
                        vim.diagnostic.open_float()
                    end,
                    desc = "Hover diagnostics",
                },
                -- a `cond` key can provided as the string of a server capability to be required to attach, or a function with `client` and `bufnr` parameters from the `on_attach` that returns a boolean
                -- gD = {
                --   function() vim.lsp.buf.declaration() end,
                --   desc = "Declaration of current symbol",
                --   cond = "textDocument/declaration",
                -- },
                -- ["<Leader>uY"] = {
                --   function() require("astrolsp.toggles").buffer_semantic_tokens() end,
                --   desc = "Toggle LSP semantic highlight (buffer)",
                --   cond = function(client) return client.server_capabilities.semanticTokensProvider and vim.lsp.semantic_tokens end,
                -- },
            },
        },
        -- A custom `on_attach` function to be run after the default `on_attach` function
        -- takes two parameters `client` and `bufnr`  (`:h lspconfig-setup`)
        on_attach = function(_client, _bufnr)
            -- this would disable semanticTokensProvider for all clients
            -- client.server_capabilities.semanticTokensProvider = nil
        end,
    },
}
