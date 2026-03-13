use eyre::Result;

use super::client::OpenAlexClient;
use super::display::display_work;
use super::types::WorkSearchParams;
use crate::cmd::search::OutputFormat;

pub async fn run(client: &OpenAlexClient, params: WorkSearchParams) -> Result<()> {
    let format = params.filters.format;
    let resp = client.search_works(&params).await?;

    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputFormat::Plain => {
            let works = resp.results.unwrap_or_default();
            let total = resp.meta.as_ref().and_then(|m| m.count).unwrap_or(0);
            println!("{} of {total} results\n", works.len());
            for (i, work) in works.iter().enumerate() {
                display_work(i + 1, work);
                println!();
            }
        }
    }

    Ok(())
}
