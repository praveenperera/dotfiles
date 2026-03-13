use super::types::Work;
use crate::cmd::search::format_authors;

pub fn display_work(idx: usize, work: &Work) {
    let title = work.title.as_deref().unwrap_or("Untitled");
    let year = work
        .publication_year
        .map(|y| format!("({y})"))
        .unwrap_or_default();

    let authors: Vec<String> = work
        .authorships
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter_map(|a| a.author.as_ref()?.display_name.clone())
        .collect();
    let authors = format_authors(&authors, 3);

    let venue = work
        .primary_location
        .as_ref()
        .and_then(|l| l.source.as_ref())
        .and_then(|s| s.display_name.as_deref())
        .unwrap_or("");
    let citations = work.cited_by_count.unwrap_or(0);

    println!("{}. {} {}", idx, title, year);

    let mut meta = format!("   {authors}");
    if !venue.is_empty() {
        meta.push_str(&format!("  ·  {venue}"));
    }
    meta.push_str(&format!("  ·  Citations: {citations}"));
    println!("{meta}");

    if let Some(doi) = &work.doi {
        println!("   DOI: {doi}");
    }
}

pub fn display_work_detail(work: &Work) {
    let title = work.title.as_deref().unwrap_or("Untitled");
    let year = work
        .publication_year
        .map(|y| format!("({y})"))
        .unwrap_or_default();

    println!("{title} {year}");
    println!();

    let authors: Vec<String> = work
        .authorships
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter_map(|a| a.author.as_ref()?.display_name.clone())
        .collect();
    if !authors.is_empty() {
        println!("Authors: {}", authors.join(", "));
    }

    if let Some(loc) = &work.primary_location {
        if let Some(source) = &loc.source {
            if let Some(name) = &source.display_name {
                println!("Source: {name}");
            }
        }
    }

    println!("Citations: {}", work.cited_by_count.unwrap_or(0));

    if let Some(doi) = &work.doi {
        println!("DOI: {doi}");
    }

    if let Some(oa) = &work.open_access {
        if let Some(status) = &oa.oa_status {
            println!("OA Status: {status}");
        }
        if let Some(url) = &oa.oa_url {
            println!("OA URL: {url}");
        }
    }

    if let Some(wt) = &work.work_type {
        println!("Type: {wt}");
    }

    if let Some(topics) = &work.topics {
        let topic_names: Vec<&str> = topics
            .iter()
            .filter_map(|t| t.display_name.as_deref())
            .take(5)
            .collect();
        if !topic_names.is_empty() {
            println!("Topics: {}", topic_names.join(", "));
        }
    }

    if let Some(id) = &work.id {
        println!("OpenAlex: {id}");
    }
}
