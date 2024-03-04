local telescope = require("user.plugins.telescope")

return {
    n = {
        -- Harpoon
        ["<leader>h"] = { name = "Harpoon" },
        ["<leader>ha"] = {
            function()
                require("harpoon.mark").add_file()
            end,
            desc = "Add file",
        },
        ["<leader>he"] = {
            function()
                require("harpoon.ui").toggle_quick_menu()
            end,
            desc = "Toggle quick menu",
        },
        ["<leader>h1"] = {
            function()
                require("harpoon.ui").nav_file(1)
            end,
            desc = "Go to file 1",
        },
        ["<leader>h2"] = {
            function()
                require("harpoon.ui").nav_file(2)
            end,
            desc = "Go to file 2",
        },
        ["<leader>h3"] = {
            function()
                require("harpoon.ui").nav_file(3)
            end,
            desc = "Go to file 3",
        },
        ["<leader>h4"] = {
            function()
                require("harpoon.ui").nav_file(4)
            end,
            desc = "Go to file 4",
        },
        ["<leader>h5"] = {
            function()
                require("harpoon.ui").nav_file(5)
            end,
            desc = "Go to file 5",
        },
        ["<leader>h6"] = {
            function()
                require("harpoon.ui").nav_file(6)
            end,
            desc = "Go to file 6",
        },
        ["g1"] = {
            function()
                require("harpoon.ui").nav_file(1)
            end,
            desc = "Go to file 1",
        },
        ["g2"] = {
            function()
                require("harpoon.ui").nav_file(2)
            end,
            desc = "Go to file 2",
        },
        ["g3"] = {
            function()
                require("harpoon.ui").nav_file(3)
            end,
            desc = "Go to file 3",
        },
        ["g4"] = {
            function()
                require("harpoon.ui").nav_file(4)
            end,
            desc = "Go to file 4",
        },
        ["g5"] = {
            function()
                require("harpoon.ui").nav_file(5)
            end,
            desc = "Go to file 5",
        },
        ["g6"] = {
            function()
                require("harpoon.ui").nav_file(6)
            end,
            desc = "Go to file 6",
        },
        ["<leader>uc"] = {
            function()
                vim.cmd("TSContextToggle")
            end,
            desc = "Toggle TS Context",
        },
        -- Save All
        ["<leader>W"] = {
            function()
                vim.cmd("wa")
            end,
            desc = "Save all",
        },
        -- quick save
        ["<C-s>"] = { "<cmd>w!<cr>", desc = "Save File" },
        -- window navigation
        ["<leader>1"] = { "1<C-w>w", desc = "Go to window 1" },
        ["<leader>2"] = { "2<C-w>w", desc = "Go to window 2" },
        ["<leader>3"] = { "3<C-w>w", desc = "Go to window 3" },
        ["<leader>4"] = { "4<C-w>w", desc = "Go to window 4" },
        -- git
        ["<leader>gs"] = { "<cmd>Neogit <CR>", desc = "Git status" },
        -- move
        ["<C-d>"] = { "<C-d>zz", desc = "Half page up" },
        ["<C-u>"] = { "<C-u>zz", desc = "Half page down" },
        -- random
        ["U"] = { "<cmd>redo<cr>" },
        ["J"] = { "mzJ`z" },
        ["n"] = { "nzzzv" },
        ["N"] = { "Nzzzv" },
        -- toggleterm
        ["<leader>th"] = {
            "<cmd>ToggleTerm direction=horizontal<cr>",
            desc = "Toggle horizontal terminal",
        },
        ["<leader>tv"] = {
            "<cmd>ToggleTerm direction=vertical<cr>",
            desc = "Toggle vertical terminal",
        },
        ["<leader>t1"] = {
            "<cmd>ToggleTerm 1<cr>",
            desc = "ToggleTerm 1st Window",
        },
        ["<leader>t2"] = {
            "<cmd>ToggleTerm 2<cr>",
            desc = "ToggleTerm 2nd Window",
        },
        ["<leader>t3"] = {
            "<cmd>ToggleTerm 3<cr>",
            desc = "ToggleTerm 3rd Window",
        },
        ["<leader>t4"] = {
            "<cmd>ToggleTerm 4<cr>",
            desc = "ToggleTerm 4th Window",
        },
        -- system yank
        ["<leader>d"] = { [["_d]], desc = "Delete without register" },
        ["<leader>Y"] = { [["+Y]], desc = "Yank to system" },
        ["<leader>y"] = { [["+y]], desc = "Yank to system" },
        -- resize
        ["<C-Home>"] = { "<C-w>+", desc = "Resize up" },
        ["<C-End>"] = { "<C-w>-", desc = "Resize down" },
        -- find
        ["<leader>fi"] = {
            "<cmd>Telescope current_buffer_fuzzy_find case_mode=ignore_case<CR>",
            desc = "Find in Buffer",
        },
        ["<leader>ft"] = { "<cmd>TodoTelescope<CR>", desc = "Find TODOs" },
        ["<leader>ff"] = { telescope.find_files, desc = "Find all files" },
        ["<leader>fs"] = {
            function()
                require("telescope").extensions.aerial.aerial()
            end,
            desc = "Search document symbols",
        },
        ["<leader>fS"] = {
            function()
                require("telescope.builtin").lsp_dynamic_workspace_symbols()
            end,
            desc = "Search project symbols",
        },
        --
        ["<leader>-"] = {
            function()
                require("oil").open()
            end,
            desc = "Open parent directory",
        },
        -- undo
        ["<leader>U"] = { vim.cmd.UndotreeToggle, desc = "Undo tree" },
        -- hardmode (no arrows)
        ["<Up>"] = { "<nop>" },
        ["<Down>"] = { "<nop>" },
        ["<Left>"] = { "<nop>" },
        ["<Right>"] = { "<nop>" },
    },
    x = {
        ["<leader>p"] = { [["_dP]], desc = "Paste from system" },
    },
    t = {
        ["<esc><esc>"] = { [[<C-\><C-n>]], desc = "Normal mode" },
    },
    v = {
        ["<leader>d"] = { [["_d]], desc = "Delete without register" },
        -- move lines up and down like option arrows
        ["K"] = { ":m '<-2<CR>gv=gv", desc = "Move selection up" },
        ["J"] = { ":m '>+1<CR>gv=gv", desc = "Move selection down" },
    },
}
