use std::{fs, io::Write, path::PathBuf};

use color_eyre::eyre::{Result, WrapErr};

use super::config;

pub fn record_prompt(prompt: &str) -> Result<()> {
    let path = history_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).wrap_err("Failed to create codex-voice history directory")?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .wrap_err("Failed to open codex-voice prompt history")?;
    writeln!(file, "{}", serde_json::json!({ "prompt": prompt }))
        .wrap_err("Failed to write codex-voice prompt history")?;
    Ok(())
}

fn history_path() -> Result<PathBuf> {
    Ok(config::home_dir()?.join(".local/share/codex-voice/prompts.jsonl"))
}
