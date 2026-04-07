local lazypath = vim.env.LAZY or vim.fn.stdpath("data") .. "/lazy/lazy.nvim"
if not (vim.env.LAZY or (vim.uv or vim.loop).fs_stat(lazypath)) then
    -- stylua: ignore
    vim.fn.system({ "git", "clone", "--filter=blob:none", "https://github.com/folke/lazy.nvim.git", "--branch=stable",
        lazypath })
end
vim.opt.rtp:prepend(lazypath)

if vim.fn.has("nvim-0.12") == 1 then
    -- keep deprecated vim.lsp.with callers working without noisy startup warnings on nvim 0.12
    vim.lsp.with = function(handler, override_config)
        return function(err, result, ctx, config)
            return handler(err, result, ctx, vim.tbl_deep_extend("force", config or {}, override_config or {}))
        end
    end

    -- keep deprecated codelens refresh callers working until upstream configs switch to enable()
    vim.lsp.codelens.refresh = function(opts)
        return vim.lsp.codelens.enable(true, { bufnr = opts and opts.bufnr })
    end

    -- keep deprecated client.supports_method callers working until upstream plugins switch to method syntax
    local lsp_client = require("vim.lsp.client")
    local create_client = lsp_client.create
    lsp_client.create = function(config)
        local client = create_client(config)
        local supports_method = client.supports_method

        client.supports_method = function(maybe_self, method, opts)
            local actual_method, actual_opts
            if maybe_self == client then
                actual_method, actual_opts = method, opts
            else
                actual_method, actual_opts = maybe_self, method
            end

            return supports_method(client, actual_method, actual_opts)
        end

        return client
    end
end

-- validate that lazy is available
if not pcall(require, "lazy") then
    -- stylua: ignore
    vim.api.nvim_echo(
        { { ("Unable to load lazy from: %s\n"):format(lazypath), "ErrorMsg" }, { "Press any key to exit...", "MoreMsg" } },
        true, {})
    vim.fn.getchar()
    vim.cmd.quit()
end

require("lazy_setup")
require("polish")
