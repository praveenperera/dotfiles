use colored::Colorize;
use eyre::{eyre, Result};

use super::db::PaperDb;
use super::doi::normalize_doi;
use crate::cmd::search::config;

pub async fn run(doi_input: &str) -> Result<()> {
    let doi = normalize_doi(doi_input)?;
    let db = PaperDb::open(&config::data_dir().join("papers.db")).await?;

    let paper = db
        .find_by_doi(&doi)
        .await?
        .ok_or_else(|| eyre!("paper not in library: {doi}"))?;

    let path = std::path::Path::new(&paper.pdf_path);
    if !path.exists() {
        return Err(eyre!("PDF file missing: {}", paper.pdf_path));
    }

    println!(
        "{} opening: {}",
        "→".cyan(),
        paper.title.as_deref().unwrap_or(&doi)
    );

    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    std::process::Command::new(cmd)
        .arg(&paper.pdf_path)
        .spawn()?;

    Ok(())
}
