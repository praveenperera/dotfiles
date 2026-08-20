---@type LazySpec
return {
    "mrcjkb/rustaceanvim",
    version = "^9",
    ---@param opts table
    opts = function(_, opts)
        opts.server = opts.server or {}
        opts.server.cmd = { "ra-multiplex", "client" }

        local original_on_attach = opts.server.on_attach
        opts.server.on_attach = function(client, bufnr)
            if original_on_attach then
                original_on_attach(client, bufnr)
            else
                require("astrolsp").on_attach(client, bufnr)
            end

            local function map(key, callback, description)
                vim.keymap.set("n", key, callback, {
                    buffer = bufnr,
                    desc = description,
                    silent = true,
                })
            end

            local function open_cargo()
                vim.cmd.RustLsp("openCargo")
            end

            map("<Leader>lz", open_cargo, "Open Cargo.toml")
            map("<Leader>lC", open_cargo, "Open Cargo.toml")
            map("<Leader>lc", function()
                vim.cmd.RustLsp("externalDocs")
            end, "Open external documentation")

            if client.server_capabilities.inlayHintProvider then
                map("<Leader>lt", function()
                    vim.lsp.inlay_hint.enable(
                        not vim.lsp.inlay_hint.is_enabled({ bufnr = bufnr }),
                        { bufnr = bufnr }
                    )
                end, "Toggle inlay hints")
            end
        end
    end,
}
