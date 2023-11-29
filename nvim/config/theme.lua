local M = {}

local function config(_, _)
    return {
        palette = "astrodark",
        highlights = {
            global = {
                modify_hl_groups = function(hl, _c)
                    hl.SpelunkerSpellBad = { fg = "NONE", bg = "NONE" }
                end,
            },
        },
    }
end

M.config = config
return M
