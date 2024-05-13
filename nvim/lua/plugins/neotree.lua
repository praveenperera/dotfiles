local function config(_, opts)
    opts.filesystem.filtered_items = {
        visible = true,
        hide_dotfiles = false,
        hide_gitignored = true,
        hide_by_pattern = {
            ".git",
        },
        never_show = {
            ".DS_Store",
        },
    }
    return opts
end

return {
    "nvim-neo-tree/neo-tree.nvim",
    version = "v2.x",
    dependencies = {
        "nvim-lua/plenary.nvim",
        "nvim-tree/nvim-web-devicons",
        "MunifTanjim/nui.nvim",
    },
    opts = config,
}
