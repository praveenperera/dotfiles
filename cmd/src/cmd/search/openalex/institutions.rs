use eyre::Result;

use super::client::OpenAlexClient;
use crate::cmd::search::OutputFormat;

pub async fn run(
    client: &OpenAlexClient,
    query: &str,
    limit: u32,
    format: OutputFormat,
) -> Result<()> {
    let resp = client.search_institutions(query, limit).await?;

    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputFormat::Plain => {
            let data = resp.results.unwrap_or_default();
            println!("{} institutions\n", data.len());
            for (i, inst) in data.iter().enumerate() {
                let name = inst.display_name.as_deref().unwrap_or("Unknown");
                let country = inst.country_code.as_deref().unwrap_or("??");
                let id = inst.id.as_deref().unwrap_or("");
                println!("{}. {} ({country})  {id}", i + 1, name);
            }
        }
    }

    Ok(())
}
