local default = {}

local function config()
    return {
        ensure_installed = {
            "rust_analyzer",
            "lua_ls",
            "tsserver",
            "yamlls",
            "tflint",
            "tailwindcss"
        }
    }
end

default.config = config
return default
