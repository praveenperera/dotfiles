local default = {}

local function config(_, _)
    local opts = {}
    opts.filesystem = {
        filtered_items = {
            visible = true,
            hide_dotfiles = false,
            hide_gitignored = true,
            hide_by_pattern = {
                ".git",
            },
            never_show = {
                ".DS_Store",
            },
        },
    }

    return opts
end

default.config = config
return default
