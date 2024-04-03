local maps = {
    -- Harpoon
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
}

-- set up key maps
for key, val in pairs(maps) do
    if type(val) == "table" and val[1] then
        vim.keymap.set("n", key, val[1], { desc = val.desc, silent = true })
    end
end
