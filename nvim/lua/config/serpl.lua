local M = {}

local Terminal = require("toggleterm.terminal").Terminal

local serpl_opts = {
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
}

-- Function to toggle the custom terminal
local function serpl_project()
    local opts = serpl_opts
    local serpl = Terminal:new(opts)
    serpl:toggle()
end

local function serpl_file()
    local current_file = vim.fn.expand("%:p")

    local opts = serpl_opts
    opts.cmd = "serpl --project-root " .. current_file

    local serpl = Terminal:new(opts)
    serpl:toggle()
end

local function serpl_dir()
    local current_file_dir = vim.fn.expand("%:p:h")

    local opts = serpl_opts
    opts.cmd = "serpl --project-root " .. current_file_dir

    local serpl = Terminal:new(opts)
    serpl:toggle()
end

M.serpl_project = serpl_project
M.serpl_file = serpl_file
M.serpl_dir = serpl_dir

return M
