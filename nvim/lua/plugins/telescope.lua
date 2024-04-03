local opts = {}

opts.find_files = function()
    require("telescope.builtin").find_files({
        find_command = { "fd", "--exclude", ".git", "--hidden" },
    })
end


return { "nvim-telescope/telescope.nvim", opts = opts }
