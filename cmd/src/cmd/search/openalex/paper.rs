use eyre::Result;

use super::client::OpenAlexClient;
use super::display::display_work_detail;
use crate::cmd::search::OutputFormat;

pub async fn run(client: &OpenAlexClient, id: &str, format: OutputFormat) -> Result<()> {
    let work = client.work_detail(id).await?;

    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&work)?),
        OutputFormat::Plain => display_work_detail(&work),
    }

    Ok(())
}
