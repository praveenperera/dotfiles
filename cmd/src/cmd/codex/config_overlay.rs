use super::*;
use crate::fsutil;
use tempfile::NamedTempFile;
use toml::Value as TomlValue;

const CONFIG_FILE: &str = "config.toml";
const CONFIG_BASE_FILE: &str = "config.base.toml";
const CONFIG_LOCAL_FILE: &str = "config.local.toml";

pub(super) fn run_config_command(command: CodexConfigCmd) -> Result<()> {
    let shared_codex_home = codex_dir()?;
    match command {
        CodexConfigCmd::Render { group } => {
            let config_home = config_home_for_command(&shared_codex_home, group.as_deref())?;
            render_config_home(&config_home)?;
            println!("Rendered codex config: {}", config_home.display());
            Ok(())
        }
        CodexConfigCmd::SyncProjects { group, all } => {
            if all {
                sync_all_config_projects(&shared_codex_home)
            } else {
                let config_home = config_home_for_command(&shared_codex_home, group.as_deref())?;
                sync_projects_from_generated_config(&config_home)?;
                render_config_home(&config_home)?;
                println!("Synced codex project trust: {}", config_home.display());
                Ok(())
            }
        }
        CodexConfigCmd::Migrate { group, all } => {
            if all {
                migrate_all_config_homes(&shared_codex_home)
            } else {
                let config_home = config_home_for_command(&shared_codex_home, group.as_deref())?;
                migrate_config_home(&config_home)?;
                println!("Migrated codex config: {}", config_home.display());
                Ok(())
            }
        }
    }
}

pub(super) fn render_config_home(config_home: &Path) -> Result<()> {
    if !config_home.join(CONFIG_BASE_FILE).exists() && config_home.join(CONFIG_FILE).exists() {
        migrate_config_home(config_home)?;
        return Ok(());
    }

    let base_config = read_toml_file(&config_home.join(CONFIG_BASE_FILE)).wrap_err_with(|| {
        format!(
            "Failed to read {}",
            config_home.join(CONFIG_BASE_FILE).display()
        )
    })?;
    validate_base_config(&base_config)?;

    let mut generated = base_config;
    let local_config_path = config_home.join(CONFIG_LOCAL_FILE);
    if local_config_path.exists() {
        let local_config = read_toml_file(&local_config_path)
            .wrap_err_with(|| format!("Failed to read {}", local_config_path.display()))?;
        validate_local_config(&local_config)?;
        deep_merge(&mut generated, local_config);
    }

    write_toml_file(&config_home.join(CONFIG_FILE), &generated)
}

pub(super) fn sync_projects_from_generated_config(config_home: &Path) -> Result<()> {
    let generated_path = config_home.join(CONFIG_FILE);
    if !generated_path.exists() {
        return Ok(());
    }

    let generated = read_toml_file(&generated_path)
        .wrap_err_with(|| format!("Failed to read {}", generated_path.display()))?;
    write_local_projects_overlay(config_home, projects_value(&generated).cloned())
}

pub(super) fn migrate_config_home(config_home: &Path) -> Result<()> {
    stdfs::create_dir_all(config_home)?;
    let base_path = config_home.join(CONFIG_BASE_FILE);
    let generated_path = config_home.join(CONFIG_FILE);
    if base_path.exists() {
        sync_projects_from_generated_config(config_home)?;
        render_config_home(config_home)?;
        return Ok(());
    }
    if !generated_path.exists() {
        return Err(eyre!(
            "No {} or {} found in {}",
            CONFIG_BASE_FILE,
            CONFIG_FILE,
            config_home.display()
        ));
    }

    let mut base_config = read_toml_file(&generated_path)
        .wrap_err_with(|| format!("Failed to read {}", generated_path.display()))?;
    let projects = take_projects_value(&mut base_config);
    write_toml_file(&base_path, &base_config)?;
    write_local_projects_overlay(config_home, projects)?;
    render_config_home(config_home)
}

pub(super) fn sync_projects_after_launch(config_home: &Path) -> Result<()> {
    sync_projects_from_generated_config(config_home)?;
    render_config_home(config_home)
}

fn config_home_for_command(shared_codex_home: &Path, group: Option<&str>) -> Result<PathBuf> {
    match group {
        Some(group) => prepare_config_group_home(shared_codex_home, group),
        None => Ok(shared_codex_home.to_path_buf()),
    }
}

fn sync_all_config_projects(shared_codex_home: &Path) -> Result<()> {
    let mut homes = all_config_homes(shared_codex_home)?;
    homes.sort();
    homes.dedup();
    for config_home in homes {
        sync_projects_from_generated_config(&config_home)?;
        render_config_home(&config_home)?;
        println!("Synced codex project trust: {}", config_home.display());
    }
    Ok(())
}

fn migrate_all_config_homes(shared_codex_home: &Path) -> Result<()> {
    let mut homes = all_config_homes(shared_codex_home)?;
    homes.sort();
    homes.dedup();
    for config_home in homes {
        if config_home.join(CONFIG_BASE_FILE).exists() || config_home.join(CONFIG_FILE).exists() {
            migrate_config_home(&config_home)?;
            println!("Migrated codex config: {}", config_home.display());
        }
    }
    Ok(())
}

fn all_config_homes(shared_codex_home: &Path) -> Result<Vec<PathBuf>> {
    let mut homes = vec![shared_codex_home.to_path_buf()];
    let config_groups_dir = shared_codex_home.join("config-groups");
    if !config_groups_dir.exists() {
        return Ok(homes);
    }

    for entry in stdfs::read_dir(config_groups_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            homes.push(entry.path());
        }
    }
    Ok(homes)
}

