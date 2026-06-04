use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Args;
use colored::Colorize;
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use xshell::Shell;

use crate::{command_exists, fsutil};

#[derive(Debug, Clone, Args)]
pub struct Install {
    /// Tool name to install
    pub tool: String,

    /// GitHub release tag to install (default: latest release)
    #[arg(long)]
    pub tag: Option<String>,

    /// Directory where the binary should be installed
    #[arg(long, value_name = "DIR")]
    pub to: Option<PathBuf>,

    /// Overwrite an existing binary
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Debug, Clone, Copy)]
struct GithubReleaseTool {
    name: &'static str,
    repo: &'static str,
    crate_name: &'static str,
}

#[derive(Debug, Deserialize)]
struct LatestRelease {
    tag_name: String,
}

const GITHUB_RELEASE_TOOLS: &[GithubReleaseTool] = &[
    GithubReleaseTool {
        name: "rustywind",
        repo: "avencera/rustywind",
        crate_name: "rustywind",
    },
    GithubReleaseTool {
        name: "smrze",
        repo: "avencera/smrze",
        crate_name: "smrze",
    },
];

pub fn run_with_flags(sh: &Shell, flags: Install) -> Result<()> {
    let tool = resolve_tool(&flags.tool)?;
    let target = current_release_target()?;
    let tag = match flags.tag {
        Some(tag) => tag,
        None => crate::runtime::block_on(fetch_latest_release_tag(tool.repo))??,
    };
    let dest_dir = flags.to.unwrap_or(fsutil::home_dir()?.join(".local/bin"));

    install_github_release(sh, tool, &tag, &target, &dest_dir, flags.force)
}

fn install_github_release(
    sh: &Shell,
    tool: GithubReleaseTool,
    tag: &str,
    target: &str,
    dest_dir: &Path,
    force: bool,
) -> Result<()> {
    ensure_release_dependencies(sh)?;

    let dest = dest_dir.join(tool.crate_name);
    if path_exists_or_symlink(&dest) && !force {
        return Err(eyre!(
            "{} already exists in {}, use --force to overwrite it",
            tool.crate_name,
            dest_dir.display()
        ));
    }

    let url = release_asset_url(tool.repo, tool.crate_name, tag, target);
    let tmp_dir = sh.create_temp_dir()?;
    let archive_path = tmp_dir
        .path()
        .join(format!("{}-{tag}-{target}.tar.gz", tool.crate_name));

    println!(
        "{} {} ({tag}, {target})",
        "Installing".green(),
        tool.name.blue()
    );
    download_release_asset(&url, &archive_path)?;
    extract_archive(&archive_path, tmp_dir.path())?;

    let binary = find_release_binary(tmp_dir.path(), tool.crate_name)?;
    fs::create_dir_all(dest_dir)
        .wrap_err_with(|| format!("failed to create install directory: {}", dest_dir.display()))?;

    if force {
        fsutil::remove_existing_path(&dest)?;
    }

    fs::copy(&binary, &dest).wrap_err_with(|| {
        format!(
            "failed to install {} from {} to {}",
            tool.crate_name,
            binary.display(),
            dest.display()
        )
    })?;
    let mut permissions = fs::metadata(&dest)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&dest, permissions)?;

    println!("{} {}", "Installed".green(), dest.display());
    Ok(())
}

fn resolve_tool(name: &str) -> Result<GithubReleaseTool> {
    GITHUB_RELEASE_TOOLS
        .iter()
        .find(|tool| tool.name == name)
        .copied()
        .ok_or_else(|| {
            let known_tools = GITHUB_RELEASE_TOOLS
                .iter()
                .map(|tool| tool.name)
                .collect::<Vec<_>>()
                .join(", ");
            eyre!("unknown install tool: {name}; known tools: {known_tools}")
        })
}

fn path_exists_or_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn current_release_target() -> Result<String> {
    release_target(env::consts::OS, env::consts::ARCH)
}

fn release_target(os: &str, arch: &str) -> Result<String> {
    let target = match (os, arch) {
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        _ => return Err(eyre!("unsupported release target: {arch}-{os}")),
    };

    Ok(target.to_string())
}

