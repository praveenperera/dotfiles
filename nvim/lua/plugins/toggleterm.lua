local function config(_, _)
    local function size(term)
        if term.direction == "horizontal" then
            return 15
        elseif term.direction == "vertical" then
            return vim.o.columns * 0.4
        end
    end

    return {
        size = size,
        persist_size = true,
        start_in_insert = true,
    }
end

return {
    "akinsho/toggleterm.nvim",
    version = "*",
    opts = config,
    cmd = {
        "ToggleTerm",
        "ToggleTermToggleAll",
    },
}
