return {
    {
        "stevearc/oil.nvim",
        dependencies = { "nvim-tree/nvim-web-devicons" },
        config = function()
            require("oil").setup({
                default_file_explorer = false,
                columns = { "icon" },
                keymaps = {
                    ["<C-h>"] = false,
                    ["<M-h>"] = "actions.select_split",
                    ["-"] = "actions.parent",
                    ["_"] = "actions.open_cwd",
                    ["`"] = "actions.cd",
                    ["~"] = "actions.tcd",
                    ["gx"] = "actions.open_external",
                    ["g."] = "actions.toggle_hidden",
                    ["g?"] = "actions.show_help",
                },
                delete_to_trash = true,
                view_options = {
                    show_hidden = true,
                },
            })

            -- Open parent directory in current window
            vim.keymap.set(
                "n",
                "-",
                "<CMD>Oil<CR>",
                { desc = "Open parent directory" }
            )

            -- Open parent directory in floating window
            vim.keymap.set("n", "<Leader>-", require("oil").toggle_float)
        end,
    },
}
