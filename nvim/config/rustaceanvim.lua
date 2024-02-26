local M = {}

M.config = function(_, opts)
    opts = {}

    -- Plugin configuration
    opts.tools = {
        inlay_hints = {
            auto = true,
            only_current_line = true
        }
    }


    -- LSP configuration
    opts.server = {
        on_attach = function(client, bufnr)
            -- default astrovim on_attach
            require("astronvim.utils.lsp").on_attach(client, bufnr)

            vim.keymap.set(
                "n",
                "<leader>a",
                function()
                    vim.cmd.RustLsp('codeAction')
                end,
                { silent = true, buffer = bufnr, desc = "Rust Code Action" }
            )

            vim.keymap.set(
                "n",
                "<leader>lC",
                function()
                    vim.cmd.RustLsp('openCargo')
                end,
                { silent = true, buffer = bufnr, desc = "Open Cargo.toml" }
            )


            vim.keymap.set(
                "n",
                "<leader>lc",
                function()
                    vim.cmd.RustLsp('externalDocs')
                end,
                { silent = true, buffer = bufnr, desc = "Open docs.rs" }
            )
        end,
        -- default_settings = {
        --     -- rust-analyzer language server configuration
        --     -- ['rust-analyzer'] = {},
        -- },
    }

    -- DAP configuration
    -- opts.dap = {}

    return opts
end

return M
