use eyre::Result;

use super::client::OpenAlexClient;
use crate::cmd::search::OutputFormat;

pub async fn run(
    client: &OpenAlexClient,
    field: &str,
    filter: &Option<String>,
    format: OutputFormat,
) -> Result<()> {
    let resp = client.group_by(field, filter).await?;

    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputFormat::Plain => {
            let groups = resp.group_by.unwrap_or_default();
            println!("{} groups\n", groups.len());
            for group in &groups {
                let key = group
                    .key_display_name
                    .as_deref()
                    .or(group.key.as_deref())
                    .unwrap_or("?");
                let count = group.count.unwrap_or(0);
                println!("  {key}: {count}");
            }
        }
    }

    Ok(())
}
