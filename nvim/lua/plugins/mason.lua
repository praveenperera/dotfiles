---@type LazySpec
return {
    {
        "mason-org/mason-lspconfig.nvim",
        opts = function(_, opts)
            opts.ensure_installed =
                require("astrocore").list_insert_unique(opts.ensure_installed, {
                    "dockerls",
                    "lua_ls",
                })
        end,
    },

    -- use mason-null-ls to configure Formatters/Linter installation for null-ls sources
    {
        "jay-babu/mason-null-ls.nvim",
        opts = function(_, opts)
            opts.ensure_installed =
                require("astrocore").list_insert_unique(opts.ensure_installed, {
                    "prettier",
                    "stylua",
                })
        end,
    },
}
