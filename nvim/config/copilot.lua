local M = {}


M.config = function(_, _)
    local opts = {
        suggestion = {
            enabled = true,
            auto_trigger = true,
            keymap = {
                accept = "<C-l>",
                accept_word = false,
                accept_line = false,
                next = "<C-s>",
                prev = "<C-a>",
                dismiss = "<C-]>",
            },
        }
    }

    return opts
end

return M
