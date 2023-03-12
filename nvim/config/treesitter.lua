local default = {}

default.config = function(_, _)
    return {
        autotag = { enable = true },
        highlight = true,
        auto_install = true,
        ensure_installed = {
            "lua",
            "vim",
            "kdl"
        }
    }
end

return default
