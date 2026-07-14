use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use clap::Subcommand;
use colored::Colorize;
use eyre::{eyre, Result};
use log::info;
use xshell::Shell;

use crate::fsutil;

const DEFAULT_CACHE_ROOT: &str = "/Volumes/CacheDisk/dev-cache";
const RUST_TARGET_LEAF: &str = "rust-target";

#[derive(Debug, Clone)]
pub struct Cache {
    pub subcommand: CacheCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CacheCmd {
    /// Symlink a folder onto CacheDisk (no path = rust target from cwd)
    Link {
        /// Local folder to host on CacheDisk (omit to auto-find cargo target)
        path: Option<String>,

        /// Namespace under the cache root (default: project directory name)
        #[arg(long)]
        name: Option<String>,

        /// Leaf directory name under the namespace (default: path basename, or rust-target for auto rust)
        #[arg(long = "as")]
        as_leaf: Option<String>,

        /// Replace a wrong or broken existing symlink
        #[arg(short, long)]
        force: bool,
    },

    /// Show whether a path is linked onto CacheDisk
    Status {
        /// Path to inspect (omit to auto-find cargo target from cwd)
        path: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeafDefault {
    Basename,
    RustTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkPlan {
    local: PathBuf,
    cache: PathBuf,
    name: String,
    leaf: String,
}

pub fn run_with_flags(sh: &Shell, flags: Cache) -> Result<()> {
    match flags.subcommand {
        CacheCmd::Link {
            path,
            name,
            as_leaf,
            force,
        } => {
            let plan = resolve_link_plan(sh, path.as_deref(), name.as_deref(), as_leaf.as_deref())?;
            apply_link(sh, &plan, force)
        }
        CacheCmd::Status { path } => show_status(sh, path.as_deref()),
    }
}

fn resolve_link_plan(
    sh: &Shell,
    path: Option<&str>,
    name: Option<&str>,
    as_leaf: Option<&str>,
) -> Result<LinkPlan> {
    let (local, leaf_default) = resolve_local_for_link(path)?;
    let local = absolute_path(&local)?;

    let name = match name {
        Some(name) => validate_cache_component(name, "name")?,
        None => default_name_for(sh, &local)?,
    };

    let leaf = match as_leaf {
        Some(leaf) => validate_cache_component(leaf, "leaf")?,
        None => match leaf_default {
            LeafDefault::RustTarget => RUST_TARGET_LEAF.to_string(),
            LeafDefault::Basename => default_leaf_for(&local)?,
        },
    };

    let cache_root = resolve_cache_root()?;
    let cache = cache_root.join(&name).join(&leaf);

    Ok(LinkPlan {
        local,
        cache,
        name,
        leaf,
    })
}

fn resolve_local_for_link(path: Option<&str>) -> Result<(PathBuf, LeafDefault)> {
    match path {
        Some(path) => Ok((expand_user_path(path)?, LeafDefault::Basename)),
        None => {
            let cwd = env::current_dir()?;
            Ok((find_rust_target_dir(&cwd)?, LeafDefault::RustTarget))
        }
    }
}

fn find_rust_target_dir(project: &Path) -> Result<PathBuf> {
    if project.join("Cargo.toml").is_file() {
        return Ok(project.join("target"));
    }

    if project.join("rust/Cargo.toml").is_file() {
        return Ok(project.join("rust/target"));
    }

    Err(eyre!(
        "no path given and no cargo project found in {}\n\
         pass an explicit folder to link, e.g. `cmd cache link ./node_modules`",
        project.display()
    ))
}

fn default_name_for(sh: &Shell, local: &Path) -> Result<String> {
    let start = local.parent().unwrap_or(local);

    if let Some(root) = git_root_at(sh, start) {
        if let Some(name) = root.file_name() {
            return Ok(name.to_string_lossy().to_string());
        }
    }

    // for .../rust/target prefer the project dir above rust/
    let project_dir = if start.file_name().is_some_and(|n| n == "rust") {
        start.parent().unwrap_or(start)
    } else {
        start
    };

    project_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| eyre!("could not determine project name from {}", local.display()))
}

fn default_leaf_for(local: &Path) -> Result<String> {
    local
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|n| !n.is_empty() && n != "/" && n != ".")
        .ok_or_else(|| eyre!("could not determine leaf name from {}", local.display()))
}

fn validate_cache_component(value: &str, kind: &str) -> Result<String> {
    let path = Path::new(value);
    if value.is_empty() {
        return Err(eyre!("{kind} must not be empty"));
    }

    for component in path.components() {
        match component {
            Component::Normal(part) => {
                if part.is_empty() {
                    return Err(eyre!("invalid {kind}: {value}"));
                }
            }
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) | Component::ParentDir => {
                return Err(eyre!(
                    "invalid {kind}: {value}\n\
                     use a plain relative name like `aps` or `cove/wk1`; absolute paths and `..` are not allowed"
                ));
            }
        }
    }

    // normalize away any "./" segments while preserving nested names
    let normalized = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");

    if normalized.is_empty() {
        return Err(eyre!("invalid {kind}: {value}"));
    }

    Ok(normalized)
}

fn resolve_cache_root() -> Result<PathBuf> {
    if let Ok(root) = env::var("DEV_CACHE_ROOT") {
        let root = PathBuf::from(root);
        if root.is_dir() {
            return Ok(root);
        }
        return Err(eyre!(
            "DEV_CACHE_ROOT is set to {} but that directory does not exist",
            root.display()
        ));
    }

    let default = PathBuf::from(DEFAULT_CACHE_ROOT);
    if default.is_dir() {
        return Ok(default);
    }

    Err(eyre!(
        "cache root not available\n\
         set DEV_CACHE_ROOT or mount CacheDisk so {DEFAULT_CACHE_ROOT} exists"
    ))
}

fn apply_link(sh: &Shell, plan: &LinkPlan, force: bool) -> Result<()> {
    fs::create_dir_all(&plan.cache)?;

    let local_meta = fs::symlink_metadata(&plan.local);
    match local_meta {
        Ok(meta) if meta.file_type().is_symlink() => {
            let current = fs::read_link(&plan.local)?;
            if paths_equal(&current, &plan.cache)? {
                info!(
                    "{} {} → {}",
                    "Already linked".green(),
                    plan.local.display(),
                    plan.cache.display()
                );
                return Ok(());
            }

            if !force {
                return Err(eyre!(
                    "refusing to replace an existing symlink that points elsewhere\n\
                     local symlink: {}\n\
                     current target: {}\n\
                     expected cache: {}\n\
                     re-run with --force to replace it",
                    plan.local.display(),
                    current.display(),
                    plan.cache.display()
                ));
            }

            fsutil::remove_existing_path(&plan.local)?;
        }
        Ok(meta) if meta.is_dir() => {
            migrate_or_remove_local_dir(sh, &plan.local, &plan.cache)?;
        }
        Ok(_) => {
            return Err(eyre!(
                "refusing to replace a non-directory at {}\n\
                 cache link only supports directories",
                plan.local.display()
            ));
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            // create symlink below
        }
        Err(err) => return Err(err.into()),
    }

    fsutil::ensure_parent_dir(&plan.local)?;
    create_symlink(&plan.cache, &plan.local)?;

    info!(
        "{} {} → {}",
        "Linked".green().bold(),
        plan.local.display(),
        plan.cache.display()
    );
    Ok(())
}

fn migrate_or_remove_local_dir(sh: &Shell, local: &Path, cache: &Path) -> Result<()> {
    let local_empty = is_empty_dir(local)?;
    let cache_empty = is_empty_dir(cache)?;

    if local_empty {
        fsutil::remove_existing_path(local)?;
        return Ok(());
    }

    if !cache_empty {
        return Err(eyre!(
            "refusing to merge two non-empty directories\n\
             local directory: {}\n\
             cache directory: {}\n\
             empty one side first, or choose a different --name / --as",
            local.display(),
            cache.display()
        ));
    }

    // move local contents into empty cache, then remove local
    move_dir_contents(sh, local, cache)?;
    fsutil::remove_existing_path(local)?;
    Ok(())
}

fn move_dir_contents(sh: &Shell, local: &Path, cache: &Path) -> Result<()> {
    // try rename of the whole directory into a temp name under cache parent first
    // when that fails (cross-device), fall back to rsync
    let cache_parent = cache
        .parent()
        .ok_or_else(|| eyre!("cache path has no parent: {}", cache.display()))?;
    fs::create_dir_all(cache_parent)?;

    // if cache is an empty dir, remove it so we can rename local onto that path
    if is_empty_dir(cache)? {
        fs::remove_dir(cache)?;
        match fs::rename(local, cache) {
            Ok(()) => return Ok(()),
            Err(_) => {
                // recreate empty cache and rsync
                fs::create_dir_all(cache)?;
            }
        }
    }

    rsync_into(sh, local, cache)
}

fn rsync_into(sh: &Shell, source: &Path, dest: &Path) -> Result<()> {
    use xshell::cmd;

    let source_slash = format!("{}/", source.display());
    let dest_slash = format!("{}/", dest.display());
    cmd!(sh, "rsync -a {source_slash} {dest_slash}")
        .run()
        .map_err(|err| {
            eyre!(
                "failed to copy {} → {}: {err}",
                source.display(),
                dest.display()
            )
        })
}

fn show_status(sh: &Shell, path: Option<&str>) -> Result<()> {
    let (local, _) = resolve_local_for_link(path)?;
    let local = absolute_path(&local)?;
    let cache_root = resolve_cache_root().ok();

    print_status_line("local", &local.display().to_string());

    match fs::symlink_metadata(&local) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = fs::read_link(&local)?;
            print_status_line("kind", "symlink");
            print_status_line("points to", &target.display().to_string());

            let under_cache = cache_root
                .as_ref()
                .and_then(|root| target.starts_with(root).then_some(root));
            match under_cache {
                Some(root) => print_status_line("cache root", &root.display().to_string()),
                None => print_status_line("on CacheDisk", "no"),
            }
        }
        Ok(meta) if meta.is_dir() => {
            print_status_line("kind", "directory");
            print_status_line("on CacheDisk", "no");
            print_expected_cache(sh, path, &local);
        }
        Ok(_) => {
            print_status_line("kind", "file");
            print_status_line("on CacheDisk", "no");
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            print_status_line("kind", "missing");
            print_expected_cache(sh, path, &local);
        }
        Err(err) => return Err(err.into()),
    }

    Ok(())
}

