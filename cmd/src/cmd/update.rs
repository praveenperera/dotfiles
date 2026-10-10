use std::env;

use clap::Args;
use colored::Colorize;
use eyre::{eyre, Result, WrapErr};
use semver::Version;
use xshell::Shell;

use crate::cmd::{bootstrap, install};

const REPO: &str = "praveenperera/dotfiles";

#[derive(Debug, Clone, Args)]
pub struct Update {
    /// Reinstall the latest release even if it is not newer
    #[arg(short, long)]
    pub force: bool,
}

pub fn run_with_flags(sh: &Shell, flags: Update) -> Result<()> {
    let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let release = crate::runtime::block_on(install::fetch_latest_release(REPO))??;
    let latest = parse_tag(&release.tag_name)?;

    if latest <= current && !flags.force {
        println!(
            "{} cmd {current} (latest release is {latest})",
            "Up to date".green()
        );
        return Ok(());
    }

    let asset_name = asset_name(&release.tag_name, env::consts::OS, env::consts::ARCH)?;

    // the release is created before its build jobs upload assets, so a fresh tag can be empty
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| {
            eyre!(
                "release {} has no {asset_name} asset yet, the build may still be running",
                release.tag_name
            )
        })?;

    install::ensure_release_dependencies(sh)?;

    println!("{} cmd {current} -> {latest}", "Updating".green());

    let tmp_dir = sh.create_temp_dir()?;
    let archive_path = tmp_dir.path().join(&asset.name);
    install::download_release_asset(&asset.browser_download_url, &archive_path)?;
    install::extract_archive(&archive_path, tmp_dir.path())?;

    let binary = install::find_release_binary(tmp_dir.path(), "cmd")?;
    bootstrap::install_cmd_binary(sh, &binary)?;

    println!("{} cmd {latest}", "Updated".green());
    Ok(())
}

fn parse_tag(tag: &str) -> Result<Version> {
    Version::parse(tag.trim_start_matches('v'))
        .wrap_err_with(|| format!("invalid release tag: {tag}"))
}

/// Mirrors the asset names published by .github/workflows/release-cmd.yml
fn asset_name(tag: &str, os: &str, arch: &str) -> Result<String> {
    let suffix = match (os, arch) {
        ("macos", "aarch64") => "macos-arm64",
        ("linux", "x86_64") => "linux-musl",
        _ => return Err(eyre!("no cmd release is published for {arch}-{os}")),
    };

    Ok(format!("cmd-{tag}-{suffix}.tar.gz"))
}

#[cfg(test)]
mod tests {
    use super::parse_tag;

    #[test]
    fn compares_release_tags_numerically() {
        assert!(parse_tag("v0.0.228").unwrap() > parse_tag("v0.0.99").unwrap());
        assert!(parse_tag("not-a-version").is_err());
    }
}
