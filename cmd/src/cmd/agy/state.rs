//! Which conversation databases an earlier run uploaded, so a repeated run only reads changed ones

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

/// A file's size and modification time
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp(u64, u128);

impl Stamp {
    fn of(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        let modified = metadata
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        Some(Self(metadata.len(), modified))
    }
}

/// A database and its write-ahead log, which holds its newest steps until SQLite checkpoints it
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    db: Option<Stamp>,
    wal: Option<Stamp>,
}

impl Fingerprint {
    pub fn of(path: &Path) -> Self {
        Self {
            db: Stamp::of(path),
            wal: Stamp::of(&path.with_extension("db-wal")),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct State {
    files: BTreeMap<PathBuf, Fingerprint>,
}

impl State {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .wrap_err_with(|| format!("reading {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error).wrap_err_with(|| format!("reading {}", path.display())),
        }
    }

    pub fn unchanged(&self, path: &Path, fingerprint: &Fingerprint) -> bool {
        self.files.get(path) == Some(fingerprint)
    }

    pub fn mark(&mut self, path: PathBuf, fingerprint: Fingerprint) {
        self.files.insert(path, fingerprint);
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        // write then rename, so an interrupted run never leaves a half-written state file
        let partial = path.with_extension("json.partial");
        fs::write(&partial, serde_json::to_vec_pretty(self)?)?;
        fs::rename(&partial, path).wrap_err_with(|| format!("writing {}", path.display()))
    }
}
