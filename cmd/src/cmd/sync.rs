use std::path::{Component, Path, PathBuf};

use clap::Subcommand;
use colored::Colorize;
use eyre::{eyre, Result};
use log::info;
use xshell::{cmd, Shell};

use crate::cmd::memory;
use crate::fsutil;

#[derive(Debug, Clone)]
pub struct Sync {
    pub subcommand: SyncCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SyncCmd {
    /// Sync AI memories (Claude/Codex) via iCloud
    Memory {
        /// Run initial setup instead of syncing
        #[arg(long)]
        setup: bool,
    },

    /// Sync a local file or directory to iCloud Drive and symlink back
    #[command(arg_required_else_help = true)]
    Icloud {
        /// Local file or directory to sync (e.g., _plans or ~/.codex/config.toml)
        path: String,
    },
}

pub fn run_with_flags(sh: &Shell, flags: Sync) -> Result<()> {
    match flags.subcommand {
        SyncCmd::Memory { setup } => {
            if setup {
                memory::setup(sh)
            } else {
                memory::sync(sh)
            }
        }
        SyncCmd::Icloud { path } => icloud_sync(sh, &path),
    }
}

fn icloud_drive() -> Result<PathBuf> {
    Ok(fsutil::home_dir()?.join("Library/Mobile Documents/com~apple~CloudDocs"))
}

fn git_root(sh: &Shell) -> Result<PathBuf> {
    let output = cmd!(sh, "git rev-parse --show-toplevel").read()?;
    Ok(PathBuf::from(output.trim()))
}

fn icloud_sync(sh: &Shell, path: &str) -> Result<()> {
    let requested_path = expand_user_path(path)?;
    let root = git_root(sh)?;
    let local_path = if requested_path.is_absolute() {
        requested_path
    } else {
        root.join(requested_path)
    };

    if local_path.is_dir() {
        return icloud_sync_dir(sh, &root, &local_path);
    }

    let icloud_target = icloud_file_target(&icloud_drive()?, &fsutil::home_dir()?, &local_path)?;
    icloud_sync_file(sh, &local_path, &icloud_target)
}

fn icloud_sync_dir(sh: &Shell, root: &Path, local_path: &Path) -> Result<()> {
    // strip leading _ if present for iCloud category name
    let dir_name = local_path
        .file_name()
        .ok_or_else(|| eyre!("invalid directory: {}", local_path.display()))?
        .to_string_lossy();

    let category = dir_name.strip_prefix('_').unwrap_or(&dir_name);

    let project_name = root
        .file_name()
        .ok_or_else(|| eyre!("could not determine project name from git root"))?
        .to_string_lossy()
        .to_string();

    let icloud_target = icloud_drive()?.join(category).join(&project_name);

    if !sh.path_exists(local_path) && !local_path.is_symlink() {
        return Err(eyre!("directory does not exist: {}", local_path.display()));
    }

    sh.create_dir(&icloud_target)?;

    if local_path.is_symlink() {
        // already symlinked, just rsync contents
        info!("{} (already symlinked, syncing)", dir_name);
        rsync_two_way(sh, local_path, &icloud_target)?;
    } else {
        // first time: copy contents, remove dir, create symlink
        rsync_two_way(sh, local_path, &icloud_target)?;
        sh.remove_path(local_path)?;
        std::os::unix::fs::symlink(&icloud_target, local_path)?;
        info!(
            "{} {} → {}",
            "Linked".green(),
            dir_name,
            icloud_target.display()
        );
    }

    info!("{} {}/{}", "Synced".green().bold(), category, project_name);
    Ok(())
}

fn icloud_sync_file(sh: &Shell, local_path: &Path, icloud_target: &Path) -> Result<()> {
    if !sh.path_exists(local_path) && !local_path.is_symlink() {
        return Err(eyre!("file does not exist: {}", local_path.display()));
    }

    fsutil::ensure_parent_dir(icloud_target)?;

    if local_path.is_symlink() {
        info!("{} (already symlinked)", local_path.display());
    } else {
        std::fs::copy(local_path, icloud_target)?;
        sh.remove_path(local_path)?;
        std::os::unix::fs::symlink(icloud_target, local_path)?;
        info!(
            "{} {} → {}",
            "Linked".green(),
            local_path.display(),
            icloud_target.display()
        );
    }

    info!("{} {}", "Synced".green().bold(), icloud_target.display());
    Ok(())
}

fn expand_user_path(path: &str) -> Result<PathBuf> {
    if path == "~" {
        return fsutil::home_dir();
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return Ok(fsutil::home_dir()?.join(rest));
    }

    Ok(PathBuf::from(path))
}

fn icloud_file_target(icloud_drive: &Path, home: &Path, local_path: &Path) -> Result<PathBuf> {
    let relative_path = local_path
        .strip_prefix(home)
        .unwrap_or(local_path)
        .components()
        .filter_map(syncable_component)
        .collect::<PathBuf>();

    if relative_path.as_os_str().is_empty() {
        return Err(eyre!("invalid file path: {}", local_path.display()));
    }

    Ok(icloud_drive.join("dotfiles").join(relative_path))
}

fn syncable_component(component: Component<'_>) -> Option<PathBuf> {
    let Component::Normal(value) = component else {
        return None;
    };

    let value = value.to_string_lossy();
    Some(PathBuf::from(value.strip_prefix('.').unwrap_or(&value)))
}

fn rsync_two_way(sh: &Shell, local: &Path, icloud: &Path) -> Result<()> {
    // trailing slash means "contents of"
    let local_str = format!("{}/", local.display());
    let icloud_str = format!("{}/", icloud.display());

    // local → iCloud
    cmd!(sh, "rsync -a {local_str} {icloud_str}").run()?;
    // iCloud → local
    cmd!(sh, "rsync -a {icloud_str} {local_str}").run()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::icloud_file_target;
    use std::path::Path;

    #[test]
    fn codex_config_file_syncs_to_stable_icloud_dotfiles_path() {
        let target = icloud_file_target(
            Path::new("/Users/praveen/Library/Mobile Documents/com~apple~CloudDocs"),
            Path::new("/Users/praveen"),
            Path::new("/Users/praveen/.codex/config.toml"),
        )
        .unwrap();

        assert_eq!(
            target,
            Path::new(
                "/Users/praveen/Library/Mobile Documents/com~apple~CloudDocs/dotfiles/codex/config.toml"
            )
        );
    }
}
