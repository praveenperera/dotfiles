---@type LazySpec
return {
    { "nvimtools/none-ls.nvim", enabled = false },
    { "jay-babu/mason-null-ls.nvim", enabled = false },
    {
        "mason-org/mason-lspconfig.nvim",
        opts = function(_, opts)
            opts.automatic_enable = false
            opts.ensure_installed = require("astrocore").list_insert_unique(
                opts.ensure_installed,
                { "dockerls", "lua_ls" }
            )
        end,
    },
    {
        "WhoIsSethDaniel/mason-tool-installer.nvim",
        opts = function(_, opts)
            opts.ensure_installed = require("astrocore").list_insert_unique(
                opts.ensure_installed or {},
                { "swiftformat", "swiftlint" }
            )
        end,
    },
    {
        "stevearc/conform.nvim",
        opts = {
            formatters_by_ft = {
                rust = { "rustfmt" },
                swift = { "swiftformat" },
            },
        },
    },
    {
        "mfussenegger/nvim-lint",
        opts = function(_, opts)
            opts.linters_by_ft = opts.linters_by_ft or {}
            opts.linters_by_ft.swift = { "swiftlint" }

            -- the community executable filter requires concrete linter tables
            local swiftlint = require("lint.linters.swiftlint")()
            swiftlint.stdin = true
            swiftlint.args = { "lint", "--use-stdin" }

            opts.linters = opts.linters or {}
            opts.linters.swiftlint = swiftlint
        end,
    },
}
