use eyre::Result;

use super::client::OpenAlexClient;
use super::display::display_work;
use super::CommonFilters;
use crate::cmd::search::OutputFormat;

pub async fn run(client: &OpenAlexClient, id: &str, filters: &CommonFilters) -> Result<()> {
    let resp = client.citations(id, filters).await?;

    match filters.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputFormat::Plain => {
            let works = resp.results.unwrap_or_default();
            let total = resp.meta.as_ref().and_then(|m| m.count).unwrap_or(0);
            println!("{} of {total} citations\n", works.len());
            for (i, work) in works.iter().enumerate() {
                display_work(i + 1, work);
                println!();
            }
        }
    }

    Ok(())
}
