local cove = require("config.cove_build")

vim.filetype.add({
    extension = {
        jinja = "jinja",
    },
    filename = {
        Fastfile = "ruby",
    },
    pattern = {
        [".*%.j2"] = function(path, bufnr)
            local source_path = path:sub(1, -4)
            local match_args = { filename = source_path }
            if bufnr >= 0 then
                match_args.buf = bufnr
            end

            local source_filetype = vim.filetype.match(match_args)
            if not source_filetype then
                return "jinja"
            end

            local parser_language = vim.treesitter.language.get_lang(
                source_filetype
            ) or source_filetype
            local template_filetype = "jinja_"
                .. source_filetype:gsub("[^%w_]", "_")
            vim.treesitter.language.register(parser_language, template_filetype)

            return template_filetype
        end,
    },
})

local cove_path = vim.fn.expand("~/code/bitcoinppl/cove/*")
local cove_group = vim.api.nvim_create_augroup("cove-build", { clear = true })

vim.api.nvim_create_autocmd("BufEnter", {
    pattern = cove_path,
    group = cove_group,
    callback = function(args)
        cove.setup_build_commands(args.buf)
    end,
})

vim.api.nvim_create_autocmd("FileType", {
    pattern = { "text", "markdown", "xml" },
    callback = function()
        vim.opt_local.autoindent = false
        vim.opt_local.smartindent = false
        vim.opt_local.cindent = false
        vim.opt_local.indentexpr = ""
        vim.opt_local.expandtab = true
        vim.opt_local.tabstop = 2
        vim.opt_local.shiftwidth = 2
    end,
})

if vim.env.SSH_CONNECTION then
    vim.g.clipboard = "osc52"
end

vim.opt.clipboard:append({ "unnamedplus" })

-- netrw owns remote URI reads; Neo-tree remains the local explorer
vim.g.netrw_localcopydir = "/tmp/netrw"
vim.g.netrw_keepdir = 0
vim.g.netrw_backup = 0
vim.g.netrw_winsize = 20
vim.cmd.packadd("netrw")
