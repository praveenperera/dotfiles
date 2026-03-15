mod client;
mod db;
mod display;
mod doi;
mod download;
mod extract;
mod list;
mod open;
mod search;
mod types;

use clap::{Args, Subcommand};
use colored::Colorize;
use eyre::Result;

use crate::cmd::search::config;
use doi::normalize_doi;

#[derive(Debug, Clone, Subcommand)]
pub enum LibraryCmd {
    /// Download a paper by DOI
    #[command(visible_alias = "dl", arg_required_else_help = true)]
    Download(DownloadArgs),

    /// Full-text search across downloaded papers
    #[command(visible_alias = "s", arg_required_else_help = true)]
    Search(SearchArgs),

    /// List all downloaded papers
    #[command(visible_alias = "ls")]
    List,

    /// Open a paper's PDF in the default viewer
    #[command(visible_alias = "o", arg_required_else_help = true)]
    Open {
        /// DOI of the paper to open
        doi: String,
    },

    /// Show paper details and text stats
    #[command(visible_alias = "i", arg_required_else_help = true)]
    Info {
        /// DOI of the paper
        doi: String,
    },

    /// Remove a paper from the library
    #[command(visible_alias = "rm", arg_required_else_help = true)]
    Remove {
        /// DOI of the paper to remove
        doi: String,
    },

    /// Show or set Sci-Hub base URL
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Args)]
pub struct DownloadArgs {
    /// DOI of the paper to download
    pub doi: String,

    /// Re-download even if already in library
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Clone, Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Maximum results
    #[arg(short, long, default_value = "10")]
    pub limit: u32,
}

#[derive(Debug, Clone, Args)]
pub struct ConfigArgs {
    /// Set the Sci-Hub base URL
    #[arg(long = "set-url")]
    pub set_url: Option<String>,
}

pub async fn run_async(cmd: LibraryCmd) -> Result<()> {
    match cmd {
        LibraryCmd::Download(args) => download::run(&args.doi, args.force).await,
        LibraryCmd::Search(args) => search::run(&args.query, args.limit).await,
        LibraryCmd::List => list::run().await,
        LibraryCmd::Open { doi } => open::run(&doi).await,
        LibraryCmd::Info { doi } => run_info(&doi).await,
        LibraryCmd::Remove { doi } => run_remove(&doi).await,
        LibraryCmd::Config(args) => run_config(args),
    }
}

async fn run_info(doi_input: &str) -> Result<()> {
    let doi = normalize_doi(doi_input)?;
    let db_path = config::data_dir().join("papers.db");
    let db = db::PaperDb::open(&db_path).await?;

    match db.find_by_doi(&doi).await? {
        Some(paper) => display::display_paper_info(&paper),
        None => println!("{} paper not in library: {doi}", "✗".red()),
    }

    Ok(())
}

async fn run_remove(doi_input: &str) -> Result<()> {
    let doi = normalize_doi(doi_input)?;
    let db_path = config::data_dir().join("papers.db");
    let db = db::PaperDb::open(&db_path).await?;

    let data_dir = config::data_dir();

    match db.remove(&doi).await? {
        Some(paper) => {
            let pdf = std::path::Path::new(&paper.pdf_path);

            // only delete if within our managed directory
            if pdf.starts_with(&data_dir) {
                if let Err(e) = std::fs::remove_file(pdf) {
                    eprintln!("{} could not delete PDF: {e}", "⚠".yellow());
                }
            }

            println!(
                "{} removed: {}",
                "✓".green(),
                paper.title.as_deref().unwrap_or(&doi)
            );
        }
        None => println!("{} paper not in library: {doi}", "✗".red()),
    }

    Ok(())
}

fn run_config(args: ConfigArgs) -> Result<()> {
    if let Some(url) = args.set_url {
        config::save_scihub_url(&url)?;
        println!("{} Sci-Hub URL set to: {}", "✓".green(), url.cyan());
        return Ok(());
    }

    // show current config
    match config::get_scihub_url() {
        Some(url) => println!("Sci-Hub URL: {}", url.cyan()),
        None => println!(
            "{} Sci-Hub URL not configured — use {} or set {}",
            "·".dimmed(),
            "--set-url".bold(),
            "SCIHUB_URL".bold()
        ),
    }

    println!(
        "{} {}",
        "data dir:".dimmed(),
        config::data_dir().display().to_string().cyan()
    );

    Ok(())
}
