use colored::Colorize;
use eyre::Result;

use super::client::download_pdf;
use super::db::PaperDb;
use super::display::display_paper_info;
use super::doi::{doi_to_filename, normalize_doi};
use super::extract::extract_text;
use super::types::LocalPaper;
use crate::cmd::search::config;
use crate::cmd::search::s2::client::S2Client;

pub async fn run(doi_input: &str, force: bool, tags: &[String]) -> Result<()> {
    let doi = normalize_doi(doi_input)?;

    let db = PaperDb::open(&config::data_dir().join("papers.db")).await?;

    if !force {
        if let Some(existing) = db.find_by_doi(&doi).await? {
            println!(
                "{} already in library: {}",
                "·".dimmed(),
                existing.title.as_deref().unwrap_or(&doi)
            );
            return Ok(());
        }
    }

    // resolve metadata from S2
    let (title, authors, year) = resolve_metadata(&doi).await;

    let display_title = title.as_deref().unwrap_or(&doi);
    println!("{} downloading: {display_title}", "↓".cyan());

    // download PDF
    let pdf_dir = config::data_dir().join("pdfs");
    let filename = format!("{}.pdf", doi_to_filename(&doi));
    let pdf_path = pdf_dir.join(&filename);

    let source = download_pdf(&doi, &pdf_path).await?;
    let file_size = std::fs::metadata(&pdf_path)?.len() as i64;

    println!("{} downloaded ({source})", "✓".green());

    // extract text
    let full_text = match extract_text(&pdf_path) {
        Ok(text) => {
            if text.len() < 100 {
                eprintln!(
                    "{} extracted text is very short ({} chars) — PDF may be scanned/image-based",
                    "⚠".yellow(),
                    text.len()
                );
            }
            Some(text)
        }
        Err(e) => {
            eprintln!("{} text extraction failed: {e}", "⚠".yellow());
            None
        }
    };

    let text_length = full_text.as_ref().map(|t| t.len() as i64);

    let paper = LocalPaper {
        id: 0,
        doi: doi.clone(),
        title,
        authors,
        year,
        pdf_path: pdf_path.to_string_lossy().to_string(),
        source,
        downloaded_at: chrono::Utc::now().to_rfc3339(),
        file_size: Some(file_size),
        text_length,
        full_text,
    };

    db.insert(&paper).await?;

    if !tags.is_empty() {
        let tags: Vec<String> = tags.iter().map(|t| t.trim().to_lowercase()).collect();
        db.add_tags(&doi, &tags).await?;
    }

    println!();
    display_paper_info(&paper, tags);

    Ok(())
}

async fn resolve_metadata(doi: &str) -> (Option<String>, Option<String>, Option<i64>) {
    let Ok(client) = S2Client::new() else {
        return (None, None, None);
    };

    let Ok(paper) = client.paper_detail(&format!("DOI:{doi}")).await else {
        return (None, None, None);
    };

    let title = paper.title;
    let year = paper.year.map(|y| y as i64);
    let authors = paper.authors.map(|a| {
        a.iter()
            .filter_map(|a| a.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    });

    (title, authors, year)
}
