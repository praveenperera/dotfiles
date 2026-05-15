use std::collections::{BTreeMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::{Parser, Subcommand};
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use xshell::Shell;

#[derive(Debug, Clone, Parser)]
pub struct Pack {
    #[command(subcommand)]
    pub subcommand: PackCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum PackCmd {
    /// Add project-local skills, MCPs, and plugins from reusable packs
    Add {
        /// Pack names to add. Opens a searchable multi-select picker when omitted
        packs: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PackEntry {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
struct PackDefinition {
    #[serde(default)]
    description: String,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    mcps: Vec<String>,
    #[serde(default)]
    plugins: Vec<String>,
    #[serde(default)]
    packs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExpandedPack {
    skills: Vec<String>,
    mcps: Vec<String>,
    plugins: Vec<String>,
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = Pack::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Pack) -> Result<()> {
    match flags.subcommand {
        PackCmd::Add { packs } => add_packs(sh, &packs),
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

    let definitions = load_pack_definitions(&available_packs)?;
    let expanded = expand_packs(&definitions, &selected_packs)?;
    validate_expanded_pack(&expanded)?;

    let skill_summary = crate::cmd::skill::add_skills_by_name(sh, &expanded.skills)?;
    let mcp_summary = crate::cmd::mcp::add_mcps_by_name(sh, &expanded.mcps)?;
    let plugin_summary = crate::cmd::mcp::enable_plugins_by_name(sh, &expanded.plugins)?;

    println!("Added packs: {}", selected_packs.join(", "));
    if !skill_summary.linked.is_empty() {
        println!("Linked skills: {}", skill_summary.linked.join(", "));
    }
    if !skill_summary.skipped.is_empty() {
        println!(
            "Skipped existing skills: {}",
            skill_summary.skipped.join(", ")
        );
    }
    if !mcp_summary.added.is_empty() {
        println!("Added MCPs: {}", mcp_summary.added.join(", "));
    }
    if !mcp_summary.skipped.is_empty() {
        println!("Skipped existing MCPs: {}", mcp_summary.skipped.join(", "));
    }
    if !plugin_summary.enabled.is_empty() {
        println!("Enabled plugins: {}", plugin_summary.enabled.join(", "));
    }
    if !plugin_summary.skipped.is_empty() {
        println!(
            "Skipped enabled plugins: {}",
            plugin_summary.skipped.join(", ")
        );
    }

    Ok(())
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
    append_unique(&mut expanded.plugins, &definition.plugins);

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

#[cfg(test)]
mod tests {
    use super::{expand_packs, list_packs, load_pack_definitions, ExpandedPack, PackEntry};
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
                plugins = ["build-ios-apps@openai-curated"]
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
        assert_eq!(web.plugins, ["build-ios-apps@openai-curated"]);
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
}
