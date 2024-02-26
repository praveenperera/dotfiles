local bufnr = vim.api.nvim_get_current_buf()

vim.keymap.set(
    "n",
    "<leader>a",
    function()
        vim.cmd.RustLsp('codeAction')
    end,
    { silent = true, buffer = bufnr, desc = "Rust Code Action" }
)

vim.keymap.set(
    "n",
    "<leader>lC",
    function()
        vim.cmd.RustLsp('openCargo')
    end,
    { silent = true, buffer = bufnr, desc = "Open Cargo.toml" }
)


vim.keymap.set(
    "n",
    "<leader>lc",
    function()
        vim.cmd.RustLsp('externalDocs')
    end,
    { silent = true, buffer = bufnr, desc = "Open docs.rs" }
)
