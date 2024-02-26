local bufnr = vim.api.nvim_get_current_buf()

vim.keymap.set(
    "n",
    "<leader>lz",
    function()
        vim.cmd.RustLsp('openCargo')
    end,
    { silent = true, buffer = bufnr, desc = "Open Cargo.toml" }
)
