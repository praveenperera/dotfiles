---@type LazySpec
return {
    "gregorias/coerce.nvim",
    event = "VeryLazy",
    keys = {
        {
            "ga",
            "<Plug>(coerce-normal)",
            mode = "n",
            desc = "Coerce word",
        },
        {
            "gA",
            "<Plug>(coerce-motion)",
            mode = "n",
            desc = "Coerce motion",
        },
        {
            "ga",
            "<Plug>(coerce-visual)",
            mode = "x",
            desc = "Coerce selection",
        },
    },
    opts = {},
}
