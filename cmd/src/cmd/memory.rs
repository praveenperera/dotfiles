use std::path::{Path, PathBuf};

use colored::Colorize;
use eyre::{eyre, Context as _, Result};
use log::info;
use xshell::Shell;

use crate::fsutil;

fn icloud_memories_dir() -> Result<PathBuf> {
    Ok(fsutil::home_dir()?.join("Library/Mobile Documents/com~apple~CloudDocs/dotfiles/memories"))
}

fn claude_projects_dir() -> Result<PathBuf> {
    Ok(fsutil::home_dir()?.join(".claude/projects"))
}

fn codex_memories_dir() -> Result<PathBuf> {
    Ok(fsutil::home_dir()?.join(".codex/memories"))
}

/// Strips the `-Users-praveen-code-` prefix from a Claude project dir name
/// Returns None if the dir doesn't match the expected pattern
fn project_short_name(dir_name: &str) -> Option<String> {
    let prefix = format!("-Users-{}-code-", std::env::var("USER").unwrap_or_default());
    dir_name.strip_prefix(&prefix).map(|s| s.to_string())
}

/// Converts short name to human-readable: "bitcoinppl-cove" → "bitcoinppl/cove"
/// Only replaces the first `-` with `/` to handle owner/repo pattern
fn human_readable_name(short_name: &str) -> String {
    if let Some(pos) = short_name.find('-') {
        format!("{}/{}", &short_name[..pos], &short_name[pos + 1..])
    } else {
        short_name.to_string()
    }
}

pub fn setup(sh: &Shell) -> Result<()> {
    let icloud_dir = icloud_memories_dir()?;
    let icloud_claude = icloud_dir.join("claude/projects");
    let icloud_codex = icloud_dir.join("codex");

    // create iCloud directory structure
    sh.create_dir(&icloud_claude)?;
    sh.create_dir(&icloud_codex)?;
    info!("{} iCloud directory structure", "Created".green());

    // migrate Claude project memories
    setup_claude_memories(sh, &icloud_claude)?;

    // migrate Codex memories
    setup_codex_memories(sh, &icloud_codex)?;

    info!("{}", "Setup complete!".green().bold());
    Ok(())
}

fn setup_claude_memories(sh: &Shell, icloud_claude: &Path) -> Result<()> {
    let claude_projects = claude_projects_dir()?;

    let entries = sh
        .read_dir(&claude_projects)
        .wrap_err("Failed to read Claude projects directory")?;

    for entry in entries {
        let memory_dir = entry.join("memory");

        // skip if no memory dir exists
        if !sh.path_exists(&memory_dir) {
            continue;
        }

        // skip if already a symlink
        if memory_dir.is_symlink() {
            let dir_name = entry.file_name().unwrap_or_default().to_string_lossy();
            info!("{} {} (already symlinked)", "Skipping".yellow(), dir_name);
            continue;
        }

        let dir_name = entry
            .file_name()
            .ok_or_else(|| eyre!("invalid dir entry"))?
            .to_string_lossy()
            .to_string();

        let icloud_target = icloud_claude.join(&dir_name);
        sh.create_dir(&icloud_target)?;

        // move contents if any exist
        let contents = sh.read_dir(&memory_dir).unwrap_or_default();

        for item in &contents {
            let dest = icloud_target.join(item.file_name().unwrap_or_default());
            std::fs::rename(item, &dest)
                .wrap_err_with(|| format!("Failed to move {}", item.display()))?;
        }

        // remove original dir and create symlink
        sh.remove_path(&memory_dir)?;
        std::os::unix::fs::symlink(&icloud_target, &memory_dir)
            .wrap_err_with(|| format!("Failed to symlink {}", memory_dir.display()))?;

        let status = if contents.is_empty() {
            "Linked"
        } else {
            "Migrated"
        };
        info!("{} {}", status.green(), dir_name);
    }

    Ok(())
}

fn setup_codex_memories(sh: &Shell, icloud_codex: &Path) -> Result<()> {
    let codex_dir = codex_memories_dir()?;

    // skip if already a symlink
    if codex_dir.is_symlink() {
        info!("{} codex memories (already symlinked)", "Skipping".yellow());
        return Ok(());
    }

    // move existing contents
    if sh.path_exists(&codex_dir) {
        let contents = sh.read_dir(&codex_dir).unwrap_or_default();
        for item in &contents {
            let dest = icloud_codex.join(item.file_name().unwrap_or_default());
            std::fs::rename(item, &dest)
                .wrap_err_with(|| format!("Failed to move {}", item.display()))?;
        }
        sh.remove_path(&codex_dir)?;
    }

    std::os::unix::fs::symlink(icloud_codex, &codex_dir)
        .wrap_err("Failed to symlink codex memories")?;

    info!("{} codex memories", "Linked".green());
    Ok(())
}

pub fn sync(sh: &Shell) -> Result<()> {
    let icloud_dir = icloud_memories_dir()?;
    let icloud_claude = icloud_dir.join("claude/projects");
    let icloud_codex = icloud_dir.join("codex");

    if !sh.path_exists(&icloud_claude) {
        return Err(eyre!(
            "iCloud Claude memories not found. Run `cmd sync memory --setup` first"
        ));
    }

    let entries = sh.read_dir(&icloud_claude)?;
    let mut synced = 0;

    for entry in entries {
        let dir_name = entry
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let short_name = match project_short_name(&dir_name) {
            Some(name) => name,
            None => continue,
        };

        // collect all .md files in this project memory dir
        let md_files: Vec<PathBuf> = sh
            .read_dir(&entry)
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
            .collect();

        if md_files.is_empty() {
            continue;
        }

        let human_name = human_readable_name(&short_name);
        let mut content = format!("# Project: {human_name}\n\n");

        for md_file in &md_files {
            let file_content = sh.read_file(md_file)?;
            if !file_content.trim().is_empty() {
                content.push_str(&file_content);
                if !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push('\n');
            }
        }

        // only write if there's actual content beyond the header
        if content.lines().count() > 2 {
            let dest = icloud_codex.join(format!("{short_name}.md"));
            sh.write_file(&dest, &content)?;
            info!("{} {}", "Synced".green(), short_name);
            synced += 1;
        }
    }

    info!(
        "{} {} project memories to Codex",
        "Synced".green().bold(),
        synced
    );
    Ok(())
}
