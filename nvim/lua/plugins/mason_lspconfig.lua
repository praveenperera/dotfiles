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

return { "williamboman/mason-lspconfig.nvim", opts = config }
