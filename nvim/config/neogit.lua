local default = {}

local function config()
    local conf = {
        disable_commit_confirmation = true,
        disable_context_highlighting = false,
    }
    return conf
end

default.config = config
return default
