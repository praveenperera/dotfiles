local M = {}

M.config = function(_, opts)
    opts.tools = {
        inlay_hints = {
            auto = false,
            only_current_line = true
        }
    }

    return opts
end

return M
