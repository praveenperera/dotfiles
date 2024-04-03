local M = {}

M.config = function(_, opts)
    local function file_exists(path)
        local file = io.open(path, "r")
        if file then
            file:close()
            return true
        else
            return false
        end
    end

    opts = {}

    -- Plugin configuration
    opts.tools = {
        inlay_hints = {
            auto = true,
            only_current_line = true,
        },
    }

    -- LSP configuration
    opts.server = {
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
            -- default astrovim on_attach
            require("astrolsp").on_attach(client, bufnr)

            vim.keymap.set("n", "<Leader>a", function()
                vim.cmd.RustLsp("codeAction")
            end, {
                silent = true,
                buffer = bufnr,
                desc = "Rust Code Action",
            })

            vim.keymap.set("n", "<Leader>lC", function()
                vim.cmd.RustLsp("openCargo")
            end, {
                silent = true,
                buffer = bufnr,
                desc = "Open Cargo.toml",
            })

            vim.keymap.set("n", "<Leader>lc", function()
                vim.cmd.RustLsp("externalDocs")
            end, {
                silent = true,
                buffer = bufnr,
                desc = "Open docs.rs",
            })
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
