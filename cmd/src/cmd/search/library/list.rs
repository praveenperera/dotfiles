use eyre::Result;

use super::db::PaperDb;
use super::display::display_paper_list;
use crate::cmd::search::config;

pub async fn run() -> Result<()> {
    let db = PaperDb::open(&config::data_dir().join("papers.db")).await?;
    let papers = db.list_all().await?;
    display_paper_list(&papers);
    Ok(())
}
