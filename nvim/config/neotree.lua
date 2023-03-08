function NeotreeConfig(config)
  config.filesystem = {
    filtered_items = {
      visible = true,
      hide_dotfiles = false,
      hide_gitignored = true,
    }
  }

  return config
end
