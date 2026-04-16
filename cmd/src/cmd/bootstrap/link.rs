use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use askama::Template;
use colored::Colorize;
use eyre::{eyre, Result};
use xshell::{cmd, Shell};

use crate::{fsutil, util::has_tool, CMD_TOOLS};

use super::{
    ManagedDirEntry, Os, OsxDefaults, SyncSymlinkOutcome, Zshrc, CONFIG_FILE_OR_DIR,
    CUSTOM_CONFIG_DIR_ENTRIES, CUSTOM_CONFIG_OR_DIR, DOTFILES, MAC_ONLY_CUSTOM_CONFIG_OR_DIR,
};

struct LinkSpec {
    source: PathBuf,
    target: PathBuf,
}

pub(crate) fn config(sh: &Shell) -> Result<()> {
    let path = crate::dotfiles_dir()?.join("zshrc");
    let zshrc = Zshrc { os: Os::current() };

    println!("writing zshrc to {}", path.display().to_string().green());
    sh.write_file(&path, zshrc.render()?)?;

    if let Os::MacOS = Os::current() {
        println!("{}", "installing osx defaults".green());
        let osx_defaults = OsxDefaults {}.render()?;
        create_and_run_file(sh, &osx_defaults, "osx_defaults.zsh")?;
        setup_ghostty_terminfo(sh)?;
    }

    setup_config_and_dotfiles(sh)?;
    create_gitconfig_local(sh)?;
    create_hardlinks(sh)?;
    reload_configs(sh);

    Ok(())
}

pub(super) fn create_hardlinks(sh: &Shell) -> Result<()> {
    let home = fsutil::home_dir()?;
    let cmd_path = home.join(".local/bin/cmd");
    if !sh.path_exists(&cmd_path) {
        return Ok(());
    }

    for (tool, _) in CMD_TOOLS {
        if *tool == "cmd" {
            continue;
        }

        let tool_path = home.join(format!(".local/bin/{tool}"));
        fsutil::remove_existing_path(&tool_path)?;
        sh.hard_link(&cmd_path, &tool_path)?;
    }

    Ok(())
}

fn reload_configs(sh: &Shell) {
    let Ok(home) = fsutil::home_dir() else {
        return;
    };

    let tmux_conf = home.join(".tmux.conf");
    if cmd!(sh, "tmux source-file {tmux_conf}")
        .quiet()
        .run()
        .is_ok()
    {
        println!("{}", "reloaded tmux config".green());
    }
}

fn setup_config_and_dotfiles(sh: &Shell) -> Result<()> {
    let home = fsutil::home_dir()?;
    let dotfiles_dir = crate::dotfiles_dir()?;

    println!("{}", "setting up zsh_plugins.zsh file...".green());
    let antidote_script = include_str!("../../../scripts/antidote.zsh");

    {
        let dotfiles_dir = dotfiles_dir
            .to_str()
            .ok_or_else(|| eyre!("invalid path: {}", dotfiles_dir.display()))?;
        let _dir = sh.push_env("DOTFILES_DIR", dotfiles_dir);
        cmd!(sh, "zsh -c {antidote_script}").quiet().run()?;
    }

    let config_dir = home.join(".config");
    if !sh.path_exists(&config_dir) {
        sh.create_dir(&config_dir)?;
    }

    prune_managed_dir_entries(&home, &dotfiles_dir)?;

    for spec in build_link_specs(&home, &dotfiles_dir)? {
        if sync_symlink(sh, &spec.source, &spec.target)? == SyncSymlinkOutcome::SkippedBrokenSource
        {
            println!(
                "{} {}",
                "skipping broken symlink source".yellow(),
                spec.source.display().to_string().blue()
            );
        }
    }

    if has_tool(sh, "tmux") {
        install_tpm(sh, &home)?;
    }

    Ok(())
}

