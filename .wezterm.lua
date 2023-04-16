local wezterm = require("wezterm")

wezterm.on("new-tab", function(_window, _pane)
    return false
end)

return {
    window_padding = {
        left = 2,
        right = 2,
        top = 0,
        bottom = 0,
    },
    window_background_opacity = 1.0,
    enable_tab_bar = false,
    enable_wayland = true,
    default_cursor_style = "BlinkingBar",
    font = wezterm.font_with_fallback({
        "OperatorMono Nerd Font",
    }),
    font_size = 12,
    keys = {
        { key = "6",            mods = "CTRL",      action = wezterm.action { SendString = "\x1e" } },
        { key = "d",            mods = "CMD",       action = wezterm.action { SendString = "\x02\x25" } },
        { key = "d",            mods = "CMD|SHIFT", action = wezterm.action { SendString = "\x02\x22" } },

        -- ALT
        { key = "j",            mods = "ALT",       action = wezterm.action { SendString = "\x1bj" } },
        { key = "k",            mods = "ALT",       action = wezterm.action { SendString = "\x1bk" } },
        { key = "h",            mods = "ALT",       action = wezterm.action { SendString = "\x1bh" } },
        { key = "l",            mods = "ALT",       action = wezterm.action { SendString = "\x1bl" } },
        { key = "RightArrow",   mods = "ALT",       action = wezterm.action { SendString = "\x1BF" } },
        { key = "LeftArrow",    mods = "ALT",       action = wezterm.action { SendString = "\x1BB" } },

        -- switch tabs with zellij using tmux mode
        { key = "1",            mods = "CMD",       action = wezterm.action { SendString = "\x021" } },
        { key = "2",            mods = "CMD",       action = wezterm.action { SendString = "\x022" } },
        { key = "3",            mods = "CMD",       action = wezterm.action { SendString = "\x023" } },
        { key = "4",            mods = "CMD",       action = wezterm.action { SendString = "\x024" } },
        { key = "5",            mods = "CMD",       action = wezterm.action { SendString = "\x025" } },
        { key = "6",            mods = "CMD",       action = wezterm.action { SendString = "\x026" } },
        { key = "7",            mods = "CMD",       action = wezterm.action { SendString = "\x027" } },
        { key = "8",            mods = "CMD",       action = wezterm.action { SendString = "\x028" } },
        { key = "9",            mods = "CMD",       action = wezterm.action { SendString = "\x029" } },

        -- command
        { key = "t",            mods = "CMD",       action = wezterm.action { SendString = "\x02c" } },
        { key = "w",            mods = "CMD|SHIFT", action = wezterm.action { SendString = "\x02&" } },
        { key = "w",            mods = "CMD",       action = wezterm.action { SendString = "\x02x" } },
        { key = "UpArrow",      mods = "CMD",       action = wezterm.action { SendString = "\x02\x1B\x5B\x41" } },
        { key = "DownArrow",    mods = "CMD",       action = wezterm.action { SendString = "\x02\x1B\x5B\x42" } },
        { key = "LeftArrow",    mods = "CMD",       action = wezterm.action { SendString = "\x02\x1B\x5B\x44" } },
        { key = "RightArrow",   mods = "CMD",       action = wezterm.action { SendString = "\x02\x1B\x5B\x43" } },
        { key = ",",            mods = "CMD",       action = wezterm.action { SendString = "\x02," } },
        { key = "RightBracket", mods = "CMD|SHIFT", action = wezterm.action { SendString = "\x02n" } }, -- Select next tab
        { key = "LeftBracket",  mods = "CMD|SHIFT", action = wezterm.action { SendString = "\x02p" } }, -- Select previous tab
    },
    -- Colors (converted from praveen.itermcolors)
    colors = {
        foreground = "#c7c7c7",
        background = "#262525",
        cursor_bg = "#c7c7c7",
        cursor_border = "#c7c7c7",
        cursor_fg = "#262525",
        selection_bg = "#c7c7c7",
        selection_fg = "#262525",
        ansi = {
            "#333333",
            "#ef286f",
            "#a7e400",
            "#f1993a",
            "#69d8f2",
            "#c930c7",
            "#00c5c7",
            "#c7c7c7",
        },
        brights = {
            "#676767",
            "#ef286f",
            "#a7e400",
            "#dfdc58",
            "#69d8f2",
            "#c930c7",
            "#5ffdff",
            "#feffff",
        },
    },
}
