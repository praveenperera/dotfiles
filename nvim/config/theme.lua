local M = {}

local function config(_, _)
    return {
        palette = "astrodark",
        highlights = {
            global = {
                ["NeogitDiffDeleteHighlight"] = {
                    fg = "#292929",
                    bg = "#f77977",
                },
                ["NeogitDiffAddHighlight"] = {
                    fg = "#292929",
                },
                ["NeogitDiffDelete"] = { fg = "#292929", bg = "#f77977" },
                ["SpelunkerSpellBad"] = { fg = "NONE", bg = "NONE" },
            },
        },
    }
end

M.config = config
return M
