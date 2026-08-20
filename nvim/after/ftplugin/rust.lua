local bufnr = vim.api.nvim_get_current_buf()

vim.keymap.set("n", "<Leader>lx", function()
    vim.notify("Restarting LSP")
    vim.cmd("lsp restart")
end, { buffer = bufnr, desc = "Restart LSP", silent = true })
