return {
    "stevearc/conform.nvim",
    event = { "BufReadPre", "BufNewFile" },
    config = function()
        local conform = require("conform")

        conform.setup({
            formatters_by_ft = {
                swift = { "swiftformat" },
            },
            format_on_save = function(_bufnr)
                return { timeout_ms = 500, lsp_fallback = true }
            end,
            log_level = vim.log.levels.ERROR,
        })
    end,
}
