local status = require("astronvim.utils.status")

local opts = {}

opts.statusline = {
    hl = { fg = "fg", bg = "bg" },
    status.component.mode({
        mode_text = { padding = { left = 1, right = 1 } },
    }), -- add the mode text
    status.component.git_branch(),
    status.component.file_info({
        unique_path = {},
    }),
    status.component.cmd_info(),
    status.component.git_diff(),
    status.component.diagnostics(),
    status.component.fill(),
    status.component.cmd_info(),
    status.component.fill(),
    status.component.lsp(),
    status.component.treesitter(),
    status.component.nav(),
}

return { "rebelot/heirline.nvim", opts = opts }
