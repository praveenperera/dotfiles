function NeotreeConfig(config)
  config.filesystem = {
    filtered_items = {
      visible = true,
      hide_dotfiles = false,
      hide_gitignored = true,
      hide_by_pattern = {
        ".git",
      },
      never_show = {
        ".DS_Store",
      },
    }
  }

  return config
end
