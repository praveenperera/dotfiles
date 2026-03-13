use eyre::Result;

use super::client::S2Client;
use super::types::AuthorDetail;
use crate::cmd::search::OutputFormat;

pub async fn run(client: &S2Client, query: &str, format: OutputFormat) -> Result<()> {
    // auto-detect: if numeric, fetch detail; otherwise search
    let is_id = query.chars().all(|c| c.is_ascii_digit());

    if is_id {
        let author = client.author_detail(query).await?;
        match format {
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&author)?),
            OutputFormat::Plain => display_detail(&author),
        }
    } else {
        let resp = client.author_search(query).await?;
        match format {
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
            OutputFormat::Plain => {
                let data = resp.data.unwrap_or_default();
                println!("{} authors\n", data.len());
                for (i, author) in data.iter().enumerate() {
                    display_brief(i + 1, author);
                }
            }
        }
    }

    Ok(())
}

fn display_detail(author: &AuthorDetail) {
    let name = author.name.as_deref().unwrap_or("Unknown");
    println!("{name}");
    println!("h-index: {}", author.h_index.unwrap_or(0));
    println!("Papers: {}", author.paper_count.unwrap_or(0));
    println!("Citations: {}", author.citation_count.unwrap_or(0));

    if let Some(affiliations) = &author.affiliations {
        if !affiliations.is_empty() {
            println!("Affiliations: {}", affiliations.join(", "));
        }
    }
}

fn display_brief(idx: usize, author: &AuthorDetail) {
    let name = author.name.as_deref().unwrap_or("Unknown");
    let h = author.h_index.unwrap_or(0);
    let papers = author.paper_count.unwrap_or(0);
    let id = author.author_id.as_deref().unwrap_or("?");
    println!("{idx}. {name}  (h={h}, papers={papers}, id={id})");
}
