use eyre::Result;

use super::client::OpenAlexClient;
use crate::cmd::search::{truncate, OutputFormat};

pub async fn run(
    client: &OpenAlexClient,
    query: &str,
    limit: u32,
    format: OutputFormat,
) -> Result<()> {
    let resp = client.search_topics(query, limit).await?;

    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputFormat::Plain => {
            let data = resp.results.unwrap_or_default();
            println!("{} topics\n", data.len());
            for (i, topic) in data.iter().enumerate() {
                let name = topic.display_name.as_deref().unwrap_or("Unknown");
                let works = topic.works_count.unwrap_or(0);
                println!("{}. {}  (works: {works})", i + 1, name);
                if let Some(desc) = &topic.description {
                    println!("   {}", truncate(desc, 120));
                }
            }
        }
    }

    Ok(())
}
