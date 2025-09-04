local config = function(_, _opts)
    local function file_exists(path)
        local file = io.open(path, "r")
        if file then
            file:close()
            return true
        else
            return false
        end
    end

    local opts = _opts or {}

    -- Plugin configuration
    opts.tools = {
        inlay_hints = {
            auto = true,
            only_current_line = true,
        },
    }

    -- LSP configuration
    opts.server = {
        cmd = { "ra-multiplex", "client" },

        settings = function(project_root)
            local ra = require("rustaceanvim.config.server")

            local current_file = vim.fn.getcwd() .. "/" .. "rust-analyzer.json"
            local closest_root_with_config

            if file_exists(current_file) then
                closest_root_with_config = vim.fn.getcwd()
            else
                closest_root_with_config = project_root
            end

            if closest_root_with_config ~= project_root then
                vim.notify(
                    "Using rust-analyzer.json from " .. closest_root_with_config
                )
            end

            return ra.load_rust_analyzer_settings(closest_root_with_config, {
                settings_file_pattern = "rust-analyzer.json",
            })
        end,

        on_attach = function(client, bufnr)
            local function desc(description)
                return {
                    noremap = true,
                    silent = true,
                    buffer = bufnr,
                    desc = description,
                }
            end

            -- default astrovim on_attach
            require("astrolsp").on_attach(client, bufnr)

            vim.keymap.set("n", "<Leader>lC", function()
                vim.cmd.RustLsp("openCargo")
            end, desc("Open Cargo.toml"))

            vim.keymap.set("n", "<Leader>lc", function()
                vim.cmd.RustLsp("externalDocs")
            end, desc("Open external docs.rs"))

            if client.server_capabilities.inlayHintProvider then
                vim.keymap.set("n", "<Leader>lt", function()
                    vim.lsp.inlay_hint.enable(
                        not vim.lsp.inlay_hint.is_enabled()
                    )
                end, desc("Toggle inlay hints"))
            end
        end,

        default_settings = {
            -- rust-analyzer language server configuration
            ["rust-analyzer"] = {
                check = { command = "check" },
                checkOnSave = { command = "check" },
            },
        },
    }

    -- DAP configuration
    -- opts.dap = {}

    return opts
end

return {
    "mrcjkb/rustaceanvim",
    version = "^4",
    ft = { "rust" },
    lazy = false,
    config = function()
        vim.g.rustaceanvim = config
    end,
}
