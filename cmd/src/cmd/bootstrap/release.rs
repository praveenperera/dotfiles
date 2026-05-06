use colored::Colorize;
use eyre::{eyre, Result};
use xshell::{cmd, Shell};

use crate::{fsutil, util::has_tool};

use super::link::create_hardlinks;

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

const LOCAL_PROJECTS: &[LocalProject] = &[project!("jju"), project!("aps")];

pub(crate) fn release(sh: &Shell, project: Option<String>) -> Result<()> {
    match project {
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

        create_hardlinks(sh)?;
        return Ok(());
    }

    sh.change_dir(crate::dotfiles_dir()?);
    sh.change_dir("cmd");

    if cmd!(sh, "./release").run().is_err() {
        println!("{}", "failed to build cmd binary".red());
        std::process::exit(1);
    }

    create_hardlinks(sh)?;

    Ok(())
}
