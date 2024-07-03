local M = {}

local Terminal = require("toggleterm.terminal").Terminal

-- Create a new terminal with the custom command
local serpl = Terminal:new({
    cmd = "serpl",
    direction = "float",
    float_opts = {
        border = "curved",
    },
    on_open = function(term)
        vim.keymap.set(
            "n",
            "q",
            "<cmd>close<CR>",
            { buffer = term.bufnr, noremap = true, silent = true }
        )
    end,
})

-- Function to toggle the custom terminal
function serpl_toggle()
    serpl:toggle()
end

M.serpl_toggle = serpl_toggle

return M
