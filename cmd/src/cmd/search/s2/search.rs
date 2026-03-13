use eyre::Result;

use super::client::S2Client;
use super::display::display_paper;
use super::CommonFilters;
use crate::cmd::search::OutputFormat;

pub async fn run(
    client: &S2Client,
    query: &str,
    filters: &CommonFilters,
    venue: &Option<String>,
    pub_type: &Option<String>,
) -> Result<()> {
    let resp = client
        .search_papers(query, filters, venue, pub_type)
        .await?;

    match filters.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputFormat::Plain => {
            let papers = resp.data.unwrap_or_default();
            let total = resp.total.unwrap_or(0);
            println!("{} of {total} results\n", papers.len());
            for (i, paper) in papers.iter().enumerate() {
                display_paper(i + 1, paper);
                println!();
            }
        }
    }

    Ok(())
}
