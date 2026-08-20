local function config(_, opts)
    opts.filesystem = opts.filesystem or {}
    opts.filesystem.filtered_items =
        vim.tbl_deep_extend("force", opts.filesystem.filtered_items or {}, {
            visible = true,
            hide_dotfiles = false,
            hide_gitignored = true,
            hide_by_pattern = { ".git" },
            never_show = { ".DS_Store" },
        })
end

return {
    "nvim-neo-tree/neo-tree.nvim",
    opts = config,
}
