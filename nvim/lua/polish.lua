-- Set filetype for terraform files
vim.cmd("au BufRead,BufNewFile *.tfvars set filetype=terraform")

-- Set filetype for jinja
vim.cmd("au BufNewFile,BufRead *.j2,*.jinja set ft=jinja")

--  Config rustaceanvim
vim.g.rustaceanvim = require("config.rustaceanvim").config()
