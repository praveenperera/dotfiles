local bufnr = vim.api.nvim_get_current_buf()

-- lint
vim.keymap.set("n", "<leader>ll", function()
    require("lint").try_lint()
end, { desc = "Lint file", silent = true, buffer = bufnr })

-- format
local conform = require("conform")
vim.keymap.set({ "n", "v" }, "<leader>lf", function()
    conform.format({
        lsp_fallback = true,
        async = false,
        timeout_ms = 500,
    })
end, { desc = "Format file or range (in visual mode)" })

-- xcodebuild
vim.keymap.set(
    "n",
    "<leader>X",
    "<cmd>XcodebuildPicker<cr>",
    { desc = "Show All Xcodebuild Actions" }
)
vim.keymap.set(
    "n",
    "<leader>xl",
    "<cmd>XcodebuildToggleLogs<cr>",
    { desc = "Toggle Xcodebuild Logs" }
)
vim.keymap.set(
    "n",
    "<leader>xb",
    "<cmd>XcodebuildBuild<cr>",
    { desc = "Build Project" }
)
vim.keymap.set(
    "n",
    "<leader>xr",
    "<cmd>XcodebuildBuildRun<cr>",
    { desc = "Build & Run Project" }
)
vim.keymap.set(
    "n",
    "<leader>xt",
    "<cmd>XcodebuildTest<cr>",
    { desc = "Run Tests" }
)
vim.keymap.set(
    "n",
    "<leader>xT",
    "<cmd>XcodebuildTestClass<cr>",
    { desc = "Run This Test Class" }
)
vim.keymap.set(
    "n",
    "<leader>xd",
    "<cmd>XcodebuildSelectDevice<cr>",
    { desc = "Select Device" }
)
vim.keymap.set(
    "n",
    "<leader>xp",
    "<cmd>XcodebuildSelectTestPlan<cr>",
    { desc = "Select Test Plan" }
)
vim.keymap.set(
    "n",
    "<leader>xc",
    "<cmd>XcodebuildToggleCodeCoverage<cr>",
    { desc = "Toggle Code Coverage" }
)
vim.keymap.set(
    "n",
    "<leader>xC",
    "<cmd>XcodebuildShowCodeCoverageReport<cr>",
    { desc = "Show Code Coverage Report" }
)
vim.keymap.set(
    "n",
    "<leaer>xq",
    "<cmd>Telescope quickfix<cr>",
    { desc = "Show QuickFix List" }
)
