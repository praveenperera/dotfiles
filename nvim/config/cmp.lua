local M = {}


M.config = function(_, opts)
    local cmp = require "cmp"

    opts.sources = cmp.config.sources {
        { name = "nvim_lsp", priority = 1000 },
        { name = "crates",   priority = 800 },
        { name = "luasnip",  priority = 750 },
        { name = "emoji",    priority = 700 },
        { name = "buffer",   priority = 500 },
        { name = "path",     priority = 250 },
    }

    return opts
end

return M
