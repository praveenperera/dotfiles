use std::path::{Path, PathBuf};

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

    /// Sync a local directory to iCloud Drive (2-way) and symlink back
    #[command(arg_required_else_help = true)]
    Icloud {
        /// Local directory to sync (e.g., _plans)
        dir: String,
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
        SyncCmd::Icloud { dir } => icloud_sync(sh, &dir),
    }
}

fn icloud_drive() -> Result<PathBuf> {
    Ok(fsutil::home_dir()?.join("Library/Mobile Documents/com~apple~CloudDocs"))
}

fn git_root(sh: &Shell) -> Result<PathBuf> {
    let output = cmd!(sh, "git rev-parse --show-toplevel").read()?;
    Ok(PathBuf::from(output.trim()))
}

fn icloud_sync(sh: &Shell, dir: &str) -> Result<()> {
    let local_dir = Path::new(dir);

    // strip leading _ if present for iCloud category name
    let dir_name = local_dir
        .file_name()
        .ok_or_else(|| eyre!("invalid directory: {dir}"))?
        .to_string_lossy();

    let category = dir_name.strip_prefix('_').unwrap_or(&dir_name);

    let root = git_root(sh)?;
    let project_name = root
        .file_name()
        .ok_or_else(|| eyre!("could not determine project name from git root"))?
        .to_string_lossy()
        .to_string();

    let icloud_target = icloud_drive()?.join(category).join(&project_name);
    let local_path = root.join(&*dir_name);

    if !sh.path_exists(&local_path) && !local_path.is_symlink() {
        return Err(eyre!("directory does not exist: {}", local_path.display()));
    }

    sh.create_dir(&icloud_target)?;

    if local_path.is_symlink() {
        // already symlinked, just rsync contents
        info!("{} (already symlinked, syncing)", dir_name);
        rsync_two_way(sh, &local_path, &icloud_target)?;
    } else {
        // first time: copy contents, remove dir, create symlink
        rsync_two_way(sh, &local_path, &icloud_target)?;
        sh.remove_path(&local_path)?;
        std::os::unix::fs::symlink(&icloud_target, &local_path)?;
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
