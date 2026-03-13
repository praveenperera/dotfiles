use eyre::Result;

use super::client::S2Client;
use super::display::display_paper;
use super::CommonFilters;
use crate::cmd::search::OutputFormat;

pub async fn run(client: &S2Client, id: &str, pool: &str, filters: &CommonFilters) -> Result<()> {
    let resp = client.recommend(id, pool, filters.limit).await?;

    match filters.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputFormat::Plain => {
            let papers = resp.recommended_papers.unwrap_or_default();
            println!("{} recommendations\n", papers.len());
            for (i, paper) in papers.iter().enumerate() {
                display_paper(i + 1, paper);
                println!();
            }
        }
    }

    Ok(())
}
