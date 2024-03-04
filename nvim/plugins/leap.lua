return {
    "ggandor/leap.nvim",
    event = "BufRead",
    config = function()
        require("leap").add_default_mappings()
    end,
}
