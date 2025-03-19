return {
    "supermaven-inc/supermaven-nvim",
    config = function()
        require("supermaven-nvim").setup({
            -- log_level = "off",
            keymaps = {
                accept_word = "<C-y>",
                accept_suggestion = "<C-l>",
                clear_suggestion = "<C-]>",
                next_suggestion = "<C-j>",
            },
        })
    end,
}
-- return {}
