-- init.lua
if vim.env.SSH_CONNECTION then -- only when remoted in
    vim.g.clipboard = require("vim.clipboard.osc52") -- built-in helper
end

-- use "+ register by default
vim.opt.clipboard:append({ "unnamedplus" })
