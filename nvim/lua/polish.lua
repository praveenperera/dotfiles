local cove = require("config.cove_build")

-- set filetype for terraform files
vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
    pattern = "*.tfvars",
    callback = function()
        vim.bo.filetype = "terraform"
    end,
})

-- set filetype for jinja
vim.api.nvim_create_autocmd({ "BufNewFile", "BufRead" }, {
    pattern = { "*.j2", "*.jinja" },
    callback = function()
        vim.bo.filetype = "jinja"
    end,
})

local cove_path = vim.fn.expand("~/code/bitcoinppl/cove/*")
local cove_group = vim.api.nvim_create_augroup("cove-build", { clear = true })

vim.api.nvim_create_autocmd("BufEnter", {
    pattern = cove_path,
    group = cove_group,
    callback = function(args)
        cove.setup_build_commands(args.buf)
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
        vim.opt_local.tabstop = 2 -- Number of spaces for a tab
        vim.opt_local.shiftwidth = 2 -- Spaces per indent level
    end,
})

-- ssh clipboard
if vim.env.SSH_CONNECTION then -- only when remoted in
    vim.g.clipboard = "osc52"
end

-- use "+ register by default
vim.opt.clipboard:append({ "unnamedplus" })

-- tell netrw to use /tmp/netrw for its local copy
vim.g.netrw_localcopydir = "/tmp/netrw"
-- don’t recreate the remote directory tree locally
vim.g.netrw_keepdir = 0
-- turn off backup files (so netrw won’t try to write *.swp next to the remote path)
vim.g.netrw_backup = 0
-- optional: narrow the netrw split height
vim.g.netrw_winsize = 20
vim.cmd.packadd("netrw")
