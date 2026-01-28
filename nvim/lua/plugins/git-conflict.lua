return {
    "akinsho/git-conflict.nvim",
    version = "*",
    event = "BufReadPre",
    opts = {
        default_mappings = false,
        default_commands = true,
        disable_diagnostics = false,
        highlights = {
            incoming = "DiffAdd",
            current = "DiffText",
        },
    },
    keys = {
        { "ch", "<cmd>GitConflictChooseOurs<cr>", desc = "Choose left (ours)" },
        { "cl", "<cmd>GitConflictChooseTheirs<cr>", desc = "Choose right (theirs)" },
        { "cb", "<cmd>GitConflictChooseBoth<cr>", desc = "Choose both" },
        { "cn", "<cmd>GitConflictChooseNone<cr>", desc = "Choose none" },
        { "vl", "<cmd>GitConflictNextConflict<cr>", desc = "Next conflict" },
        { "vh", "<cmd>GitConflictPrevConflict<cr>", desc = "Previous conflict" },
    },
}