fn build_link_specs(home: &Path, dotfiles_dir: &Path) -> Result<Vec<LinkSpec>> {
    let mut specs = DOTFILES
        .iter()
        .map(|filename| LinkSpec {
            source: dotfiles_dir.join(filename),
            target: home.join(format!(".{filename}")),
        })
        .collect::<Vec<_>>();

    specs.extend(CONFIG_FILE_OR_DIR.iter().map(|filename| LinkSpec {
        source: dotfiles_dir.join("config").join(filename),
        target: home.join(format!(".config/{filename}")),
    }));

    if let Os::MacOS = Os::current() {
        specs.extend(
            MAC_ONLY_CUSTOM_CONFIG_OR_DIR
                .iter()
                .map(|(src, dest)| LinkSpec {
                    source: dotfiles_dir.join(src),
                    target: home.join(dest),
                }),
        );
    }

    specs.extend(CUSTOM_CONFIG_OR_DIR.iter().map(|(src, dest)| LinkSpec {
        source: dotfiles_dir.join(src),
        target: home.join(dest),
    }));

    for entry in CUSTOM_CONFIG_DIR_ENTRIES {
        let src_dir = dotfiles_dir.join(entry.source);
        let dest_dir = home.join(entry.target);

        for entry in fs::read_dir(&src_dir)? {
            let entry = entry?;
            specs.push(LinkSpec {
                source: entry.path(),
                target: dest_dir.join(entry.file_name()),
            });
        }
    }

    Ok(specs)
}

fn prune_managed_dir_entries(home: &Path, dotfiles_dir: &Path) -> Result<()> {
    for entry in CUSTOM_CONFIG_DIR_ENTRIES {
        let source_dir = dotfiles_dir.join(entry.source);
        let target_dir = home.join(entry.target);
        prune_managed_dir_entry(&source_dir, &target_dir, dotfiles_dir, entry)?;
    }

    Ok(())
}

fn prune_managed_dir_entry(
    source_dir: &Path,
    target_dir: &Path,
    dotfiles_dir: &Path,
    entry: &ManagedDirEntry,
) -> Result<()> {
    let target_metadata = match fs::symlink_metadata(target_dir) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };

    if !target_metadata.is_dir() || target_metadata.file_type().is_symlink() {
        return Ok(());
    }

    let managed_entries = fs::read_dir(source_dir)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<std::io::Result<HashSet<OsString>>>()?;

    for target_entry in fs::read_dir(target_dir)? {
        let target_entry = target_entry?;
        if managed_entries.contains(&target_entry.file_name()) {
            continue;
        }

        let target_path = target_entry.path();
        let metadata = fs::symlink_metadata(&target_path)?;
        if !metadata.file_type().is_symlink() {
            continue;
        }

        let link_target = fs::read_link(&target_path)?;
        if is_managed_link_target(&link_target, dotfiles_dir, entry) {
            fsutil::remove_existing_path(&target_path)?;
        }
    }

    Ok(())
}

fn is_managed_link_target(
    link_target: &Path,
    dotfiles_dir: &Path,
    entry: &ManagedDirEntry,
) -> bool {
    link_target.starts_with(dotfiles_dir.join(entry.source))
}

pub(super) fn sync_symlink(sh: &Shell, path: &Path, target: &Path) -> Result<SyncSymlinkOutcome> {
    if is_broken_symlink(path)? {
        fsutil::remove_existing_path(target)?;
        return Ok(SyncSymlinkOutcome::SkippedBrokenSource);
    }

    fsutil::remove_existing_path(target)?;
    fsutil::ensure_parent_dir(target)?;

    cmd!(sh, "ln -s {path} {target}").run()?;
    Ok(SyncSymlinkOutcome::Linked)
}

fn is_broken_symlink(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_symlink() && !path.exists()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err.into()),
    }
}

