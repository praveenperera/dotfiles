local opts = {
    autotag = { enable = true },
    highlight = { enable = true },
    indent = {
        enable = true,
        disable = { "yaml" },
    },
    additional_vim_regex_highlighting = false,
    auto_install = true,
    ensure_installed = {
        "lua",
        "vim",
    },
}

return { "nvim-treesitter/nvim-treesitter", opts = opts }
