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

use crate::{fsutil, util::has_tool};

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
    method: InstallMethod,
}

#[derive(Debug, Clone, Copy)]
enum InstallMethod {
    /// Download `<crate>-<tag>-<target>.tar.gz` from the release
    ReleaseArchive,
    /// Run the repository's install.sh, which takes an optional tag as its
    /// first argument and the destination directory from this variable
    Script { install_dir_env: &'static str },
}

#[derive(Debug, Deserialize)]
pub(crate) struct LatestRelease {
    pub(crate) tag_name: String,
    #[serde(default)]
    pub(crate) assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReleaseAsset {
    pub(crate) name: String,
    pub(crate) browser_download_url: String,
}

const GITHUB_RELEASE_TOOLS: &[GithubReleaseTool] = &[
    GithubReleaseTool {
        name: "rustywind",
        repo: "avencera/rustywind",
        crate_name: "rustywind",
        method: InstallMethod::ReleaseArchive,
    },
    GithubReleaseTool {
        name: "smrze",
        repo: "avencera/smrze",
        crate_name: "smrze",
        method: InstallMethod::ReleaseArchive,
    },
    // a private repo whose install.sh also verifies release checksums
    GithubReleaseTool {
        name: "cln",
        repo: "praveenperera/cln",
        crate_name: "cln",
        method: InstallMethod::Script {
            install_dir_env: "CLN_INSTALL_DIR",
        },
    },
];

pub fn run_with_flags(sh: &Shell, flags: Install) -> Result<()> {
    let tool = resolve_tool(&flags.tool)?;
    let dest_dir = flags.to.unwrap_or(fsutil::home_dir()?.join(".local/bin"));

    let dest = dest_dir.join(tool.crate_name);
    if path_exists_or_symlink(&dest) && !flags.force {
        return Err(eyre!(
            "{} already exists in {}, use --force to overwrite it",
            tool.crate_name,
            dest_dir.display()
        ));
    }

    match tool.method {
        InstallMethod::ReleaseArchive => {
            let target = current_release_target()?;
            let tag = match flags.tag {
                Some(tag) => tag,
                None => crate::runtime::block_on(fetch_latest_release(tool.repo))??.tag_name,
            };

            install_github_release(sh, tool, &tag, &target, &dest_dir, flags.force)
        }
        InstallMethod::Script { install_dir_env } => {
            install_with_script(sh, tool, install_dir_env, flags.tag.as_deref(), &dest_dir)
        }
    }
}

fn install_with_script(
    sh: &Shell,
    tool: GithubReleaseTool,
    install_dir_env: &str,
    tag: Option<&str>,
    dest_dir: &Path,
) -> Result<()> {
    // private repos need a token for the script itself, and the script falls
    // back to GH_TOKEN for release downloads when gh is not logged in
    let token = github_token().or_else(gh_auth_token);
    let script = crate::runtime::block_on(fetch_install_script(tool.repo, token.as_deref()))??;

    let tmp_dir = sh.create_temp_dir()?;
    let script_path = tmp_dir.path().join("install.sh");
    fs::write(&script_path, script)?;

    println!(
        "{} {} (install.sh from {})",
        "Installing".green(),
        tool.name.blue(),
        tool.repo
    );

    let mut command = Command::new("bash");
    command
        .arg(&script_path)
        .args(tag)
        .env(install_dir_env, dest_dir);

    if let Some(token) = token {
        command.env("GH_TOKEN", token);
    }

    let status = command.status().wrap_err("failed to run bash")?;
    if !status.success() {
        return Err(eyre!("{} install.sh failed with {status}", tool.repo));
    }

    Ok(())
}

async fn fetch_install_script(repo: &str, token: Option<&str>) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("cmd-install")
        .build()?;
    let mut request = client
        .get(format!(
            "https://api.github.com/repos/{repo}/contents/install.sh"
        ))
        .header("Accept", "application/vnd.github.raw");

    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }

    let response = request.send().await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await?;
        return Err(eyre!(
            "failed to fetch install.sh from {repo}: HTTP {status}: {body}"
        ));
    }

    Ok(response.text().await?)
}

fn gh_auth_token() -> Option<String> {
    let output = Command::new("gh").args(["auth", "token"]).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let token = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!token.is_empty()).then_some(token)
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

pub(crate) async fn fetch_latest_release(repo: &str) -> Result<LatestRelease> {
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

    Ok(response.json().await?)
}

pub(crate) fn ensure_release_dependencies(sh: &Shell) -> Result<()> {
    for command in ["curl", "tar"] {
        if !has_tool(sh, command) {
            return Err(eyre!("need {command} (command not found)"));
        }
    }

    Ok(())
}

pub(crate) fn download_release_asset(url: &str, archive_path: &Path) -> Result<()> {
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

pub(crate) fn extract_archive(archive_path: &Path, dest_dir: &Path) -> Result<()> {
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

pub(crate) fn find_release_binary(root: &Path, crate_name: &str) -> Result<PathBuf> {
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
    ["GITHUB_TOKEN", "GH_TOKEN"]
        .into_iter()
        .filter_map(|name| env::var(name).ok())
        .find(|token| !token.trim().is_empty())
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
