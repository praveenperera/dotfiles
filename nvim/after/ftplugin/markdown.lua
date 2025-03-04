local bufnr = vim.api.nvim_get_current_buf()

vim.keymap.set(
    "n",
    "<leader>mo",
    "<CMD>PeekOpen<CR>",
    { silent = true, buffer = bufnr, desc = "[M]arkdown [O]pen Preview" }
)

vim.keymap.set(
    "n",
    "<leader>mc",
    "<CMD>PeekClose<CR>",
    { silent = true, buffer = bufnr, desc = "[M]arkdown [C]lose Preview" }
)

-- vim.keymap.set(
--     "n",
--     "<leader>mt",
--     "<CMD>RenderMarkdown toggle<CR>",
--     { silent = true, buffer = bufnr, desc = "Render Markdown Toggle" }
-- )
