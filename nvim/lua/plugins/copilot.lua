local opts = {
    suggestion = {
        enabled = true,
        auto_trigger = true,
        keymap = {
            accept = "<C-l>",
            accept_word = "<C-y>",
            accept_line = false,
            next = "<C-s>",
            prev = "<C-a>",
            dismiss = "<C-]>",
        },
    },
}

return {
    "zbirenbaum/copilot.lua",
    cmd = "Copilot",
    event = "InsertEnter",
    opts = opts,
}
