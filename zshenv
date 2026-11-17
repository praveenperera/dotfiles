. "$HOME/.cargo/env"

# keep Homebrew tools available to non-login shells such as remote SSH commands
export PATH="/opt/homebrew/bin:/opt/homebrew/sbin:$PATH"

# gpu-ab looks up `--adapter <repo>` here; live adapters stay in private-workbench
export GPU_AB_ADAPTERS="$HOME/code/private-workbench/ops/{name}/hill-climb/gpu-ab.toml"
