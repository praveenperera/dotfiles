-- highlight on yank
vim.api.nvim_create_autocmd("TextYankPost", {
    desc = "Highlight when yanking (copying) text",
    group = vim.api.nvim_create_augroup("highlight-yank", { clear = true }),
    callback = function()
        vim.highlight.on_yank()
    end,
})

-- set filetype for Fastfile
vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
    pattern = "Fastfile",
    callback = function()
        vim.bo.filetype = "ruby"
    end,
})