fn install_tpm(sh: &Shell, home: &Path) -> Result<()> {
    let target = home.join(".tmux/plugins/tpm");

    if !sh.path_exists(&target) {
        println!("{}", "tmux package manager not foud".blue());
        println!("{}", "install tmux package manager (TPM)".green());
        cmd!(sh, "git clone https://github.com/tmux-plugins/tpm {target}").run()?;
    }

    Ok(())
}

/// Install Ghostty's xterm-ghostty terminfo entry to ~/.terminfo so it's
/// available to the terminfo library before any shell initialization runs
fn setup_ghostty_terminfo(sh: &Shell) -> Result<()> {
    let ghostty_terminfo = "/Applications/Ghostty.app/Contents/Resources/terminfo";
    if !sh.path_exists(ghostty_terminfo) {
        return Ok(());
    }

    let home = fsutil::home_dir()?;
    if sh.path_exists(home.join(".terminfo/78/xterm-ghostty")) {
        return Ok(());
    }

    println!("{}", "installing ghostty terminfo to ~/.terminfo".green());
    let script = format!("TERMINFO_DIRS='{ghostty_terminfo}' infocmp -x xterm-ghostty | tic -x -");
    cmd!(sh, "sh -c {script}").run()?;

    Ok(())
}

fn create_gitconfig_local(sh: &Shell) -> Result<()> {
    let path = fsutil::home_dir()?.join(".gitconfig.local");
    if sh.path_exists(&path) {
        return Ok(());
    }

    println!("{}", "creating ~/.gitconfig.local".green());
    sh.write_file(&path, "[user]\n  signingkey = CHANGE_THIS\n")?;
    Ok(())
}

fn create_and_run_file(sh: &Shell, contents: &str, file: &str) -> Result<()> {
    let tmp_dir = sh.create_temp_dir()?;
    let tmp_path = tmp_dir.path().join(file);
    sh.write_file(&tmp_path, contents)?;

    println!("running {}", file.green());
    cmd!(sh, "zsh {tmp_path}").quiet().run()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{build_link_specs, prune_managed_dir_entry, ManagedDirEntry};

    #[test]
    fn keeps_unmanaged_entries_in_target_dir() {
        let dir = tempdir().unwrap();
        let dotfiles_dir = dir.path().join("dotfiles");
        let source_dir = dotfiles_dir.join("agents/skills");
        let target_dir = dir.path().join("target");
        let unmanaged_dir = target_dir.join("figma");

        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();
        fs::create_dir(&unmanaged_dir).unwrap();

        let entry = ManagedDirEntry {
            source: "agents/skills",
            target: ".codex/skills",
        };

        prune_managed_dir_entry(&source_dir, &target_dir, &dotfiles_dir, &entry).unwrap();

        assert!(unmanaged_dir.is_dir());
    }

    #[test]
    fn includes_general_agent_files_for_managed_targets() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        let dotfiles_dir = dir.path().join("dotfiles");
        let agents_dir = dotfiles_dir.join("agents");

        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(agents_dir.join("AGENTS.md"), "agents").unwrap();
        fs::write(agents_dir.join("commit-message-guide.md"), "guide").unwrap();

        let specs = build_link_specs(&home, &dotfiles_dir).unwrap();

        assert!(specs.iter().any(|spec| {
            spec.source == agents_dir.join("AGENTS.md")
                && spec.target == home.join(".agents/AGENTS.md")
        }));
        assert!(specs.iter().any(|spec| {
            spec.source == agents_dir.join("commit-message-guide.md")
                && spec.target == home.join(".agents/commit-message-guide.md")
        }));
        assert!(specs.iter().any(|spec| {
            spec.source == agents_dir.join("AGENTS.md")
                && spec.target == home.join(".codex/AGENTS.md")
        }));
        assert!(specs.iter().any(|spec| {
            spec.source == agents_dir.join("AGENTS.md")
                && spec.target == home.join(".config/opencode/AGENTS.md")
        }));
    }
}
