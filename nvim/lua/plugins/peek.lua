-- markdown preview
return {
    "praveenperera/peek.nvim",
    event = { "VeryLazy" },
    build = "deno task --quiet build:fast",
    config = function()
        require("peek").setup({
            theme = "light",
            auto_os_theme = true,
        })
        vim.api.nvim_create_user_command("PeekOpen", require("peek").open, {})
        vim.api.nvim_create_user_command("PeekOpenDark", function(bufnr)
            require("peek").open(
                bufnr,
                { theme = "dark", auto_os_theme = false }
            )
        end, {})
        vim.api.nvim_create_user_command("PeekOpenLight", function(bufnr)
            require("peek").open(
                bufnr,
                { theme = "light", auto_os_theme = false }
            )
        end, {})
        vim.api.nvim_create_user_command("PeekClose", require("peek").close, {})
    end,
}
