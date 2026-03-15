use eyre::Result;

use super::db::PaperDb;
use super::display::display_search_results;
use crate::cmd::search::config;

pub async fn run(query: &str, limit: u32) -> Result<()> {
    let db = PaperDb::open(&config::data_dir().join("papers.db")).await?;
    let results = db.search(query, limit).await?;
    display_search_results(&results);
    Ok(())
}
