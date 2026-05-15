use std::collections::{BTreeMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;

use clap::{Parser, Subcommand};
use eyre::{eyre, Result, WrapErr};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use xshell::Shell;

#[derive(Debug, Clone, Parser)]
pub struct Pack {
    #[command(subcommand)]
    pub subcommand: PackCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum PackCmd {
    /// Add project-local skills and MCPs from reusable packs
    Add {
        /// Pack names to add. Opens a searchable multi-select picker when omitted
        packs: Vec<String>,
    },

    /// Refresh registered project pack links and plugin MCPs
    Refresh {
        /// Refresh every registered project instead of only the current repo
        #[arg(long)]
        all: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PackEntry {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PackDefinition {
    #[serde(default)]
    description: String,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    mcps: Vec<String>,
    #[serde(default, alias = "plugins")]
    plugin_sources: Vec<String>,
    #[serde(default)]
    packs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExpandedPack {
    skills: Vec<String>,
    mcps: Vec<String>,
    plugin_sources: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct ResolvedPluginSources {
    skills: Vec<crate::cmd::skill::SkillSource>,
    mcps: Vec<crate::cmd::mcp::McpServerSource>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
struct PackProjectRegistry {
    #[serde(default)]
    projects: BTreeMap<String, PackProject>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
struct PackProject {
    packs: Vec<String>,
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = Pack::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Pack) -> Result<()> {
    match flags.subcommand {
        PackCmd::Add { packs } => add_packs(sh, &packs),
        PackCmd::Refresh { all } => refresh_packs(sh, all),
    }
}

fn add_packs(sh: &Shell, requested_packs: &[String]) -> Result<()> {
    let pack_dir = crate::dotfiles_dir()?.join("agents/skill-packs");
    let available_packs = list_packs(&pack_dir)?;
    let selected_packs = if requested_packs.is_empty() {
        select_packs(sh, &available_packs)?
    } else {
        requested_packs.to_vec()
    };

    if selected_packs.is_empty() {
        println!("No packs selected");
        return Ok(());
    }

    install_packs(sh, &available_packs, &selected_packs, "Added packs")?;
    register_current_project(sh, &selected_packs)?;

    Ok(())
}

fn refresh_packs(sh: &Shell, all: bool) -> Result<()> {
    if all {
        return refresh_registered_packs(sh);
    }

    let git_root = git_root(sh)?;
    let registry_path = pack_registry_path()?;
    let registry = load_pack_registry(&registry_path)?;
    let Some(project) = registry.projects.get(&path_key(&git_root)) else {
        return Err(eyre!(
            "project is not registered for pack refresh: {}",
            git_root.display()
        ));
    };
    let available_packs = list_packs(&pack_dir()?)?;
    install_packs(sh, &available_packs, &project.packs, "Refreshed packs")
}

pub fn refresh_registered_packs(_sh: &Shell) -> Result<()> {
    let registry_path = pack_registry_path()?;
    let mut registry = load_pack_registry(&registry_path)?;
    if registry.projects.is_empty() {
        return Ok(());
    }

    let available_packs = list_packs(&pack_dir()?)?;
    let refresh_targets = registry
        .projects
        .iter()
        .filter_map(|(project_root, project)| {
            let project_root_path = PathBuf::from(project_root);
            project_root_path.is_dir().then(|| {
                (
                    project_root.clone(),
                    project_root_path,
                    project.packs.clone(),
                )
            })
        })
        .collect::<Vec<_>>();

    let failures = refresh_targets_concurrently(refresh_targets, available_packs);

    for project_root in prune_missing_projects(&mut registry) {
        println!("Removing missing pack project: {project_root}");
    }
    save_pack_registry(&registry_path, &registry)?;

    if failures.is_empty() {
        return Ok(());
    }

    Err(eyre!(
        "failed to refresh some pack projects:\n{}",
        failures.join("\n")
    ))
}

fn refresh_targets_concurrently(
    refresh_targets: Vec<(String, PathBuf, Vec<String>)>,
    available_packs: Vec<PackEntry>,
) -> Vec<String> {
    thread::scope(|scope| {
        let handles = refresh_targets
            .into_iter()
            .map(|(project_root, project_root_path, packs)| {
                let available_packs = available_packs.clone();
                scope.spawn(move || {
                    println!("Refreshing packs in {project_root}");
                    let result = (|| {
                        let sh = Shell::new()?;
                        let _dir = sh.push_dir(&project_root_path);
                        install_packs(&sh, &available_packs, &packs, "Refreshed packs")
                    })();
                    result.map_err(|err| format!("{project_root}: {err}"))
                })
            })
            .collect::<Vec<_>>();

        handles
            .into_iter()
            .filter_map(|handle| match handle.join() {
                Ok(Ok(())) => None,
                Ok(Err(err)) => Some(err),
                Err(_) => Some("pack refresh worker panicked".to_string()),
            })
            .collect()
    })
}

fn install_packs(
    sh: &Shell,
    available_packs: &[PackEntry],
    selected_packs: &[String],
    summary_label: &str,
) -> Result<()> {
    let definitions = load_pack_definitions(available_packs)?;
    let expanded = expand_packs(&definitions, selected_packs)?;
    validate_expanded_pack(&expanded)?;
    let plugin_sources = resolve_plugin_sources(&expanded.plugin_sources)?;

    let skill_summary = crate::cmd::skill::add_skills_by_name(sh, &expanded.skills)?;
    let plugin_skill_summary = crate::cmd::skill::add_skill_sources(sh, &plugin_sources.skills)?;
    let mcp_summary = crate::cmd::mcp::add_mcps_by_name(sh, &expanded.mcps)?;
    let plugin_mcp_summary = crate::cmd::mcp::add_mcp_servers(sh, plugin_sources.mcps)?;

    println!("{summary_label}: {}", selected_packs.join(", "));
    if !skill_summary.linked.is_empty() {
        println!("Linked skills: {}", skill_summary.linked.join(", "));
    }
    if !plugin_skill_summary.linked.is_empty() {
        println!(
            "Linked plugin skills: {}",
            plugin_skill_summary.linked.join(", ")
        );
    }
    if !skill_summary.skipped.is_empty() {
        println!(
            "Skipped existing skills: {}",
            skill_summary.skipped.join(", ")
        );
    }
    if !plugin_skill_summary.skipped.is_empty() {
        println!(
            "Skipped existing plugin skills: {}",
            plugin_skill_summary.skipped.join(", ")
        );
    }
    if !mcp_summary.added.is_empty() {
        println!("Added MCPs: {}", mcp_summary.added.join(", "));
    }
    if !plugin_mcp_summary.added.is_empty() {
        println!("Added plugin MCPs: {}", plugin_mcp_summary.added.join(", "));
    }
    if !mcp_summary.skipped.is_empty() {
        println!("Skipped existing MCPs: {}", mcp_summary.skipped.join(", "));
    }
    if !plugin_mcp_summary.skipped.is_empty() {
        println!(
            "Skipped existing plugin MCPs: {}",
            plugin_mcp_summary.skipped.join(", ")
        );
    }

    Ok(())
}

fn register_current_project(sh: &Shell, packs: &[String]) -> Result<()> {
    let git_root = git_root(sh)?;
    let registry_path = pack_registry_path()?;
    let mut registry = load_pack_registry(&registry_path)?;
    register_project(&mut registry, &git_root, packs);
    save_pack_registry(&registry_path, &registry)
}

fn register_project(registry: &mut PackProjectRegistry, project_root: &Path, packs: &[String]) {
    let project = registry.projects.entry(path_key(project_root)).or_default();
    append_unique(&mut project.packs, packs);
}

fn prune_missing_projects(registry: &mut PackProjectRegistry) -> Vec<String> {
    let missing_projects = registry
        .projects
        .keys()
        .filter(|project_root| !Path::new(project_root.as_str()).is_dir())
        .cloned()
        .collect::<Vec<_>>();

    for project_root in &missing_projects {
        registry.projects.remove(project_root);
    }

    missing_projects
}

fn pack_dir() -> Result<PathBuf> {
    Ok(crate::dotfiles_dir()?.join("agents/skill-packs"))
}

fn pack_registry_path() -> Result<PathBuf> {
    Ok(crate::fsutil::home_dir()?.join(".local/state/cmd/pack-projects.toml"))
}

fn load_pack_registry(path: &Path) -> Result<PackProjectRegistry> {
    match fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text)
            .wrap_err_with(|| format!("invalid pack project registry: {}", path.display())),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(PackProjectRegistry::default()),
        Err(err) => Err(err)
            .wrap_err_with(|| format!("failed to read pack project registry: {}", path.display())),
    }
}

fn save_pack_registry(path: &Path, registry: &PackProjectRegistry) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).wrap_err_with(|| {
            format!(
                "failed to create pack project registry directory: {}",
                parent.display()
            )
        })?;
    }

    fs::write(path, toml::to_string_pretty(registry)?)
        .wrap_err_with(|| format!("failed to write pack project registry: {}", path.display()))
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn git_root(sh: &Shell) -> Result<PathBuf> {
    let output = xshell::cmd!(sh, "git rev-parse --show-toplevel").read()?;
    Ok(PathBuf::from(output.trim()))
}

fn list_packs(pack_dir: &Path) -> Result<Vec<PackEntry>> {
    let mut packs = Vec::new();

    let entries = match fs::read_dir(pack_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(packs),
        Err(err) => {
            return Err(err)
                .wrap_err_with(|| format!("failed to read pack directory: {}", pack_dir.display()))
        }
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("toml") {
            continue;
        }

        let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };

        packs.push(PackEntry {
            name: name.to_string(),
            path,
        });
    }

    packs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(packs)
}

fn select_packs(sh: &Shell, available_packs: &[PackEntry]) -> Result<Vec<String>> {
    if available_packs.is_empty() {
        return Err(eyre!(
            "no packs found in {}",
            crate::dotfiles_dir()?.join("agents/skill-packs").display()
        ));
    }

    if !crate::util::has_tool(sh, "fzf") {
        return Err(eyre!(
            "fzf is required for interactive pack selection; install fzf or pass pack names directly"
        ));
    }

    let input = available_packs
        .iter()
        .map(|pack| pack.name.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let mut child = Command::new("fzf")
        .args([
            "--multi",
            "--prompt",
            "pack> ",
            "--height=100%",
            "--no-sort",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .wrap_err("failed to start fzf")?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write as _;
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn load_pack_definitions(entries: &[PackEntry]) -> Result<BTreeMap<String, PackDefinition>> {
    let mut definitions = BTreeMap::new();

    for entry in entries {
        let text = fs::read_to_string(&entry.path)
            .wrap_err_with(|| format!("failed to read pack: {}", entry.path.display()))?;
        let definition = toml::from_str::<PackDefinition>(&text)
            .wrap_err_with(|| format!("invalid pack: {}", entry.path.display()))?;
        definitions.insert(entry.name.clone(), definition);
    }

    Ok(definitions)
}

fn expand_packs(
    definitions: &BTreeMap<String, PackDefinition>,
    selected_packs: &[String],
) -> Result<ExpandedPack> {
    let mut expanded = ExpandedPack::default();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();
    let mut stack = Vec::new();

    for pack in selected_packs {
        expand_pack(
            definitions,
            pack,
            &mut expanded,
            &mut visited,
            &mut visiting,
            &mut stack,
        )?;
    }

    Ok(expanded)
}

fn expand_pack(
    definitions: &BTreeMap<String, PackDefinition>,
    name: &str,
    expanded: &mut ExpandedPack,
    visited: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
    stack: &mut Vec<String>,
) -> Result<()> {
    if visited.contains(name) {
        return Ok(());
    }
    if visiting.contains(name) {
        stack.push(name.to_string());
        return Err(eyre!("pack cycle detected: {}", stack.join(" -> ")));
    }

    let definition = definitions
        .get(name)
        .ok_or_else(|| eyre!("pack not found: {name}"))?;

    visiting.insert(name.to_string());
    stack.push(name.to_string());

    for nested in &definition.packs {
        expand_pack(definitions, nested, expanded, visited, visiting, stack)?;
    }
    append_unique(&mut expanded.skills, &definition.skills);
    append_unique(&mut expanded.mcps, &definition.mcps);
    append_unique(&mut expanded.plugin_sources, &definition.plugin_sources);

    stack.pop();
    visiting.remove(name);
    visited.insert(name.to_string());

    Ok(())
}

fn append_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}

fn validate_expanded_pack(expanded: &ExpandedPack) -> Result<()> {
    let skill_names = crate::cmd::skill::project_skill_names()?;
    let mcp_names = crate::cmd::mcp::project_mcp_names()?;

    for skill in &expanded.skills {
        if !skill_names.contains(skill) {
            return Err(eyre!("project skill not found: {skill}"));
        }
    }

    for mcp in &expanded.mcps {
        if !mcp_names.contains(mcp) {
            return Err(eyre!("project MCP not found: {mcp}"));
        }
    }

    Ok(())
}

fn resolve_plugin_sources(plugin_ids: &[String]) -> Result<ResolvedPluginSources> {
    let mut resolved = ResolvedPluginSources::default();

    for plugin_id in plugin_ids {
        let plugin = resolve_installed_plugin(plugin_id)?;
        let plugin_skills = plugin_skill_sources(plugin_id, &plugin)?;
        let plugin_mcps = plugin_mcp_sources(&plugin)?;
        resolved.skills.extend(plugin_skills);
        resolved.mcps.extend(plugin_mcps);
    }

    Ok(resolved)
}

#[derive(Debug, Clone)]
struct InstalledPlugin {
    root: PathBuf,
    manifest: JsonValue,
}

fn resolve_installed_plugin(plugin_id: &str) -> Result<InstalledPlugin> {
    let (name, marketplace) = plugin_id
        .split_once('@')
        .ok_or_else(|| eyre!("plugin source must be name@marketplace: {plugin_id}"))?;
    let plugin_root = codex_home()?
        .join("plugins/cache")
        .join(marketplace)
        .join(name);
    let mut candidates = Vec::new();

    for entry in fs::read_dir(&plugin_root)
        .wrap_err_with(|| format!("plugin source is not installed: {plugin_id}"))?
    {
        let entry = entry?;
        let path = entry.path();
        let manifest_path = path.join(".codex-plugin/plugin.json");
        if !manifest_path.is_file() {
            continue;
        }
        let modified = entry.metadata()?.modified()?;
        candidates.push((modified, path, manifest_path));
    }

    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let Some((_, root, manifest_path)) = candidates.into_iter().next() else {
        return Err(eyre!("plugin source has no cached installs: {plugin_id}"));
    };
    let manifest = read_json_file(&manifest_path)?;

    Ok(InstalledPlugin { root, manifest })
}

fn plugin_skill_sources(
    plugin_id: &str,
    plugin: &InstalledPlugin,
) -> Result<Vec<crate::cmd::skill::SkillSource>> {
    let Some(skills_path) = plugin.manifest.get("skills").and_then(JsonValue::as_str) else {
        return Ok(Vec::new());
    };
    let skills_dir = resolve_plugin_relative_path(&plugin.root, skills_path);
    let stable_dir = codex_home()?
        .join("plugin-skill-links")
        .join(plugin_id.replace('/', "__"));
    let mut skills = Vec::new();

    for entry in fs::read_dir(&skills_dir)
        .wrap_err_with(|| format!("failed to read plugin skills: {}", skills_dir.display()))?
    {
        let entry = entry?;
        let source = entry.path();
        if !source.join("SKILL.md").is_file() {
            continue;
        }
        let Some(name) = source.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let stable_source = stable_dir.join(name);
        refresh_stable_symlink(&source, &stable_source)?;
        skills.push(crate::cmd::skill::SkillSource {
            name: name.to_string(),
            path: stable_source,
            refresh_existing_symlink: true,
        });
    }

    skills.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(skills)
}

fn plugin_mcp_sources(plugin: &InstalledPlugin) -> Result<Vec<crate::cmd::mcp::McpServerSource>> {
    let Some(mcp_path) = plugin
        .manifest
        .get("mcpServers")
        .and_then(JsonValue::as_str)
    else {
        return Ok(Vec::new());
    };
    let mcp_path = resolve_plugin_relative_path(&plugin.root, mcp_path);
    let json = read_json_file(&mcp_path)?;
    let Some(servers) = json.get("mcpServers").and_then(JsonValue::as_object) else {
        return Err(eyre!(
            "plugin MCP file must contain an mcpServers object: {}",
            mcp_path.display()
        ));
    };

    let mut sources = Vec::new();
    for (name, server) in servers {
        sources.push(crate::cmd::mcp::McpServerSource {
            name: name.clone(),
            value: toml::Value::try_from(server.clone())?,
        });
    }
    sources.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(sources)
}

fn read_json_file(path: &Path) -> Result<JsonValue> {
    let text =
        fs::read_to_string(path).wrap_err_with(|| format!("failed to read: {}", path.display()))?;
    serde_json::from_str(&text).wrap_err_with(|| format!("invalid JSON: {}", path.display()))
}

fn resolve_plugin_relative_path(root: &Path, relative_path: &str) -> PathBuf {
    let path = Path::new(relative_path);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    root.join(path)
}

fn codex_home() -> Result<PathBuf> {
    let codex_home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or(crate::fsutil::home_dir()?.join(".codex"));

    if let Ok(plugins_target) = fs::read_link(codex_home.join("plugins")) {
        if let Some(parent) = plugins_target.parent() {
            return Ok(parent.to_path_buf());
        }
    }

    Ok(codex_home)
}

fn refresh_stable_symlink(source: &Path, target: &Path) -> Result<()> {
    static PLUGIN_LINK_REFRESH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    let _guard = PLUGIN_LINK_REFRESH_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| eyre!("plugin skill link refresh lock is poisoned"))?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::fsutil::remove_existing_path(target)?;
    create_symlink(source, target)
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
    use super::{
        expand_packs, list_packs, load_pack_definitions, load_pack_registry,
        prune_missing_projects, register_project, save_pack_registry, ExpandedPack, PackEntry,
        PackProject, PackProjectRegistry,
    };
    use std::collections::BTreeMap;

    #[test]
    fn lists_pack_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("web.toml"), "").unwrap();
        std::fs::write(dir.path().join("rust.toml"), "").unwrap();
        std::fs::write(dir.path().join("README.md"), "").unwrap();

        let packs = list_packs(dir.path()).unwrap();
        let names = packs.into_iter().map(|pack| pack.name).collect::<Vec<_>>();

        assert_eq!(names, ["rust", "web"]);
    }

    #[test]
    fn loads_pack_definitions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("web.toml");
        std::fs::write(
            &path,
            r#"
                description = "Web"
                skills = ["svelte"]
                mcps = ["xcodebuildmcp"]
                plugin_sources = ["build-ios-apps@openai-curated"]
                packs = ["cloud"]
            "#,
        )
        .unwrap();

        let definitions = load_pack_definitions(&[PackEntry {
            name: "web".to_string(),
            path,
        }])
        .unwrap();
        let web = definitions.get("web").unwrap();

        assert_eq!(web.skills, ["svelte"]);
        assert_eq!(web.mcps, ["xcodebuildmcp"]);
        assert_eq!(web.plugin_sources, ["build-ios-apps@openai-curated"]);
        assert_eq!(web.packs, ["cloud"]);
    }

    #[test]
    fn expands_nested_packs_and_dedupes() {
        let definitions = BTreeMap::from([
            (
                "cloud".to_string(),
                super::PackDefinition {
                    skills: vec!["cloudflare".to_string()],
                    ..Default::default()
                },
            ),
            (
                "web".to_string(),
                super::PackDefinition {
                    skills: vec!["svelte".to_string(), "cloudflare".to_string()],
                    plugin_sources: vec!["build-ios-apps@openai-curated".to_string()],
                    packs: vec!["cloud".to_string()],
                    ..Default::default()
                },
            ),
        ]);

        let expanded = expand_packs(&definitions, &["web".to_string()]).unwrap();

        assert_eq!(
            expanded,
            ExpandedPack {
                skills: vec!["cloudflare".to_string(), "svelte".to_string()],
                plugin_sources: vec!["build-ios-apps@openai-curated".to_string()],
                ..Default::default()
            }
        );
    }

    #[test]
    fn errors_for_missing_pack() {
        let err = expand_packs(&BTreeMap::new(), &["missing".to_string()]).unwrap_err();

        assert!(err.to_string().contains("pack not found: missing"));
    }

    #[test]
    fn errors_for_cycles() {
        let definitions = BTreeMap::from([
            (
                "one".to_string(),
                super::PackDefinition {
                    packs: vec!["two".to_string()],
                    ..Default::default()
                },
            ),
            (
                "two".to_string(),
                super::PackDefinition {
                    packs: vec!["one".to_string()],
                    ..Default::default()
                },
            ),
        ]);

        let err = expand_packs(&definitions, &["one".to_string()]).unwrap_err();

        assert!(err.to_string().contains("pack cycle detected"));
    }

    #[test]
    fn loads_missing_pack_registry_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let registry = load_pack_registry(&dir.path().join("pack-projects.toml")).unwrap();

        assert!(registry.projects.is_empty());
    }

    #[test]
    fn saves_and_loads_pack_registry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state/cmd/pack-projects.toml");
        let registry = PackProjectRegistry {
            projects: BTreeMap::from([(
                "/tmp/project".to_string(),
                PackProject {
                    packs: vec!["native".to_string()],
                },
            )]),
        };

        save_pack_registry(&path, &registry).unwrap();
        let loaded = load_pack_registry(&path).unwrap();

        assert_eq!(loaded, registry);
    }

    #[test]
    fn registers_project_packs_without_duplicates() {
        let mut registry = PackProjectRegistry::default();
        let project_root = std::path::Path::new("/tmp/project");

        register_project(
            &mut registry,
            project_root,
            &["native".to_string(), "web".to_string()],
        );
        register_project(
            &mut registry,
            project_root,
            &["native".to_string(), "cli".to_string()],
        );

        assert_eq!(
            registry.projects["/tmp/project"].packs,
            ["native", "web", "cli"]
        );
    }

    #[test]
    fn prunes_missing_pack_projects() {
        let dir = tempfile::tempdir().unwrap();
        let existing_project = dir.path().join("existing");
        std::fs::create_dir(&existing_project).unwrap();
        let missing_project = dir.path().join("missing");
        let mut registry = PackProjectRegistry {
            projects: BTreeMap::from([
                (
                    existing_project.to_string_lossy().into_owned(),
                    PackProject {
                        packs: vec!["native".to_string()],
                    },
                ),
                (
                    missing_project.to_string_lossy().into_owned(),
                    PackProject {
                        packs: vec!["web".to_string()],
                    },
                ),
            ]),
        };

        let pruned = prune_missing_projects(&mut registry);

        assert_eq!(pruned, [missing_project.to_string_lossy().into_owned()]);
        assert!(registry
            .projects
            .contains_key(existing_project.to_string_lossy().as_ref()));
        assert!(!registry
            .projects
            .contains_key(missing_project.to_string_lossy().as_ref()));
    }
}
