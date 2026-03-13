use eyre::Result;

use super::client::S2Client;
use super::display::display_paper;
use super::CommonFilters;
use crate::cmd::search::OutputFormat;

pub async fn run(client: &S2Client, id: &str, filters: &CommonFilters) -> Result<()> {
    let resp = client.references(id, filters).await?;

    match filters.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputFormat::Plain => {
            let data = resp.data.unwrap_or_default();
            println!("{} references\n", data.len());
            for (i, reference) in data.iter().enumerate() {
                if let Some(paper) = &reference.cited_paper {
                    display_paper(i + 1, paper);
                    println!();
                }
            }
        }
    }

    Ok(())
}
