use colored::Colorize;

use super::types::{LocalPaper, SearchResult};
use crate::cmd::search::format_authors;

pub fn display_paper_list(papers: &[LocalPaper]) {
    if papers.is_empty() {
        println!("{}", "no papers in library".dimmed());
        return;
    }

    println!("{} papers\n", papers.len());

    for (i, paper) in papers.iter().enumerate() {
        let title = paper.title.as_deref().unwrap_or("Untitled");
        let year = paper.year.map(|y| format!("({y})")).unwrap_or_default();

        println!("{}. {} {}", i + 1, title.bold(), year);

        if let Some(authors) = &paper.authors {
            let names: Vec<String> = authors.split(", ").map(String::from).collect();
            println!("   {}", format_authors(&names, 3));
        }

        println!("   DOI: {}", paper.doi.cyan());

        let size = paper
            .file_size
            .map(|s| format_size(s as u64))
            .unwrap_or_default();
        let text_len = paper
            .text_length
            .map(|l| format!("{l} chars"))
            .unwrap_or_else(|| "no text".to_string());

        println!(
            "   {} · {} · {}",
            paper.source.dimmed(),
            size.dimmed(),
            text_len.dimmed()
        );
        println!();
    }
}

pub fn display_search_results(results: &[SearchResult]) {
    if results.is_empty() {
        println!("{}", "no results".dimmed());
        return;
    }

    println!("{} results\n", results.len());

    for (i, result) in results.iter().enumerate() {
        let title = result.title.as_deref().unwrap_or("Untitled");
        let year = result.year.map(|y| format!("({y})")).unwrap_or_default();

        println!("{}. {} {}", i + 1, title.bold(), year);

        if let Some(authors) = &result.authors {
            let names: Vec<String> = authors.split(", ").map(String::from).collect();
            println!("   {}", format_authors(&names, 3));
        }

        println!("   DOI: {}", result.doi.cyan());
        println!("   {}", result.snippet.dimmed());
        println!();
    }
}

pub fn display_paper_info(paper: &LocalPaper, tags: &[String]) {
    let title = paper.title.as_deref().unwrap_or("Untitled");
    let year = paper.year.map(|y| format!("({y})")).unwrap_or_default();

    println!("{} {}", title.bold(), year);

    if let Some(authors) = &paper.authors {
        println!("Authors: {authors}");
    }

    println!("DOI: {}", paper.doi.cyan());
    println!("Source: {}", paper.source);
    println!("Downloaded: {}", paper.downloaded_at);
    println!("PDF: {}", paper.pdf_path);

    if let Some(size) = paper.file_size {
        println!("File size: {}", format_size(size as u64));
    }

    if let Some(len) = paper.text_length {
        println!("Text length: {len} chars");
    }

    if !tags.is_empty() {
        println!("Tags: {}", tags.join(", ").cyan());
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
