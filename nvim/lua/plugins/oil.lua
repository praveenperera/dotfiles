---@type LazySpec
return {
    "stevearc/oil.nvim",
    keys = {
        { "-", "<Cmd>Oil<CR>", desc = "Open parent directory" },
        {
            "<Leader>-",
            function()
                require("oil").toggle_float()
            end,
            desc = "Open parent directory in a float",
        },
    },
    opts = {
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
        view_options = { show_hidden = true },
    },
}
