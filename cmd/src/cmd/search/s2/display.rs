use super::types::Paper;
use crate::cmd::search::format_authors;

pub fn display_paper(idx: usize, paper: &Paper) {
    let title = paper.title.as_deref().unwrap_or("Untitled");
    let year = paper.year.map(|y| format!("({y})")).unwrap_or_default();

    let authors: Vec<String> = paper
        .authors
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter_map(|a| a.name.clone())
        .collect();
    let authors = format_authors(&authors, 3);

    let venue = paper.venue.as_deref().unwrap_or("");
    let citations = paper.citation_count.unwrap_or(0);

    println!("{}. {} {}", idx, title, year);

    let mut meta = format!("   {authors}");
    if !venue.is_empty() {
        meta.push_str(&format!("  ·  {venue}"));
    }
    meta.push_str(&format!("  ·  Citations: {citations}"));
    println!("{meta}");

    if let Some(ids) = &paper.external_ids {
        if let Some(doi) = &ids.doi {
            println!("   DOI: {doi}");
        } else if let Some(arxiv) = &ids.arxiv {
            println!("   arXiv: {arxiv}");
        }
    }
}

pub fn display_paper_detail(paper: &Paper) {
    let title = paper.title.as_deref().unwrap_or("Untitled");
    let year = paper.year.map(|y| format!("({y})")).unwrap_or_default();

    println!("{title} {year}");
    println!();

    let authors: Vec<String> = paper
        .authors
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter_map(|a| a.name.clone())
        .collect();
    if !authors.is_empty() {
        println!("Authors: {}", authors.join(", "));
    }

    if let Some(venue) = &paper.venue {
        if !venue.is_empty() {
            println!("Venue: {venue}");
        }
    }

    println!("Citations: {}", paper.citation_count.unwrap_or(0));

    if let Some(ids) = &paper.external_ids {
        if let Some(doi) = &ids.doi {
            println!("DOI: {doi}");
        }
        if let Some(arxiv) = &ids.arxiv {
            println!("arXiv: {arxiv}");
        }
    }

    if let Some(fields) = &paper.fields_of_study {
        if !fields.is_empty() {
            println!("Fields: {}", fields.join(", "));
        }
    }

    if let Some(types) = &paper.publication_types {
        if !types.is_empty() {
            println!("Types: {}", types.join(", "));
        }
    }

    if let Some(pdf) = &paper.open_access_pdf {
        if let Some(url) = &pdf.url {
            println!("PDF: {url}");
        }
    }

    if let Some(abs) = &paper.r#abstract {
        println!("\n{abs}");
    }
}