fn read_toml_file(path: &Path) -> Result<TomlValue> {
    let raw = stdfs::read_to_string(path)?;
    let value = raw.parse::<TomlValue>()?;
    ensure_table_root(path, &value)?;
    Ok(value)
}

fn ensure_table_root(path: &Path, value: &TomlValue) -> Result<()> {
    if value.as_table().is_some() {
        Ok(())
    } else {
        Err(eyre!("{} must contain a TOML table", path.display()))
    }
}

fn validate_base_config(config: &TomlValue) -> Result<()> {
    if projects_value(config).is_some() {
        return Err(eyre!(
            "{} cannot contain [projects]; move project trust entries to {}",
            CONFIG_BASE_FILE,
            CONFIG_LOCAL_FILE
        ));
    }
    Ok(())
}

fn validate_local_config(config: &TomlValue) -> Result<()> {
    let Some(table) = config.as_table() else {
        return Err(eyre!("{CONFIG_LOCAL_FILE} must contain a TOML table"));
    };
    let unexpected_keys = table
        .keys()
        .filter(|key| key.as_str() != "projects")
        .cloned()
        .collect::<Vec<_>>();
    if !unexpected_keys.is_empty() {
        return Err(eyre!(
            "{} can only contain [projects], found: {}",
            CONFIG_LOCAL_FILE,
            unexpected_keys.join(", ")
        ));
    }
    Ok(())
}

fn write_local_projects_overlay(config_home: &Path, projects: Option<TomlValue>) -> Result<()> {
    let local_path = config_home.join(CONFIG_LOCAL_FILE);
    let Some(projects) = projects else {
        if local_path.exists() {
            stdfs::remove_file(local_path)?;
        }
        return Ok(());
    };

    let mut table = toml::map::Map::new();
    table.insert("projects".to_string(), projects);
    write_toml_file(&local_path, &TomlValue::Table(table))
}

fn projects_value(config: &TomlValue) -> Option<&TomlValue> {
    config.as_table()?.get("projects")
}

fn take_projects_value(config: &mut TomlValue) -> Option<TomlValue> {
    config.as_table_mut()?.remove("projects")
}

fn deep_merge(base: &mut TomlValue, overlay: TomlValue) {
    match (base, overlay) {
        (TomlValue::Table(base_table), TomlValue::Table(overlay_table)) => {
            for (key, overlay_value) in overlay_table {
                match base_table.get_mut(&key) {
                    Some(base_value) => deep_merge(base_value, overlay_value),
                    None => {
                        base_table.insert(key, overlay_value);
                    }
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value;
        }
    }
}

fn write_toml_file(path: &Path, value: &TomlValue) -> Result<()> {
    fsutil::ensure_parent_dir(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| eyre!("Path has no parent: {}", path.display()))?;
    let mut temp = NamedTempFile::new_in(parent)?;
    let raw = toml::to_string_pretty(value)?;
    temp.write_all(raw.as_bytes())?;
    temp.flush()?;
    temp.persist(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn render_merges_base_with_local_projects() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(CONFIG_BASE_FILE),
            "model = \"gpt-5.4\"\n[profiles.fast]\nmodel = \"gpt-5.4-mini\"\n",
        )
        .unwrap();
        fs::write(
            dir.path().join(CONFIG_LOCAL_FILE),
            "[projects.\"/tmp/project\"]\ntrust_level = \"trusted\"\n",
        )
        .unwrap();

        render_config_home(dir.path()).unwrap();

        let rendered = fs::read_to_string(dir.path().join(CONFIG_FILE)).unwrap();
        assert!(rendered.contains("model = \"gpt-5.4\""));
        assert!(rendered.contains("[profiles.fast]"));
        assert!(rendered.contains("[projects.\"/tmp/project\"]"));
    }

    #[test]
    fn render_rejects_projects_in_base() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(CONFIG_BASE_FILE),
            "[projects.\"/tmp/project\"]\ntrust_level = \"trusted\"\n",
        )
        .unwrap();

        let err = render_config_home(dir.path()).unwrap_err();

        assert!(err.to_string().contains("cannot contain [projects]"));
    }

    #[test]
    fn render_rejects_non_projects_in_local_overlay() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_BASE_FILE), "model = \"gpt-5.4\"\n").unwrap();
        fs::write(
            dir.path().join(CONFIG_LOCAL_FILE),
            "model = \"gpt-5.4-mini\"\n",
        )
        .unwrap();

        let err = render_config_home(dir.path()).unwrap_err();

        assert!(err.to_string().contains("can only contain [projects]"));
    }

    #[test]
    fn sync_projects_extracts_only_projects_from_generated_config() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_BASE_FILE), "model = \"gpt-5.4\"\n").unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE),
            "model = \"mutated\"\n[projects.\"/tmp/project\"]\ntrust_level = \"trusted\"\n",
        )
        .unwrap();

        sync_projects_from_generated_config(dir.path()).unwrap();

        let local = fs::read_to_string(dir.path().join(CONFIG_LOCAL_FILE)).unwrap();
        assert!(local.contains("[projects.\"/tmp/project\"]"));
        assert!(!local.contains("mutated"));
    }

    #[test]
    fn migrate_splits_generated_config() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE),
            "model = \"gpt-5.4\"\n[projects.\"/tmp/project\"]\ntrust_level = \"trusted\"\n",
        )
        .unwrap();

        migrate_config_home(dir.path()).unwrap();

        let base = fs::read_to_string(dir.path().join(CONFIG_BASE_FILE)).unwrap();
        let local = fs::read_to_string(dir.path().join(CONFIG_LOCAL_FILE)).unwrap();
        assert!(base.contains("model = \"gpt-5.4\""));
        assert!(!base.contains("projects"));
        assert!(local.contains("[projects.\"/tmp/project\"]"));
    }
}
