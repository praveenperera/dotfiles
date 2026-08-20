require("lazy").setup({
    {
        "AstroNvim/AstroNvim",
        version = "^6",
        import = "astronvim.plugins",
        opts = {
            mapleader = " ",
            maplocalleader = ",",
            icons_enabled = true,
            pin_plugins = nil,
            update_notifications = true,
        },
    },
    { import = "community" },
    { import = "plugins" },
} --[[@as LazySpec]], {
    -- Configure any other `lazy.nvim` configuration options here
    install = { colorscheme = { "astrodark", "habamax" } },
    rocks = { enabled = false },
    ui = { backdrop = 100 },
    performance = {
        rtp = {
            -- disable some rtp plugins, add more to your liking
            disabled_plugins = {
                "gzip",
                "tarPlugin",
                "tohtml",
                "zipPlugin",
            },
        },
    },
} --[[@as LazyConfig]])
