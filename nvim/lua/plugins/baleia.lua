return {
    "m00qek/baleia.nvim",
    version = "*",
    cmd = "BaleiaColorize",
    config = function()
        vim.g.conjure_baleia = require("baleia").setup({ line_starts_at = 3 })

        local augroup =
            vim.api.nvim_create_augroup("ConjureBaleia", { clear = true })

        vim.api.nvim_create_autocmd({ "BufEnter" }, {
            pattern = "conjure-log-*",
            group = augroup,
            callback = function()
                vim.keymap.set(
                    { "n", "v" },
                    "[c",
                    "<CMD>call search('^; -\\+$', 'bw')<CR>",
                    {
                        silent = true,
                        buffer = true,
                        desc = "Jumps to the begining of previous evaluation output.",
                    }
                )
                vim.keymap.set(
                    { "n", "v" },
                    "]c",
                    "<CMD>call search('^; -\\+$', 'w')<CR>",
                    {
                        silent = true,
                        buffer = true,
                        desc = "Jumps to the begining of next evaluation output.",
                    }
                )
            end,
        })

        vim.api.nvim_create_user_command("BaleiaColorize", function()
            vim.g.conjure_baleia.once(vim.api.nvim_get_current_buf())
        end, { bang = true })

        vim.api.nvim_create_user_command(
            "BaleiaLogs",
            vim.g.conjure_baleia.logger.show,
            { bang = true }
        )
    end,
}
