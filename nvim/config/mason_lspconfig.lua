local M = {}

local function config(_, opts)
    opts.ensure_installed = {
        "rust_analyzer",
        "lua_ls",
        "tsserver",
        "tflint",
        "tailwindcss",
    }

    return opts
end

M.config = config
return M
