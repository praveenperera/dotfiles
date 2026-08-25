use std::path::{Path, PathBuf};

use eyre::{eyre, Result, WrapErr};
use xshell::{cmd, Shell};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectRoot(PathBuf);

impl ProjectRoot {
    pub(crate) fn resolve(sh: &Shell) -> Result<Self> {
        let output = cmd!(sh, "git rev-parse --show-toplevel")
            .env("LC_ALL", "C")
            .ignore_status()
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8(output.stdout)
                .wrap_err("git returned a non-UTF-8 project root")?;

            return Self::from_git_output(&stdout);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            return Self::from_absolute_path(sh.current_dir());
        }

        Err(eyre!(
            "failed to resolve project root with git: {}",
            stderr.trim()
        ))
    }

    fn from_git_output(output: &str) -> Result<Self> {
        Self::from_absolute_path(PathBuf::from(output.trim()))
            .wrap_err("git returned an invalid project root")
    }

    fn from_absolute_path(path: PathBuf) -> Result<Self> {
        if path.as_os_str().is_empty() {
            return Err(eyre!("project root is empty"));
        }
        if !path.is_absolute() {
            return Err(eyre!("project root is not absolute: {}", path.display()));
        }

        let path = path
            .canonicalize()
            .wrap_err_with(|| format!("failed to canonicalize project root: {}", path.display()))?;

        Ok(Self(path))
    }
}

impl AsRef<Path> for ProjectRoot {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectRoot;
    use xshell::{cmd, Shell};

    #[test]
    fn uses_current_directory_outside_git_repository() {
        let dir = tempfile::tempdir().unwrap();
        let sh = Shell::new().unwrap();
        let _dir = sh.push_dir(dir.path());

        let project_root = ProjectRoot::resolve(&sh).unwrap();

        assert_eq!(project_root.as_ref(), dir.path().canonicalize().unwrap());
    }

    #[test]
    fn uses_git_top_level_from_nested_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested_dir = dir.path().join("nested/project");
        std::fs::create_dir_all(&nested_dir).unwrap();
        let sh = Shell::new().unwrap();
        let _dir = sh.push_dir(dir.path());
        cmd!(sh, "git init --quiet").run().unwrap();
        let _nested_dir = sh.push_dir(&nested_dir);

        let project_root = ProjectRoot::resolve(&sh).unwrap();

        assert_eq!(project_root.as_ref(), dir.path().canonicalize().unwrap());
    }
}
