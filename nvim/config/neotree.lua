local default = {}

local function config(conf)
    conf.filesystem = {
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

    return conf
end

default.config = config

return default
