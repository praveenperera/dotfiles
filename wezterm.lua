local wezterm = require("wezterm")

return {
    window_padding = {
        left = 2,
        right = 2,
        top = 0,
        bottom = 0,
    },
    window_background_opacity = 1.0,
    enable_tab_bar = true,
    enable_wayland = true,
    default_cursor_style = "BlinkingBar",
    font = wezterm.font_with_fallback({
        "JetBrains Mono",
        { family = "Symbols Nerd Font", scale = 0.75 }
    }),
    font_size = 11.5,
    keys = {
        { key = "T", mods = "CMD",       action = wezterm.action { SpawnTab = "CurrentPaneDomain" } },
        { key = 'd', mods = 'CMD|SHIFT', action = wezterm.action.SplitVertical { domain = 'CurrentPaneDomain' } },
        { key = 'd', mods = 'CMD',       action = wezterm.action.SplitHorizontal { domain = 'CurrentPaneDomain' } },
        {
            key = ',',
            mods = 'CMD',
            action = wezterm.action.PromptInputLine {
                description = 'Enter new name for tab',
                action = wezterm.action_callback(function(window, pane, line)
                    if line then
                        window:active_tab():set_title(line)
                    end
                end),
            },
        },
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
