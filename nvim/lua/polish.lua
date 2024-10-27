local cove = require("config.cove_build")

-- Set filetype for terraform files
vim.cmd("au BufRead,BufNewFile *.tfvars set filetype=terraform")

-- Set filetype for jinja
vim.cmd("au BufNewFile,BufRead *.j2,*.jinja set ft=jinja")

--  Config rustaceanvim
vim.g.rustaceanvim = require("config.rustaceanvim").config()

local home = vim.fn.expand("$HOME")
local project_path = home .. "/code/bitcoinppl/cove/*"

vim.api.nvim_create_autocmd("BufEnter", {
    pattern = project_path,
    callback = function()
        cove.setup_build_commands()
    end,
})

-- Disable auto indentation for text files
vim.api.nvim_create_autocmd("FileType", {
    pattern = { "text", "markdown", "xml" },
    callback = function()
        vim.opt_local.autoindent = false
        vim.opt_local.smartindent = false
        vim.opt_local.cindent = false
        vim.opt_local.indentexpr = ""

        vim.opt_local.expandtab = true -- Use spaces
        vim.opt_local.tabstop = 2      -- Number of spaces for a tab
        vim.opt_local.shiftwidth = 2   -- Spaces per indent level
    end,
})
