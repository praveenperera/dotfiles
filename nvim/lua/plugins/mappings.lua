local telescope = require("telescope")

-- Mappings can be configured through AstroCore as well.
-- NOTE: keycodes follow the casing in the vimdocs. For example, `<Leader>` must be capitalized
return {
    {
        "AstroNvim/astrocore",
        ---@type AstroCoreOpts
        opts = {
            mappings = {
                n = {
                    -- Overwrite astronvim leader h, the rest is in harpoon.lua
                    ["<Leader>h"] = { name = "Harpoon" },

                    -- Overwrite astronvim leader uc
                    ["<Leader>uc"] = {
                        function()
                            vim.cmd("TSContextToggle")
                        end,
                        desc = "Toggle TS Context",
                    },

                    -- Save All
                    ["<Leader>W"] = {
                        function()
                            vim.cmd("wa")
                        end,
                        desc = "Save all",
                    },

                    -- Replace word
                    ["<Leader>rw"] = {
                        [[:%s/\<<C-r><C-w>\>/<C-r><C-w>/gI<Left><Left><Left>]],
                        desc = "Replace word under cursor",
                    },

                    -- Replace word with confirmation, in quickfix
                    ["<Leader>rc"] = {
                        [[:cdo %s/\<<C-r><C-w>\>/<C-r><C-w>/gc<Left><Left><Left>]],
                        desc = "CDO Replace word under cursor",
                    },

                    -- Close quickfix
                    ["<Leader>rq"] = {
                        [[:cclose<CR>:lclose<CR>]],
                        desc = "Close quickfix menu",
                    },

                    -- quickfix
                    ["<Leader>q"] = { "<nop>", desc = "Quickfix" },
                    ["<Leader>qo"] = { ":copen<CR>", desc = "Open Quickfix" },
                    ["<Leader>qq"] = { ":cclose<CR>", desc = "Close Quickfix" },
                    ["<Leader>qn"] = { ":cnext<CR>", desc = "Next Quickfix" },
                    ["<Leader>qp"] = {
                        ":cprev<CR>",
                        desc = "Previous Quickfix",
                    },

                    ["<Leader>ql"] = {
                        ":lopen<CR>",
                        desc = "Open Location List",
                    },
                    ["<Leader>qL"] = {
                        ":lclose<CR>",
                        desc = "Close Location List",
                    },
                    ["<Leader>qN"] = {
                        ":lnext<CR>",
                        desc = "Next Location List",
                    },
                    ["<Leader>qP"] = {
                        ":lprev<CR>",
                        desc = "Previous Location List",
                    },

                    -- window navigation
                    ["<Leader>1"] = { "1<C-w>w", desc = "Go to window 1" },
                    ["<Leader>2"] = { "2<C-w>w", desc = "Go to window 2" },
                    ["<Leader>3"] = { "3<C-w>w", desc = "Go to window 3" },
                    ["<Leader>4"] = { "4<C-w>w", desc = "Go to window 4" },

                    -- git
                    ["<Leader>gs"] = { "<cmd>Neogit <CR>", desc = "Git status" },

                    -- move
                    ["<C-d>"] = { "<C-d>zz", desc = "Half page up" },
                    ["<C-u>"] = { "<C-u>zz", desc = "Half page down" },

                    -- random
                    ["U"] = { "<cmd>redo<cr>" },
                    ["J"] = { "mzJ`z" },
                    ["n"] = { "nzzzv" },
                    ["N"] = { "Nzzzv" },

                    -- toggleterm
                    ["<Leader>th"] = {
                        "<cmd>ToggleTerm direction=horizontal<cr>",
                        desc = "Toggle horizontal terminal",
                    },
                    ["<Leader>tv"] = {
                        "<cmd>ToggleTerm direction=vertical<cr>",
                        desc = "Toggle vertical terminal",
                    },
                    ["<Leader>t1"] = {
                        "<cmd>ToggleTerm 1<cr>",
                        desc = "ToggleTerm 1st Window",
                    },
                    ["<Leader>t2"] = {
                        "<cmd>ToggleTerm 2<cr>",
                        desc = "ToggleTerm 2nd Window",
                    },
                    ["<Leader>t3"] = {
                        "<cmd>ToggleTerm 3<cr>",
                        desc = "ToggleTerm 3rd Window",
                    },
                    ["<Leader>t4"] = {
                        "<cmd>ToggleTerm 4<cr>",
                        desc = "ToggleTerm 4th Window",
                    },

                    -- system yank
                    ["<Leader>d"] = {
                        [["_d]],
                        desc = "Delete without register",
                    },
                    ["<Leader>Y"] = { [["+Y]], desc = "Yank to system" },
                    ["<Leader>y"] = { [["+y]], desc = "Yank to system" },

                    -- resize
                    ["<C-Home>"] = { "<C-w>+", desc = "Resize up" },
                    ["<C-End>"] = { "<C-w>-", desc = "Resize down" },

                    -- find
                    ["<Leader>fi"] = {
                        "<cmd>Telescope current_buffer_fuzzy_find case_mode=ignore_case<CR>",
                        desc = "Find in Buffer",
                    },

                    ["<Leader>ft"] = {
                        "<cmd>TodoTelescope<CR>",
                        desc = "Find TODOs",
                    },
                    ["<Leader>ff"] = {
                        telescope.find_files,
                        desc = "Find all files",
                    },
                    ["<Leader>fs"] = {
                        function()
                            require("telescope").extensions.aerial.aerial()
                        end,
                        desc = "Search document symbols",
                    },
                    ["<Leader>fS"] = {
                        function()
                            require("telescope.builtin").lsp_dynamic_workspace_symbols()
                        end,
                        desc = "Search project symbols",
                    },

                    -- Split resizing
                    ["<M-h>"] = { "<C-w><", desc = "Resize left" },
                    ["<M-l>"] = { "<C-w>>", desc = "Resize right" },
                    ["<M-j>"] = { "<C-w>-", desc = "Resize down" },
                    ["<M-k>"] = { "<C-w>+", desc = "Resize up" },

                    -- source file
                    ["<Leader>so"] = {
                        "<cmd>source %<CR>",
                        desc = "Source file",
                    },

                    -- undo
                    ["<Leader>U"] = {
                        vim.cmd.UndotreeToggle,
                        desc = "Undo tree",
                    },

                    -- hardmode (no arrows)
                    ["<Up>"] = { "<nop>" },
                    ["<Down>"] = { "<nop>" },
                    ["<Left>"] = { "<nop>" },
                    ["<Right>"] = { "<nop>" },
                },
                x = {
                    ["<Leader>p"] = { [["_dP]], desc = "Paste from system" },
                },
                t = {
                    ["<esc><esc>"] = { [[<C-\><C-n>]], desc = "Normal mode" },
                },
                v = {
                    ["<Leader>d"] = {
                        [["_d]],
                        desc = "Delete without register",
                    },
                    -- move lines up and down like option arrows
                    ["K"] = { ":m '<-2<CR>gv=gv", desc = "Move selection up" },
                    ["J"] = { ":m '>+1<CR>gv=gv", desc = "Move selection down" },
                },
            },
        },
    },
}
