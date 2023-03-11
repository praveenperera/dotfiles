local default = {}

local function config(_, opts)
    opts.disable_commit_confirmation = true
    opts.disable_context_highlighting = false

    return opts
end

default.config = config
return default
