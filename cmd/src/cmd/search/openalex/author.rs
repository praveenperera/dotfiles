use eyre::Result;

use super::client::OpenAlexClient;
use super::types::OAAuthorDetail;
use crate::cmd::search::OutputFormat;

pub async fn run(client: &OpenAlexClient, query: &str, format: OutputFormat) -> Result<()> {
    let is_id = query.starts_with('A') && query[1..].chars().all(|c| c.is_ascii_digit());

    if is_id {
        let author = client.author_detail(query).await?;
        match format {
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&author)?),
            OutputFormat::Plain => display_detail(&author),
        }
    } else {
        let resp = client.author_search(query, 10).await?;
        match format {
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
            OutputFormat::Plain => {
                let data = resp.results.unwrap_or_default();
                println!("{} authors\n", data.len());
                for (i, author) in data.iter().enumerate() {
                    display_brief(i + 1, author);
                }
            }
        }
    }

    Ok(())
}

fn display_detail(author: &OAAuthorDetail) {
    let name = author.display_name.as_deref().unwrap_or("Unknown");
    println!("{name}");
    if let Some(stats) = &author.summary_stats {
        println!("h-index: {}", stats.h_index.unwrap_or(0));
    }
    println!("Works: {}", author.works_count.unwrap_or(0));
    println!("Citations: {}", author.cited_by_count.unwrap_or(0));

    if let Some(insts) = &author.last_known_institutions {
        let names: Vec<&str> = insts
            .iter()
            .filter_map(|i| i.display_name.as_deref())
            .collect();
        if !names.is_empty() {
            println!("Institutions: {}", names.join(", "));
        }
    }

    if let Some(id) = &author.id {
        println!("OpenAlex: {id}");
    }
}

fn display_brief(idx: usize, author: &OAAuthorDetail) {
    let name = author.display_name.as_deref().unwrap_or("Unknown");
    let works = author.works_count.unwrap_or(0);
    let h = author
        .summary_stats
        .as_ref()
        .and_then(|s| s.h_index)
        .unwrap_or(0);
    let id = author.id.as_deref().unwrap_or("?");
    println!("{idx}. {name}  (h={h}, works={works}, id={id})");
}
