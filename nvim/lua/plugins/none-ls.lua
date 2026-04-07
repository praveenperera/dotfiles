return {
    {
        "nvimtools/none-ls.nvim",
        commit = "7f9301e416533b5d74e2fb3b1ce5059eeaed748b",
        version = false,
        opts = function(_, opts)
            opts = opts or {}

            local prev_on_init = opts.on_init
            opts.on_init = function(client, initialize_result)
                local methods = require("null-ls.methods")

                client.supports_method = function(maybe_self, method)
                    local actual_method = maybe_self == client and method or maybe_self
                    local capability_map = vim.lsp.protocol._request_name_to_capability
                        or vim.lsp._request_name_to_capability
                        or vim.lsp.protocol._request_name_to_server_capability
                    local required_capability = capability_map and capability_map[actual_method]

                    if not required_capability
                        or vim.tbl_get(client.server_capabilities, unpack(required_capability)) == false
                    then
                        return false
                    end

                    local internal_method = methods.map[actual_method]
                    if internal_method then
                        return require("null-ls.generators").can_run(vim.bo.filetype, internal_method)
                    end

                    return methods.lsp[actual_method] ~= nil
                end

                if prev_on_init then
                    prev_on_init(client, initialize_result)
                end
            end

            return opts
        end,
    },
}
