use std::fs;
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

        /// Shared iCloud source name to use instead of the git root directory name
        source: Option<String>,
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
        SyncCmd::Icloud { path, source } => icloud_sync(sh, &path, source.as_deref()),
    }
}

fn icloud_drive() -> Result<PathBuf> {
    Ok(fsutil::home_dir()?.join("Library/Mobile Documents/com~apple~CloudDocs/local_sync"))
}

fn git_root(sh: &Shell) -> Result<PathBuf> {
    let output = cmd!(sh, "git rev-parse --show-toplevel").read()?;
    Ok(PathBuf::from(output.trim()))
}

fn icloud_sync(sh: &Shell, path: &str, source: Option<&str>) -> Result<()> {
    let requested_path = expand_user_path(path)?;
    let root = git_root(sh)?;
    let local_path = if requested_path == Path::new(".") {
        root.clone()
    } else if requested_path.is_absolute() {
        requested_path
    } else {
        root.join(requested_path)
    };

    if local_path.is_dir() {
        return icloud_sync_dir(sh, &root, &local_path, source);
    }

    if source.is_some() {
        return Err(eyre!(
            "refusing to use an iCloud source name for file sync: {path}\n\
             source names change the shared directory target, but file syncs use a stable path under iCloud Drive/local_sync/dotfiles\n\
             run without the source name, or sync a directory instead"
        ));
    }

    let icloud_target = icloud_file_target(&icloud_drive()?, &fsutil::home_dir()?, &local_path)?;
    icloud_sync_file(sh, &local_path, &icloud_target)
}

fn icloud_sync_dir(sh: &Shell, root: &Path, local_path: &Path, source: Option<&str>) -> Result<()> {
    let target = IcloudDirTarget::resolve(&icloud_drive()?, root, local_path, source)?;

    if !sh.path_exists(local_path) && !local_path.is_symlink() {
        return Err(eyre!("directory does not exist: {}", local_path.display()));
    }

    let target_has_content = sh.path_exists(&target.path) && !is_empty_dir(&target.path)?;
    sh.create_dir(&target.path)?;

    if local_path.is_symlink() {
        ensure_symlink_target(local_path, &target.path)?;
        // already symlinked, just rsync contents
        info!("{} (already symlinked, syncing)", target.dir_name);
        rsync_two_way(sh, local_path, &target.path)?;
    } else {
        if target_has_content && !is_empty_dir(local_path)? {
            return Err(eyre!(
                "refusing to merge two non-empty directories\n\
                 local directory: {}\n\
                 iCloud target: {}\n\
                 continuing could overwrite files in the shared iCloud source\n\
                 move the local directory aside, empty it before linking, or choose a new source name",
                local_path.display(),
                target.path.display()
            ));
        }

        // first time: copy contents, remove dir, create symlink
        rsync_two_way(sh, local_path, &target.path)?;
        sh.remove_path(local_path)?;
        std::os::unix::fs::symlink(&target.path, local_path)?;
        info!(
            "{} {} → {}",
            "Linked".green(),
            target.dir_name,
            target.path.display()
        );
    }

    info!(
        "{} {}/{}",
        "Synced".green().bold(),
        target.category,
        target.source.as_str()
    );
    Ok(())
}

