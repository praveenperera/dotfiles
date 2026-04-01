use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use eyre::{eyre, Result, WrapErr};

pub fn home_dir() -> Result<PathBuf> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .wrap_err("HOME env var not set")
}

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| eyre!("path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)?;
    Ok(())
}

pub fn remove_existing_path(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };

    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }

    Ok(())
}
