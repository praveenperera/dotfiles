local default = {}

default.config = function(_, _)
    return {
        autotag = { enable = true },
        highlight = { enable = true },
        indent = { enable = true },
        additional_vim_regex_highlighting = false,
        auto_install = true,
        ensure_installed = {
            "lua",
            "vim",
        },
    }
end

return default
