local M = {}

local function config(_, _)
    return {
        disable_commit_confirmation = true,
        disable_context_highlighting = false,
        auto_refresh = true,
        integrations = {
            diffview = true,
        },
    }
end

M.config = config
return M
