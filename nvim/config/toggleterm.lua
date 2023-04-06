local M = {}

M.config = function(_, _)
    local opts = {
        size = 15,
        persist_size = true,
        start_in_insert = true,
    }

    return opts
end


return M
