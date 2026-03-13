use eyre::Result;

use super::client::S2Client;
use crate::cmd::search::{truncate, OutputFormat};

pub async fn run(client: &S2Client, query: &str, limit: u32, format: OutputFormat) -> Result<()> {
    let resp = client.snippets(query, limit).await?;

    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&resp)?),
        OutputFormat::Plain => {
            let data = resp.data.unwrap_or_default();
            println!("{} snippets\n", data.len());
            for (i, snippet) in data.iter().enumerate() {
                let text = snippet.snippet_text.as_deref().unwrap_or("");
                let title = snippet
                    .paper
                    .as_ref()
                    .and_then(|p| p.title.as_deref())
                    .unwrap_or("Unknown");
                println!("{}. [{}]", i + 1, title);
                println!("   {}", truncate(text, 200));
                println!();
            }
        }
    }

    Ok(())
}
