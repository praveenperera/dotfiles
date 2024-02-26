local M = {}

local function config(_, opts)
    opts.ensure_installed = {
        "lua_ls",
        "tsserver",
        "tflint",
        "tailwindcss",
        "pyre",
        "pyright",
        "gopls",
    }

    return opts
end

M.config = config
return M
