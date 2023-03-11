local default = {}

default.config = function()
    return {
        ensure_installer = {
            "lua",
            "vim"
        }
    }
end

return default
