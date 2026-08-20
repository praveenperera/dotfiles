local function harpoon_list()
    return require("harpoon"):list()
end

local function select_file(index)
    return function()
        harpoon_list():select(index)
    end
end

local keys = {
    {
        "<Leader>ha",
        function()
            harpoon_list():add()
        end,
        desc = "Add file",
    },
    {
        "<Leader>he",
        function()
            local harpoon = require("harpoon")
            harpoon.ui:toggle_quick_menu(harpoon:list())
        end,
        desc = "Toggle quick menu",
    },
}

for index = 1, 6 do
    table.insert(keys, {
        "<Leader>h" .. index,
        select_file(index),
        desc = "Go to file " .. index,
    })
    table.insert(keys, {
        "g" .. index,
        select_file(index),
        desc = "Go to file " .. index,
    })
end

---@type LazySpec
return {
    "ThePrimeagen/harpoon",
    branch = "harpoon2",
    dependencies = { "nvim-lua/plenary.nvim" },
    keys = keys,
    opts = {},
    config = function(_, opts)
        require("harpoon"):setup(opts)
    end,
}
