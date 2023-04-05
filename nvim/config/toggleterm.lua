local M = {}

M.config = function(_, _)
    local opts = {}

    opts.size = 20
    opts.persist_size = true
    opts.start_in_insert = true

    return opts
end


return M
