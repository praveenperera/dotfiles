use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::{Parser, Subcommand};
use eyre::{eyre, Result, WrapErr};
use toml::{map::Map, Value};
use xshell::{cmd, Shell};

use crate::cmd::agent_target::AgentTarget;

#[derive(Debug, Clone, Parser)]
pub struct Mcp {
    #[command(subcommand)]
    pub subcommand: McpCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum McpCmd {
    /// Add project-local MCP servers to the current Git repo
    Add {
        /// MCP names to add. Opens a searchable multi-select picker when omitted
        mcps: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct McpEntry {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MergeStatus {
    Added,
    SkippedExisting,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpInstallSummary {
    pub added: Vec<String>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct McpServerSource {
    pub name: String,
    pub value: Value,
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = Mcp::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Mcp) -> Result<()> {
    match flags.subcommand {
        McpCmd::Add { mcps } => add_mcps(sh, &mcps),
    }
}

fn add_mcps(sh: &Shell, requested_mcps: &[String]) -> Result<()> {
    let project_mcps_dir = crate::dotfiles_dir()?.join("agents/project-mcps");
    let available_mcps = list_project_mcps(&project_mcps_dir)?;
    let selected_mcps = if requested_mcps.is_empty() {
        select_mcps(sh, &available_mcps)?
    } else {
        requested_mcps.to_vec()
    };

    if selected_mcps.is_empty() {
        println!("No MCPs selected");
        return Ok(());
    }

    let summary = add_mcps_by_name(sh, &selected_mcps)?;
    print_mcp_summary(&summary);

    Ok(())
}

pub fn add_mcps_by_name(sh: &Shell, selected_mcps: &[String]) -> Result<McpInstallSummary> {
    add_mcps_by_name_for_agent(sh, AgentTarget::Codex, selected_mcps)
}

pub fn add_mcps_by_name_for_agent(
    sh: &Shell,
    agent: AgentTarget,
    selected_mcps: &[String],
) -> Result<McpInstallSummary> {
    if selected_mcps.is_empty() {
        return Ok(McpInstallSummary {
            added: Vec::new(),
            skipped: Vec::new(),
        });
    }

    let project_mcps_dir = crate::dotfiles_dir()?.join("agents/project-mcps");
    let available_mcps = list_project_mcps(&project_mcps_dir)?;
    let sources = selected_mcps
        .iter()
        .map(|name| {
            let entry = available_mcps
                .iter()
                .find(|entry| entry.name == *name)
                .ok_or_else(|| eyre!("project MCP not found: {name}"))?;
            let snippet = read_mcp_snippet(entry)?;
            Ok(McpServerSource {
                name: snippet.name,
                value: snippet.value,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    add_mcp_servers_for_agent(sh, agent, sources)
}

pub fn project_mcp_names() -> Result<BTreeSet<String>> {
    let project_mcps_dir = crate::dotfiles_dir()?.join("agents/project-mcps");
    Ok(list_project_mcps(&project_mcps_dir)?
        .into_iter()
        .map(|mcp| mcp.name)
        .collect())
}

pub fn add_mcp_servers(sh: &Shell, servers: Vec<McpServerSource>) -> Result<McpInstallSummary> {
    add_mcp_servers_for_agent(sh, AgentTarget::Codex, servers)
}

pub fn add_mcp_servers_for_agent(
    sh: &Shell,
    agent: AgentTarget,
    servers: Vec<McpServerSource>,
) -> Result<McpInstallSummary> {
    if servers.is_empty() {
        return Ok(McpInstallSummary {
            added: Vec::new(),
            skipped: Vec::new(),
        });
    }

    let git_root = git_root(sh)?;
    match agent {
        AgentTarget::Codex => add_codex_mcp_servers(&git_root, servers),
        AgentTarget::Claude => add_claude_mcp_servers(&git_root, servers),
    }
}

fn add_codex_mcp_servers(
    git_root: &Path,
    servers: Vec<McpServerSource>,
) -> Result<McpInstallSummary> {
    let config_path = AgentTarget::Codex.project_mcp_config_path(git_root);
    let mut config = read_project_config(&config_path)?;
    let mut summary = McpInstallSummary {
        added: Vec::new(),
        skipped: Vec::new(),
    };

    for server in servers {
        match merge_mcp_server(&mut config, server.name.as_str(), server.value)? {
            MergeStatus::Added => summary.added.push(server.name),
            MergeStatus::SkippedExisting => summary.skipped.push(server.name),
        }
    }

    if !summary.added.is_empty() {
        write_project_config(&config_path, &config)?;
    }

    Ok(summary)
}

fn add_claude_mcp_servers(
    git_root: &Path,
    servers: Vec<McpServerSource>,
) -> Result<McpInstallSummary> {
    let config_path = AgentTarget::Claude.project_mcp_config_path(git_root);
    let mut config = read_claude_mcp_config(&config_path)?;
    let mut summary = McpInstallSummary {
        added: Vec::new(),
        skipped: Vec::new(),
    };

    for server in servers {
        let value = serde_json::to_value(&server.value)?;
        match merge_claude_mcp_server(&mut config, server.name.as_str(), value)? {
            MergeStatus::Added => summary.added.push(server.name),
            MergeStatus::SkippedExisting => summary.skipped.push(server.name),
        }
    }

    if !summary.added.is_empty() {
        write_claude_mcp_config(&config_path, &config)?;
    }

    Ok(summary)
}

fn print_mcp_summary(summary: &McpInstallSummary) {
    if !summary.added.is_empty() {
        println!("Added MCPs: {}", summary.added.join(", "));
    }
    if !summary.skipped.is_empty() {
        println!("Skipped existing MCPs: {}", summary.skipped.join(", "));
    }
}

fn git_root(sh: &Shell) -> Result<PathBuf> {
    let output = cmd!(sh, "git rev-parse --show-toplevel").read()?;
    Ok(PathBuf::from(output.trim()))
}

fn list_project_mcps(project_mcps_dir: &Path) -> Result<Vec<McpEntry>> {
    let mut mcps = Vec::new();

    let entries = match fs::read_dir(project_mcps_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(mcps),
        Err(err) => {
            return Err(err).wrap_err_with(|| {
                format!(
                    "failed to read project MCP directory: {}",
                    project_mcps_dir.display()
                )
            })
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

        mcps.push(McpEntry {
            name: name.to_string(),
            path,
        });
    }

    mcps.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(mcps)
}

fn select_mcps(sh: &Shell, available_mcps: &[McpEntry]) -> Result<Vec<String>> {
    if available_mcps.is_empty() {
        return Err(eyre!(
            "no project MCPs found in {}",
            crate::dotfiles_dir()?.join("agents/project-mcps").display()
        ));
    }

    if !crate::util::has_tool(sh, "fzf") {
        return Err(eyre!(
            "fzf is required for interactive MCP selection; install fzf or pass MCP names directly"
        ));
    }

    let input = available_mcps
        .iter()
        .map(|mcp| mcp.name.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let mut child = Command::new("fzf")
        .args(["--multi", "--prompt", "mcp> ", "--height=100%", "--no-sort"])
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

#[derive(Debug, Clone)]
struct McpSnippet {
    name: String,
    value: Value,
}

fn read_mcp_snippet(entry: &McpEntry) -> Result<McpSnippet> {
    let text = fs::read_to_string(&entry.path)
        .wrap_err_with(|| format!("failed to read MCP snippet: {}", entry.path.display()))?;
    parse_mcp_snippet(&entry.name, &text)
        .wrap_err_with(|| format!("invalid MCP snippet: {}", entry.path.display()))
}

fn parse_mcp_snippet(expected_name: &str, text: &str) -> Result<McpSnippet> {
    let value = text.parse::<Value>()?;
    let Value::Table(mut root) = value else {
        return Err(eyre!("MCP snippet root must be a TOML table"));
    };
    let Some(Value::Table(mcp_servers)) = root.remove("mcp_servers") else {
        return Err(eyre!("MCP snippet must contain an [mcp_servers] table"));
    };

    if mcp_servers.len() != 1 {
        return Err(eyre!(
            "MCP snippet must contain exactly one [mcp_servers.<name>] table"
        ));
    }

    let (name, value) = mcp_servers
        .into_iter()
        .next()
        .ok_or_else(|| eyre!("MCP snippet must contain one server"))?;

    if name != expected_name {
        return Err(eyre!(
            "MCP snippet file name {expected_name} does not match server name {name}"
        ));
    }

    Ok(McpSnippet { name, value })
}

fn read_project_config(config_path: &Path) -> Result<Value> {
    match fs::read_to_string(config_path) {
        Ok(text) => Ok(text.parse::<Value>()?),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(Value::Table(Map::new())),
        Err(err) => Err(err).wrap_err_with(|| {
            format!(
                "failed to read project Codex config: {}",
                config_path.display()
            )
        }),
    }
}

fn write_project_config(config_path: &Path, config: &Value) -> Result<()> {
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).wrap_err_with(|| {
            format!(
                "failed to create project Codex config directory: {}",
                parent.display()
            )
        })?;
    }
    fs::write(config_path, toml::to_string_pretty(config)?).wrap_err_with(|| {
        format!(
            "failed to write project Codex config: {}",
            config_path.display()
        )
    })?;
    Ok(())
}

fn read_claude_mcp_config(config_path: &Path) -> Result<serde_json::Value> {
    match fs::read_to_string(config_path) {
        Ok(text) => serde_json::from_str(&text)
            .wrap_err_with(|| format!("invalid Claude MCP config: {}", config_path.display())),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(err) => Err(err).wrap_err_with(|| {
            format!(
                "failed to read Claude MCP config: {}",
                config_path.display()
            )
        }),
    }
}

fn write_claude_mcp_config(config_path: &Path, config: &serde_json::Value) -> Result<()> {
    fs::write(
        config_path,
        format!("{}\n", serde_json::to_string_pretty(config)?),
    )
    .wrap_err_with(|| {
        format!(
            "failed to write Claude MCP config: {}",
            config_path.display()
        )
    })
}

fn merge_mcp_server(config: &mut Value, name: &str, server: Value) -> Result<MergeStatus> {
    let root = config
        .as_table_mut()
        .ok_or_else(|| eyre!("project Codex config root must be a TOML table"))?;
    let mcp_servers = root
        .entry("mcp_servers".to_string())
        .or_insert_with(|| Value::Table(Map::new()))
        .as_table_mut()
        .ok_or_else(|| eyre!("project Codex config mcp_servers entry must be a TOML table"))?;

    if mcp_servers.contains_key(name) {
        return Ok(MergeStatus::SkippedExisting);
    }

    mcp_servers.insert(name.to_string(), server);
    Ok(MergeStatus::Added)
}

fn merge_claude_mcp_server(
    config: &mut serde_json::Value,
    name: &str,
    server: serde_json::Value,
) -> Result<MergeStatus> {
    let root = config
        .as_object_mut()
        .ok_or_else(|| eyre!("Claude MCP config root must be a JSON object"))?;
    let mcp_servers = root
        .entry("mcpServers".to_string())
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| eyre!("Claude MCP config mcpServers entry must be a JSON object"))?;

    if mcp_servers.contains_key(name) {
        return Ok(MergeStatus::SkippedExisting);
    }

    mcp_servers.insert(name.to_string(), server);
    Ok(MergeStatus::Added)
}

#[cfg(test)]
mod tests {
    use super::{
        list_project_mcps, merge_claude_mcp_server, merge_mcp_server, parse_mcp_snippet,
        MergeStatus,
    };

    #[test]
    fn lists_project_mcp_snippets() {
        let dir = tempfile::tempdir().unwrap();
        let mcps_dir = dir.path().join("project-mcps");
        std::fs::create_dir_all(&mcps_dir).unwrap();
        std::fs::write(mcps_dir.join("beta.toml"), "").unwrap();
        std::fs::write(mcps_dir.join("alpha.toml"), "").unwrap();
        std::fs::write(mcps_dir.join("README.md"), "").unwrap();

        let mcps = list_project_mcps(&mcps_dir).unwrap();
        let names = mcps.into_iter().map(|mcp| mcp.name).collect::<Vec<_>>();

        assert_eq!(names, ["alpha", "beta"]);
    }

    #[test]
    fn parses_matching_mcp_snippet() {
        let snippet = parse_mcp_snippet(
            "xcodebuildmcp",
            r#"
                [mcp_servers.xcodebuildmcp]
                command = "npx"
                args = ["-y", "xcodebuildmcp@latest", "mcp"]
            "#,
        )
        .unwrap();

        assert_eq!(snippet.name, "xcodebuildmcp");
    }

    #[test]
    fn rejects_snippet_name_mismatch() {
        let err = parse_mcp_snippet(
            "xcodebuildmcp",
            r#"
                [mcp_servers.other]
                command = "npx"
            "#,
        )
        .unwrap_err();

        assert!(err.to_string().contains("does not match server name"));
    }

    #[test]
    fn adds_mcp_to_empty_config() {
        let mut config = toml::Value::Table(toml::map::Map::new());
        let server = "command = \"npx\"".parse::<toml::Value>().unwrap();

        let status = merge_mcp_server(&mut config, "xcodebuildmcp", server).unwrap();

        assert_eq!(status, MergeStatus::Added);
        assert!(config
            .get("mcp_servers")
            .and_then(|value| value.get("xcodebuildmcp"))
            .is_some());
    }

    #[test]
    fn preserves_unrelated_project_config() {
        let mut config = r#"
            model = "gpt-5.5"

            [projects."/tmp/example"]
            trust_level = "trusted"
        "#
        .parse::<toml::Value>()
        .unwrap();
        let server = "command = \"npx\"".parse::<toml::Value>().unwrap();

        merge_mcp_server(&mut config, "xcodebuildmcp", server).unwrap();

        assert_eq!(
            config.get("model").and_then(toml::Value::as_str),
            Some("gpt-5.5")
        );
        assert!(config
            .get("projects")
            .and_then(|value| value.get("/tmp/example"))
            .is_some());
    }

    #[test]
    fn skips_existing_mcp() {
        let mut config = r#"
            [mcp_servers.xcodebuildmcp]
            command = "existing"
        "#
        .parse::<toml::Value>()
        .unwrap();
        let server = "command = \"npx\"".parse::<toml::Value>().unwrap();

        let status = merge_mcp_server(&mut config, "xcodebuildmcp", server).unwrap();

        assert_eq!(status, MergeStatus::SkippedExisting);
        assert_eq!(
            config
                .get("mcp_servers")
                .and_then(|value| value.get("xcodebuildmcp"))
                .and_then(|value| value.get("command"))
                .and_then(toml::Value::as_str),
            Some("existing")
        );
    }

    #[test]
    fn adds_mcp_to_empty_claude_config() {
        let mut config = serde_json::json!({});
        let server = serde_json::json!({
            "command": "npx",
            "args": ["-y", "xcodebuildmcp@latest", "mcp"],
        });

        let status = merge_claude_mcp_server(&mut config, "xcodebuildmcp", server).unwrap();

        assert_eq!(status, MergeStatus::Added);
        assert!(config
            .get("mcpServers")
            .and_then(|value| value.get("xcodebuildmcp"))
            .is_some());
    }

    #[test]
    fn skips_existing_claude_mcp() {
        let mut config = serde_json::json!({
            "mcpServers": {
                "xcodebuildmcp": {
                    "command": "existing"
                }
            }
        });
        let server = serde_json::json!({ "command": "npx" });

        let status = merge_claude_mcp_server(&mut config, "xcodebuildmcp", server).unwrap();

        assert_eq!(status, MergeStatus::SkippedExisting);
        assert_eq!(
            config
                .get("mcpServers")
                .and_then(|value| value.get("xcodebuildmcp"))
                .and_then(|value| value.get("command"))
                .and_then(serde_json::Value::as_str),
            Some("existing")
        );
    }
}
