local bufnr = vim.api.nvim_get_current_buf()

local function map(mode, key, command, description)
    vim.keymap.set(mode, key, command, {
        buffer = bufnr,
        desc = description,
        silent = true,
    })
end

map("n", "<Leader>ll", function()
    require("lint").try_lint()
end, "Lint file")

local xcodebuild_mappings = {
    ["<Leader>X"] = { "XcodebuildPicker", "Show all Xcodebuild actions" },
    ["<Leader>xl"] = { "XcodebuildToggleLogs", "Toggle Xcodebuild logs" },
    ["<Leader>xb"] = { "XcodebuildBuild", "Build project" },
    ["<Leader>xr"] = { "XcodebuildBuildRun", "Build and run project" },
    ["<Leader>xt"] = { "XcodebuildTest", "Run tests" },
    ["<Leader>xT"] = { "XcodebuildTestClass", "Run this test class" },
    ["<Leader>xd"] = { "XcodebuildSelectDevice", "Select device" },
    ["<Leader>xp"] = { "XcodebuildSelectTestPlan", "Select test plan" },
    ["<Leader>xc"] = {
        "XcodebuildToggleCodeCoverage",
        "Toggle code coverage",
    },
    ["<Leader>xC"] = {
        "XcodebuildShowCodeCoverageReport",
        "Show code coverage report",
    },
}

for key, mapping in pairs(xcodebuild_mappings) do
    map("n", key, "<Cmd>" .. mapping[1] .. "<CR>", mapping[2])
end

map("n", "<Leader>xq", function()
    require("snacks").picker.qflist()
end, "Show quickfix list")
map("n", "<Leader>lx", function()
    vim.notify("Restarting LSP")
    vim.cmd("lsp restart")
end, "Restart LSP")
