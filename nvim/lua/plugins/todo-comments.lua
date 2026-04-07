return {
    "folke/todo-comments.nvim",
    opts = {},
    event = "BufRead",
    cmd = {
        "TodoQuickFix",
        "TodoLocList",
        "TodoTrouble",
    },
}
