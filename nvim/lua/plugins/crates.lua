return {
    "saecki/crates.nvim",
    version = "v0.3.0",
    dependencies = { "nvim-lua/plenary.nvim" },
    event = "BufRead Cargo.toml",
    opts = {
        null_ls = {
            enabled = true,
            name = "crates.nvim",
        },
    },
}
