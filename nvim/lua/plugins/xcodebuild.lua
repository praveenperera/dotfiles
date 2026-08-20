---@type LazySpec
return {
    "wojciech-kulik/xcodebuild.nvim",
    ft = "swift",
    dependencies = { "MunifTanjim/nui.nvim" },
    opts = {
        code_coverage = { enabled = true },
        integrations = {
            telescope_nvim = { enabled = false },
            snacks_nvim = { enabled = true },
        },
    },
}
