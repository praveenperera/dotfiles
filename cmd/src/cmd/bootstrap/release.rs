use std::fs;
use std::path::Path;

use colored::Colorize;
use eyre::{eyre, Result, WrapErr};
use xshell::{cmd, Shell};

use crate::{cmd::main_cmd::ReleaseArgs, fsutil, util::has_tool};

use super::link::{create_hardlinks, create_hardlinks_in_bin_dir};

struct LocalProject {
    name: &'static str,
    path: Option<&'static str>,
    command: Option<&'static str>,
}

macro_rules! project {
    ($name:literal) => {
        LocalProject {
            name: $name,
            path: None,
            command: None,
        }
    };
    ($name:literal, $path:literal) => {
        LocalProject {
            name: $name,
            path: Some($path),
            command: None,
        }
    };
    ($name:literal, $path:literal, $cmd:literal) => {
        LocalProject {
            name: $name,
            path: Some($path),
            command: Some($cmd),
        }
    };
}

const LOCAL_PROJECTS: &[LocalProject] = &[project!("jju"), project!("aps"), project!("planport")];

pub(crate) fn release(sh: &Shell, args: ReleaseArgs) -> Result<()> {
    if let Some(path) = args.install_built {
        if let Some(project) = args.project {
            eyre::bail!("cannot install a built cmd binary while releasing project: {project}");
        }

        install_built_cmd(sh, &path)?;
        return Ok(());
    }

    if args.link_installed {
        if let Some(project) = args.project {
            eyre::bail!("cannot refresh cmd links while releasing project: {project}");
        }

        create_hardlinks(sh)?;
        return Ok(());
    }

    match args.project {
        None => release_cmd(sh),
        Some(name) => release_local(sh, &name),
    }
}

fn release_local(sh: &Shell, name: &str) -> Result<()> {
    let project = LOCAL_PROJECTS
        .iter()
        .find(|project| project.name == name)
        .ok_or_else(|| eyre!("unknown project: {name}"))?;

    let home = fsutil::home_dir()?;
    let path = project
        .path
        .map(|path| path.replace("~", &home.display().to_string()))
        .unwrap_or_else(|| format!("{}/code/{name}", home.display()));

    if !sh.path_exists(&path) {
        eyre::bail!("project path does not exist: {path}");
    }

    let command = project.command.unwrap_or("just release local");

    sh.change_dir(&path);
    cmd!(sh, "sh -c {command}").run()?;
    Ok(())
}

fn release_cmd(sh: &Shell) -> Result<()> {
    if !has_tool(sh, "cargo") || !has_tool(sh, "rustc") {
        println!(
            "{}",
            "detected minimal install, using release-minimal script".blue()
        );

        sh.change_dir(crate::dotfiles_dir()?);
        sh.change_dir("cmd");

        if cmd!(sh, "./release-minimal").run().is_err() {
            println!("{}", "failed to download cmd binary from github".red());
            std::process::exit(1);
        }

        return Ok(());
    }

    sh.change_dir(crate::dotfiles_dir()?);
    sh.change_dir("cmd");

    if cmd!(sh, "./release").run().is_err() {
        println!("{}", "failed to build cmd binary".red());
        std::process::exit(1);
    }

    Ok(())
}

fn install_built_cmd(sh: &Shell, built_cmd: &Path) -> Result<()> {
    let bin_dir = fsutil::home_dir()?.join(".local/bin");
    install_built_cmd_in_bin_dir(sh, built_cmd, &bin_dir)
}

fn install_built_cmd_in_bin_dir(sh: &Shell, built_cmd: &Path, bin_dir: &Path) -> Result<()> {
    let cmd_path = bin_dir.join("cmd");
    let old_cmd_path = bin_dir.join("cmd.old");

    fs::create_dir_all(bin_dir)
        .wrap_err_with(|| format!("failed to create bin directory: {}", bin_dir.display()))?;
    fsutil::remove_existing_path(&old_cmd_path)?;

    let had_existing_cmd = cmd_path.exists();
    if had_existing_cmd {
        fs::rename(&cmd_path, &old_cmd_path).wrap_err_with(|| {
            format!(
                "failed to move existing cmd binary from {} to {}",
                cmd_path.display(),
                old_cmd_path.display()
            )
        })?;
    }

    if let Err(err) = fs::copy(built_cmd, &cmd_path) {
        let _ = fsutil::remove_existing_path(&cmd_path);
        if had_existing_cmd {
            fs::rename(&old_cmd_path, &cmd_path).wrap_err_with(|| {
                format!(
                    "failed to restore old cmd binary after copy failed: {}",
                    cmd_path.display()
                )
            })?;
        }
        return Err(err).wrap_err_with(|| {
            format!(
                "failed to install cmd binary from {} to {}",
                built_cmd.display(),
                cmd_path.display()
            )
        });
    }

    fsutil::remove_existing_path(&old_cmd_path)?;
    create_hardlinks_in_bin_dir(sh, bin_dir)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;
    use xshell::Shell;

    use super::install_built_cmd_in_bin_dir;
    use crate::CMD_TOOLS;

    #[test]
    fn install_built_cmd_refreshes_tool_links() {
        let dir = tempdir().unwrap();
        let sh = Shell::new().unwrap();
        let built_cmd = dir.path().join("built-cmd");
        let bin_dir = dir.path().join("bin");

        fs::write(&built_cmd, "new cmd").unwrap();

        install_built_cmd_in_bin_dir(&sh, &built_cmd, &bin_dir).unwrap();

        assert_eq!(fs::read_to_string(bin_dir.join("cmd")).unwrap(), "new cmd");
        assert!(!bin_dir.join("cmd.old").exists());

        for (tool, _) in CMD_TOOLS {
            if *tool == "cmd" {
                continue;
            }

            assert_eq!(fs::read_to_string(bin_dir.join(tool)).unwrap(), "new cmd");
        }
    }

    #[test]
    fn install_built_cmd_restores_existing_binary_when_copy_fails() {
        let dir = tempdir().unwrap();
        let sh = Shell::new().unwrap();
        let bin_dir = dir.path().join("bin");
        let cmd_path = bin_dir.join("cmd");
        let missing_built_cmd = dir.path().join("missing-cmd");

        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(&cmd_path, "old cmd").unwrap();

        let err = install_built_cmd_in_bin_dir(&sh, &missing_built_cmd, &bin_dir).unwrap_err();

        assert!(
            err.to_string().contains("failed to install cmd binary"),
            "{err:?}"
        );
        assert_eq!(fs::read_to_string(&cmd_path).unwrap(), "old cmd");
        assert!(!bin_dir.join("cmd.old").exists());
    }
}
