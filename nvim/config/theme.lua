local M = {}

local function config(_, _)
    return {
        palette = "astrodark",
        highlights = {
            global = {
                modify_hl_groups = function(hl, _c)
                    hl.NeogitDiffDeleteHighlight = { fg = "#292929", bg = "#f77977" }
                    hl.NeogitDiffAddHighlight = { fg = "#292929" }
                    hl.NeogitDiffDelete = { fg = "#292929", bg = "#f77977" }
                    hl.SpelunkerSpellBad = { fg = "NONE", bg = "NONE" }
                end,
            },
        },
    }
end

M.config = config
return M
