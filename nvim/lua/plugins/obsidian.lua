local vault = vim.fn.expand("~/code/logseq_local/main")

---@type LazySpec
return {
    "obsidian-nvim/obsidian.nvim",
    version = "*",
    cmd = "Obsidian",
    event = {
        "BufReadPre " .. vault .. "/**.md",
        "BufNewFile " .. vault .. "/**.md",
    },
    keys = {
        {
            "<Leader>od",
            "<Cmd>Obsidian dailies<CR>",
            desc = "Obsidian dailies",
        },
        {
            "<Leader>oy",
            "<Cmd>Obsidian yesterday<CR>",
            desc = "Obsidian yesterday",
        },
        {
            "<Leader>ot",
            "<Cmd>Obsidian today<CR>",
            desc = "Obsidian today",
        },
        {
            "<Leader>os",
            "<Cmd>Obsidian quick_switch<CR>",
            desc = "Obsidian quick switch",
        },
        {
            "<Leader>ch",
            "<Cmd>Obsidian toggle_checkbox<CR>",
            mode = { "n", "x" },
            desc = "Toggle checkbox",
        },
    },
    opts = {
        legacy_commands = false,
        picker = { name = "snacks.picker" },
        workspaces = {
            {
                name = "main",
                path = vault,
                overrides = { notes_subdir = "pages" },
            },
        },
        daily_notes = {
            folder = "journals",
            date_format = "%Y_%m_%d",
            alias_format = "%B %-d, %Y",
            template = nil,
            workdays_only = false,
        },
    },
}
