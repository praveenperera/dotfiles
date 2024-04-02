local opts = {
    palette = "astrodark",
    highlights = {
        global = {
            modify_hl_groups = function(hl, _c)
                hl.SpelunkerSpellBad = { fg = "NONE", bg = "NONE" }
            end,
        },
    },
}

return { "AstroNvim/astrotheme", opts = opts }
