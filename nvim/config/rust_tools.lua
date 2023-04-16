local M = {}

M.config = function(_, opts)
    opts.tools = {
        inlay_hints = {
            auto = true,
            only_current_line = true
        }
    }

    local on_attach =
        function(client, bufnr)
            -- default astrovim on_attach
            require("astronvim.utils.lsp").on_attach(client, bufnr)

            -- custom on_attach
            vim.keymap.set("n", "<leader>lc", "<cmd>RustOpenExternalDocs<cr>",
                { desc = "Open external docs", buffer = bufnr })
        end

    opts.server.on_attach = on_attach

    return opts
end

return M
