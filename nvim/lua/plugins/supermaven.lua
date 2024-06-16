return {
    "supermaven-inc/supermaven-nvim",
    config = function()
        require("supermaven-nvim").setup({
            keymaps = {
                accept_word = "<C-k>",
                accept_suggestion = "<C-l>",
                clear_suggestion = "<C-]>",
            },
        })
    end,
}
