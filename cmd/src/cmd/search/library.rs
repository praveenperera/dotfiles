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
    List(ListArgs),

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

    /// Output extracted paper text to stdout
    #[command(visible_alias = "r", arg_required_else_help = true)]
    Read {
        /// DOI of the paper
        doi: String,
    },

    /// Manage paper tags
    #[command(visible_alias = "t", subcommand)]
    Tag(TagCmd),

    /// Re-extract text for all papers using pdftotext
    Reindex,

    /// Show or set Sci-Hub base URL
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum TagCmd {
    /// Add tag(s) to a paper
    #[command(arg_required_else_help = true)]
    Add {
        /// DOI of the paper
        doi: String,
        /// Tag(s) to add
        #[arg(required = true)]
        tags: Vec<String>,
    },

    /// Remove tag(s) from a paper
    #[command(visible_alias = "rm", arg_required_else_help = true)]
    Remove {
        /// DOI of the paper
        doi: String,
        /// Tag(s) to remove
        #[arg(required = true)]
        tags: Vec<String>,
    },

    /// List all tags with paper counts
    #[command(visible_alias = "ls")]
    List,
}

#[derive(Debug, Clone, Args)]
pub struct DownloadArgs {
    /// DOI of the paper to download
    pub doi: String,

    /// Re-download even if already in library
    #[arg(long)]
    pub force: bool,

    /// Tag(s) to apply to the downloaded paper
    #[arg(short, long)]
    pub tag: Vec<String>,
}

#[derive(Debug, Clone, Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Maximum results
    #[arg(short, long, default_value = "10")]
    pub limit: u32,

    /// Filter results by tag
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct ListArgs {
    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct ConfigArgs {
    /// Set the Sci-Hub base URL
    #[arg(long = "set-url")]
    pub set_url: Option<String>,
}

pub async fn run_async(cmd: LibraryCmd) -> Result<()> {
    match cmd {
        LibraryCmd::Download(args) => download::run(&args.doi, args.force, &args.tag).await,
        LibraryCmd::Search(args) => search::run(&args.query, args.limit, args.tag.as_deref()).await,
        LibraryCmd::List(args) => list::run(args.tag.as_deref()).await,
        LibraryCmd::Open { doi } => open::run(&doi).await,
        LibraryCmd::Info { doi } => run_info(&doi).await,
        LibraryCmd::Remove { doi } => run_remove(&doi).await,
        LibraryCmd::Read { doi } => run_read(&doi).await,
        LibraryCmd::Tag(cmd) => run_tag(cmd).await,
        LibraryCmd::Reindex => run_reindex().await,
        LibraryCmd::Config(args) => run_config(args),
    }
}

async fn run_info(doi_input: &str) -> Result<()> {
    let doi = normalize_doi(doi_input)?;
    let db_path = config::data_dir().join("papers.db");
    let db = db::PaperDb::open(&db_path).await?;

    match db.find_by_doi(&doi).await? {
        Some(paper) => {
            let tags = db.get_tags(&doi).await?;
            display::display_paper_info(&paper, &tags);
        }
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

async fn run_read(doi_input: &str) -> Result<()> {
    let doi = normalize_doi(doi_input)?;
    let db_path = config::data_dir().join("papers.db");
    let db = db::PaperDb::open(&db_path).await?;

    let paper = match db.find_by_doi(&doi).await? {
        Some(p) => p,
        None => {
            eprintln!("{} not in library, downloading first...", "·".dimmed());
            download::run(&doi, false, &[]).await?;
            db.find_by_doi(&doi)
                .await?
                .ok_or_else(|| eyre::eyre!("download succeeded but paper not found in db"))?
        }
    };

    let path = std::path::Path::new(&paper.pdf_path);
    if !path.exists() {
        eyre::bail!("PDF file missing: {}", paper.pdf_path);
    }

    let text = extract::extract_text(path)?;
    print!("{text}");

    Ok(())
}

async fn run_reindex() -> Result<()> {
    let db_path = config::data_dir().join("papers.db");
    let db = db::PaperDb::open(&db_path).await?;
    let papers = db.list_all().await?;

    if papers.is_empty() {
        println!("{} library is empty", "·".dimmed());
        return Ok(());
    }

    println!(
        "{} re-extracting text for {} papers...",
        "↻".cyan(),
        papers.len()
    );

    let mut success = 0u32;
    let mut failed = 0u32;

    for paper in &papers {
        let path = std::path::Path::new(&paper.pdf_path);
        let display = paper.title.as_deref().unwrap_or(&paper.doi);

        if !path.exists() {
            eprintln!("{} PDF missing, skipping: {display}", "⚠".yellow());
            failed += 1;
            continue;
        }

        match extract::extract_text(path) {
            Ok(text) => {
                let len = text.len() as i64;
                db.update_full_text(&paper.doi, &text, len).await?;
                println!("{} {display} ({len} chars)", "✓".green());
                success += 1;
            }
            Err(e) => {
                eprintln!("{} {display}: {e}", "✗".red());
                failed += 1;
            }
        }
    }

    println!(
        "\n{} reindex complete: {success} updated, {failed} failed",
        "✓".green()
    );

    Ok(())
}

async fn run_tag(cmd: TagCmd) -> Result<()> {
    let db = db::PaperDb::open(&config::data_dir().join("papers.db")).await?;

    match cmd {
        TagCmd::Add { doi, tags } => {
            let doi = normalize_doi(&doi)?;
            let tags: Vec<String> = tags.iter().map(|t| t.trim().to_lowercase()).collect();
            db.add_tags(&doi, &tags).await?;
            println!("{} added tags: {}", "✓".green(), tags.join(", ").cyan());
        }
        TagCmd::Remove { doi, tags } => {
            let doi = normalize_doi(&doi)?;
            let tags: Vec<String> = tags.iter().map(|t| t.trim().to_lowercase()).collect();
            db.remove_tags(&doi, &tags).await?;
            println!("{} removed tags: {}", "✓".green(), tags.join(", "));
        }
        TagCmd::List => {
            let tags = db.list_all_tags().await?;
            if tags.is_empty() {
                println!("{}", "no tags".dimmed());
            } else {
                for (tag, count) in &tags {
                    let label = if *count == 1 { "paper" } else { "papers" };
                    println!("  {} ({count} {label})", tag.cyan());
                }
            }
        }
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
