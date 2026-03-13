use eyre::Result;

use super::client::S2Client;
use super::CommonFilters;
use crate::cmd::search::OutputFormat;

pub async fn run(client: &S2Client, id: &str, filters: &CommonFilters) -> Result<()> {
    let resp = client.citations(id, filters).await?;

    match filters.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputFormat::Plain => {
            let data = resp.data.unwrap_or_default();
            println!("{} citations\n", data.len());
            for (i, cite) in data.iter().enumerate() {
                if let Some(paper) = &cite.citing_paper {
                    let influential = if cite.is_influential == Some(true) {
                        " [influential]"
                    } else {
                        ""
                    };
                    let title = paper.title.as_deref().unwrap_or("Untitled");
                    let year = paper.year.map(|y| format!("({y})")).unwrap_or_default();
                    println!("{}. {title} {year}{influential}", i + 1);

                    if let Some(intents) = &cite.intents {
                        if !intents.is_empty() {
                            println!("   Intents: {}", intents.join(", "));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