fn release_asset_url(repo: &str, crate_name: &str, tag: &str, target: &str) -> String {
    format!("https://github.com/{repo}/releases/download/{tag}/{crate_name}-{tag}-{target}.tar.gz")
}

async fn fetch_latest_release_tag(repo: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("cmd-install")
        .build()?;
    let mut request = client.get(format!(
        "https://api.github.com/repos/{repo}/releases/latest"
    ));

    if let Some(token) = github_token() {
        request = request.header("Authorization", format!("Bearer {token}"));
    }

    let response = request.send().await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await?;
        return Err(eyre!(
            "failed to fetch latest release for {repo}: HTTP {status}: {body}"
        ));
    }

    let latest: LatestRelease = response.json().await?;
    Ok(latest.tag_name)
}

fn ensure_release_dependencies(sh: &Shell) -> Result<()> {
    for command in ["curl", "tar"] {
        if !command_exists(sh, command) {
            return Err(eyre!("need {command} (command not found)"));
        }
    }

    Ok(())
}

fn download_release_asset(url: &str, archive_path: &Path) -> Result<()> {
    let mut command = Command::new("curl");
    command
        .arg("--fail")
        .arg("--location")
        .arg("--silent")
        .arg("--show-error");

    if let Some(token) = github_token() {
        command
            .arg("--header")
            .arg(format!("Authorization: Bearer {token}"));
    }

    let status = command
        .arg("-o")
        .arg(archive_path)
        .arg(url)
        .status()
        .wrap_err("failed to run curl")?;

    if !status.success() {
        return Err(eyre!("failed to download release asset: {url}"));
    }

    Ok(())
}

fn extract_archive(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let status = Command::new("tar")
        .arg("-C")
        .arg(dest_dir)
        .arg("-xzf")
        .arg(archive_path)
        .status()
        .wrap_err("failed to run tar")?;

    if !status.success() {
        return Err(eyre!(
            "failed to extract release archive: {}",
            archive_path.display()
        ));
    }

    Ok(())
}

fn find_release_binary(root: &Path, crate_name: &str) -> Result<PathBuf> {
    let direct_binary = root.join(crate_name);
    if direct_binary.is_file() {
        return Ok(direct_binary);
    }

    let mut candidates = Vec::new();
    collect_executable_files(root, &mut candidates)?;

    if let Some(binary) = candidates
        .iter()
        .find(|path| path.file_name().is_some_and(|name| name == crate_name))
    {
        return Ok(binary.clone());
    }

    match candidates.as_slice() {
        [binary] => Ok(binary.clone()),
        [] => Err(eyre!(
            "release archive did not contain an executable binary named {crate_name}"
        )),
        _ => Err(eyre!(
            "release archive contained multiple executables; expected one named {crate_name}"
        )),
    }
}

fn collect_executable_files(dir: &Path, candidates: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            collect_executable_files(&path, candidates)?;
        } else if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 {
            candidates.push(path);
        }
    }

    Ok(())
}

fn github_token() -> Option<String> {
    env::var("GITHUB_TOKEN")
        .ok()
        .filter(|token| !token.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::{release_asset_url, release_target, resolve_tool};

    #[test]
    fn resolves_known_tools() {
        let smrze = resolve_tool("smrze").unwrap();
        assert_eq!(smrze.repo, "avencera/smrze");

        let rustywind = resolve_tool("rustywind").unwrap();
        assert_eq!(rustywind.repo, "avencera/rustywind");
    }

    #[test]
    fn builds_rustywind_style_release_asset_url() {
        let url = release_asset_url(
            "avencera/rustywind",
            "rustywind",
            "v0.7.0",
            "aarch64-apple-darwin",
        );

        assert_eq!(
            url,
            "https://github.com/avencera/rustywind/releases/download/v0.7.0/rustywind-v0.7.0-aarch64-apple-darwin.tar.gz"
        );
    }

    #[test]
    fn maps_supported_release_targets() {
        assert_eq!(
            release_target("macos", "aarch64").unwrap(),
            "aarch64-apple-darwin"
        );
        assert_eq!(
            release_target("macos", "x86_64").unwrap(),
            "x86_64-apple-darwin"
        );
        assert_eq!(
            release_target("linux", "x86_64").unwrap(),
            "x86_64-unknown-linux-musl"
        );
    }
}
