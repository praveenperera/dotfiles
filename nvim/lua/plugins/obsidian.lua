return {
    "epwalsh/obsidian.nvim",
    version = "*",
    lazy = true,
    event = {
        "BufReadPre " .. vim.fn.expand("~") .. "/code/logseq_local/main/**.md",
        "BufNewFile " .. vim.fn.expand("~") .. "/code/logseq_local/main/**.md",
    },
    dependencies = {
        "nvim-lua/plenary.nvim",
    },
    opts = {
        workspaces = {
            {
                name = "main",
                path = "~/code/logseq_local/main",
                overrides = {
                    notes_subdir = "pages",
                },
            },
        },

        mappings = {
            -- Toggle check-boxes.
            ["<leader>ch"] = {
                action = function()
                    return require("obsidian").util.toggle_checkbox()
                end,
                opts = { buffer = true },
            },
        },

        daily_notes = {
            folder = "journals",
            date_format = "%Y_%m_%d",
            alias_format = "%B %-d, %Y",
            template = nil,
        },
    },

    config = function(_, opts)
        require("obsidian").setup(opts)

        vim.keymap.set("n", "<space>od", "<cmd>ObsidianDailies<CR>", {
            silent = true,
            noremap = true,
            desc = "[O]bsidian [D]aily",
        })
        vim.keymap.set("n", "<space>oy", "<cmd>ObsidianYesterday<CR>", {
            silent = true,
            noremap = true,
            desc = "[O]bsidian [Y]esterday",
        })

        vim.keymap.set("n", "<space>ot", "<cmd>ObsidianToday<CR>", {
            silent = true,
            noremap = true,
            desc = "[O]bsidian [T]oday",
        })

        vim.keymap.set("n", "<space>os", ":ObsidianQuickSwitch<CR>", {
            silent = true,
            noremap = true,
            desc = "[O]bsidian Quick[S]witch",
        })
    end,
}
