return {
    "folke/todo-comments.nvim",
    dependencies = "nvim-lua/plenary.nvim",
    opts = {},
    event = "BufRead",
    cmd = { "TodoQuickFix", "TodoLocList",
        "TodoTrouble",
        "TodoTelescope",
    },
}
