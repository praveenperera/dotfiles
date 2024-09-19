local bufnr = vim.api.nvim_get_current_buf()

vim.keymap.set("n", "<leader>mp", function()
    vim.cmd.MarkdownPreview()
end, { silent = true, buffer = bufnr, desc = "Markdown Preview" })

vim.keymap.set(
    "n",
    "<leader>mt",
    "<CMD>RenderMarkdown toggle<CR>",
    { silent = true, buffer = bufnr, desc = "Render Markdown Toggle" }
)