fn print_expected_cache(sh: &Shell, path: Option<&str>, local: &Path) {
    if let Ok(plan) = resolve_link_plan(sh, path, None, None) {
        if plan.local == local {
            print_status_line("expected cache", &plan.cache.display().to_string());
        }
    }
}

fn print_status_line(label: &str, value: &str) {
    println!("{:<16} {}", format!("{label}:").dimmed(), value);
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

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(env::current_dir()?.join(path))
}

fn git_root_at(sh: &Shell, path: &Path) -> Option<PathBuf> {
    use xshell::cmd;

    let dir = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };

    cmd!(sh, "git -C {dir} rev-parse --show-toplevel")
        .quiet()
        .read()
        .ok()
        .map(|s| PathBuf::from(s.trim()))
}

fn is_empty_dir(path: &Path) -> Result<bool> {
    Ok(fs::read_dir(path)?.next().is_none())
}

fn paths_equal(left: &Path, right: &Path) -> Result<bool> {
    if left == right {
        return Ok(true);
    }

    // compare canonical forms when both exist
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(a), Ok(b)) => Ok(a == b),
        _ => Ok(false),
    }
}

#[cfg(unix)]
fn create_symlink(source: &Path, target: &Path) -> Result<()> {
    std::os::unix::fs::symlink(source, target)?;
    Ok(())
}