fn icloud_sync_file(sh: &Shell, local_path: &Path, icloud_target: &Path) -> Result<()> {
    if !sh.path_exists(local_path) && !local_path.is_symlink() {
        return Err(eyre!("file does not exist: {}", local_path.display()));
    }

    fsutil::ensure_parent_dir(icloud_target)?;

    if local_path.is_symlink() {
        ensure_symlink_target(local_path, icloud_target)?;
        info!("{} (already symlinked)", local_path.display());
    } else {
        if sh.path_exists(icloud_target) && !files_match(local_path, icloud_target)? {
            return Err(eyre!(
                "refusing to overwrite an existing iCloud file with different contents\n\
                 local file: {}\n\
                 iCloud file: {}\n\
                 continuing would replace the shared copy\n\
                 compare the files manually, remove the stale iCloud file, or update the local file to match",
                local_path.display(),
                icloud_target.display(),
            ));
        }

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

struct IcloudDirTarget {
    dir_name: String,
    category: String,
    source: SyncSource,
    path: PathBuf,
}

impl IcloudDirTarget {
    fn resolve(
        icloud_drive: &Path,
        root: &Path,
        local_path: &Path,
        source: Option<&str>,
    ) -> Result<Self> {
        let dir_name = local_path
            .file_name()
            .ok_or_else(|| eyre!("invalid directory: {}", local_path.display()))?
            .to_string_lossy()
            .to_string();

        let category = if local_path == root {
            "repos".to_string()
        } else {
            dir_name.strip_prefix('_').unwrap_or(&dir_name).to_string()
        };

        let source = match source {
            Some(source) => SyncSource::parse(source)?,
            None => SyncSource::from_git_root(root)?,
        };

        let path = icloud_drive.join(&category).join(source.as_str());

        Ok(Self {
            dir_name,
            category,
            source,
            path,
        })
    }
}

#[derive(Debug)]
struct SyncSource(String);

impl SyncSource {
    fn parse(source: &str) -> Result<Self> {
        let path = Path::new(source);
        let mut components = path.components();
        let Some(Component::Normal(value)) = components.next() else {
            return Err(eyre!(
                "invalid iCloud source name: {source}\n\
                 use a plain name like `cove`; absolute paths, `.` and `..` are not valid source names"
            ));
        };

        if components.next().is_some() {
            return Err(eyre!(
                "invalid iCloud source name: {source}\n\
                 source names cannot contain path separators because they select one shared folder name\n\
                 use a plain name like `cove`"
            ));
        }

        Ok(Self(value.to_string_lossy().to_string()))
    }

    fn from_git_root(root: &Path) -> Result<Self> {
        Ok(Self(
            root.file_name()
                .ok_or_else(|| eyre!("could not determine project name from git root"))?
                .to_string_lossy()
                .to_string(),
        ))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

fn ensure_symlink_target(path: &Path, expected_target: &Path) -> Result<()> {
    let target = fs::read_link(path)?;
    if target == expected_target {
        return Ok(());
    }

    Err(eyre!(
        "refusing to sync an existing symlink that points at a different target\n\
         local symlink: {}\n\
         current target: {}\n\
         expected iCloud target: {}\n\
         continuing would sync the wrong shared source\n\
         remove or update the symlink if this location should use the expected target",
        path.display(),
        target.display(),
        expected_target.display()
    ))
}

fn is_empty_dir(path: &Path) -> Result<bool> {
    Ok(fs::read_dir(path)?.next().is_none())
}

fn files_match(left: &Path, right: &Path) -> Result<bool> {
    let left_metadata = fs::metadata(left)?;
    let right_metadata = fs::metadata(right)?;
    if !left_metadata.is_file() || !right_metadata.is_file() {
        return Ok(false);
    }

    if left_metadata.len() != right_metadata.len() {
        return Ok(false);
    }

    Ok(fs::read(left)? == fs::read(right)?)
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
    use super::{icloud_file_target, IcloudDirTarget, SyncSource};
    use std::path::Path;

    #[test]
    fn codex_config_file_syncs_to_stable_icloud_dotfiles_path() {
        let target = icloud_file_target(
            Path::new("~/Library/Mobile Documents/com~apple~CloudDocs/local_sync"),
            Path::new("~"),
            Path::new("~/.codex/config.toml"),
        )
        .unwrap();

        assert_eq!(
            target,
            Path::new(
                "~/Library/Mobile Documents/com~apple~CloudDocs/local_sync/dotfiles/codex/config.toml"
            )
        );
    }

    #[test]
    fn directory_sync_uses_git_root_name_by_default() {
        let target = IcloudDirTarget::resolve(
            Path::new("~/Library/Mobile Documents/com~apple~CloudDocs/local_sync"),
            Path::new("~/code/bitcoinppl/cove-wk1"),
            Path::new("~/code/bitcoinppl/cove-wk1/_plans"),
            None,
        )
        .unwrap();

        assert_eq!(target.category, "plans");
        assert_eq!(target.source.as_str(), "cove-wk1");
        assert_eq!(
            target.path,
            Path::new("~/Library/Mobile Documents/com~apple~CloudDocs/local_sync/plans/cove-wk1")
        );
    }

    #[test]
    fn directory_sync_source_overrides_git_root_name() {
        let target = IcloudDirTarget::resolve(
            Path::new("~/Library/Mobile Documents/com~apple~CloudDocs/local_sync"),
            Path::new("~/code/bitcoinppl/cove-wk1"),
            Path::new("~/code/bitcoinppl/cove-wk1/_plans"),
            Some("cove"),
        )
        .unwrap();

        assert_eq!(target.category, "plans");
        assert_eq!(target.source.as_str(), "cove");
        assert_eq!(
            target.path,
            Path::new("~/Library/Mobile Documents/com~apple~CloudDocs/local_sync/plans/cove")
        );
    }

    #[test]
    fn git_root_directory_sync_uses_repos_category() {
        let target = IcloudDirTarget::resolve(
            Path::new("~/Library/Mobile Documents/com~apple~CloudDocs/local_sync"),
            Path::new("~/code/bitcoinppl/cove-wk1"),
            Path::new("~/code/bitcoinppl/cove-wk1"),
            Some("cove"),
        )
        .unwrap();

        assert_eq!(target.category, "repos");
        assert_eq!(target.source.as_str(), "cove");
        assert_eq!(
            target.path,
            Path::new("~/Library/Mobile Documents/com~apple~CloudDocs/local_sync/repos/cove")
        );
    }

    #[test]
    fn source_name_rejects_path_separators() {
        let err = SyncSource::parse("cove/wk1").unwrap_err();

        assert!(
            err.to_string().contains("path separators"),
            "unexpected error: {err}"
        );
    }
}
