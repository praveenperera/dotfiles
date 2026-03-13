use eyre::Result;

use super::client::S2Client;
use super::display::display_paper_detail;
use crate::cmd::search::OutputFormat;

pub async fn run(client: &S2Client, id: &str, format: OutputFormat) -> Result<()> {
    let paper = client.paper_detail(id).await?;

    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&paper)?),
        OutputFormat::Plain => display_paper_detail(&paper),
    }

    Ok(())
}