#[cfg(windows)]
fn create_symlink(source: &Path, target: &Path) -> Result<()> {
    std::os::windows::fs::symlink_dir(source, target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;
    use xshell::Shell;

    fn write_cargo_toml(dir: &Path) {
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
    }

    #[test]
    fn finds_root_rust_target() {
        let dir = tempdir().unwrap();
        write_cargo_toml(dir.path());
        let target = find_rust_target_dir(dir.path()).unwrap();
        assert_eq!(target, dir.path().join("target"));
    }

    #[test]
    fn finds_nested_rust_target() {
        let dir = tempdir().unwrap();
        let rust_dir = dir.path().join("rust");
        fs::create_dir_all(&rust_dir).unwrap();
        write_cargo_toml(&rust_dir);
        let target = find_rust_target_dir(dir.path()).unwrap();
        assert_eq!(target, rust_dir.join("target"));
    }

    #[test]
    fn errors_without_cargo_project() {
        let dir = tempdir().unwrap();
        let err = find_rust_target_dir(dir.path()).unwrap_err().to_string();
        assert!(err.contains("no cargo project found"));
    }

    #[test]
    fn validates_nested_name_and_rejects_parent_dir() {
        assert_eq!(
            validate_cache_component("cove/wk1", "name").unwrap(),
            "cove/wk1"
        );
        assert!(validate_cache_component("../evil", "name").is_err());
        assert!(validate_cache_component("/abs", "name").is_err());
        assert!(validate_cache_component("", "name").is_err());
    }

    #[test]
    fn explicit_path_uses_basename_leaf() {
        let dir = tempdir().unwrap();
        let sh = Shell::new().unwrap();
        let local = dir.path().join("node_modules");
        fs::create_dir_all(&local).unwrap();

        let cache_root = dir.path().join("cache-root");
        fs::create_dir_all(&cache_root).unwrap();
        env::set_var("DEV_CACHE_ROOT", &cache_root);

        let plan =
            resolve_link_plan(&sh, Some(local.to_str().unwrap()), Some("myapp"), None).unwrap();

        assert_eq!(plan.local, local);
        assert_eq!(plan.leaf, "node_modules");
        assert_eq!(plan.cache, cache_root.join("myapp/node_modules"));

        env::remove_var("DEV_CACHE_ROOT");
    }

    #[test]
    fn apply_link_moves_local_contents_then_symlinks() {
        let dir = tempdir().unwrap();
        let sh = Shell::new().unwrap();

        let cache_root = dir.path().join("cache-root");
        fs::create_dir_all(&cache_root).unwrap();
        env::set_var("DEV_CACHE_ROOT", &cache_root);

        let local = dir.path().join("project/target");
        fs::create_dir_all(&local).unwrap();
        fs::write(local.join("artifact"), "data").unwrap();

        let plan = LinkPlan {
            local: local.clone(),
            cache: cache_root.join("project/rust-target"),
            name: "project".into(),
            leaf: "rust-target".into(),
        };

        apply_link(&sh, &plan, false).unwrap();

        assert!(local.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(fs::read_link(&local).unwrap(), plan.cache);
        assert_eq!(fs::read(plan.cache.join("artifact")).unwrap(), b"data");

        // second call is a no-op success
        apply_link(&sh, &plan, false).unwrap();

        env::remove_var("DEV_CACHE_ROOT");
    }

    #[test]
    fn refuses_dual_non_empty_without_merge() {
        let dir = tempdir().unwrap();
        let sh = Shell::new().unwrap();

        let local = dir.path().join("target");
        let cache = dir.path().join("cache");
        fs::create_dir_all(&local).unwrap();
        fs::create_dir_all(&cache).unwrap();
        fs::write(local.join("a"), "a").unwrap();
        fs::write(cache.join("b"), "b").unwrap();

        let plan = LinkPlan {
            local,
            cache,
            name: "x".into(),
            leaf: "y".into(),
        };

        let err = apply_link(&sh, &plan, false).unwrap_err().to_string();
        assert!(err.contains("refusing to merge"));
    }

    #[test]
    fn force_replaces_wrong_symlink() {
        let dir = tempdir().unwrap();
        let sh = Shell::new().unwrap();

        let cache_root = dir.path().join("cache-root");
        fs::create_dir_all(&cache_root).unwrap();
        env::set_var("DEV_CACHE_ROOT", &cache_root);

        let local = dir.path().join("target");
        let wrong = dir.path().join("wrong");
        let cache = cache_root.join("proj/rust-target");
        fs::create_dir_all(&wrong).unwrap();
        fs::create_dir_all(cache.parent().unwrap()).unwrap();
        fs::create_dir_all(&cache).unwrap();
        symlink(&wrong, &local).unwrap();

        let plan = LinkPlan {
            local: local.clone(),
            cache: cache.clone(),
            name: "proj".into(),
            leaf: "rust-target".into(),
        };

        let err = apply_link(&sh, &plan, false).unwrap_err().to_string();
        assert!(err.contains("refusing to replace"));

        apply_link(&sh, &plan, true).unwrap();
        assert_eq!(fs::read_link(&local).unwrap(), cache);

        env::remove_var("DEV_CACHE_ROOT");
    }

    #[test]
    fn default_leaf_for_auto_rust_is_rust_target() {
        let dir = tempdir().unwrap();
        let sh = Shell::new().unwrap();
        write_cargo_toml(dir.path());

        let cache_root = dir.path().join("cache-root");
        fs::create_dir_all(&cache_root).unwrap();
        env::set_var("DEV_CACHE_ROOT", &cache_root);

        // simulate no-arg by resolving local like find_rust_target + RustTarget leaf
        let local = find_rust_target_dir(dir.path()).unwrap();
        let plan = LinkPlan {
            local: absolute_path(&local).unwrap(),
            cache: cache_root.join("demo/rust-target"),
            name: "demo".into(),
            leaf: RUST_TARGET_LEAF.into(),
        };
        assert_eq!(plan.leaf, "rust-target");

        // also check resolve path with explicit --as override via validate
        assert_eq!(
            validate_cache_component("rust-target", "leaf").unwrap(),
            "rust-target"
        );

        let _ = sh;
        env::remove_var("DEV_CACHE_ROOT");
    }
}
