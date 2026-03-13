mod author;
mod citations;
pub mod client;
mod display;
mod match_title;
mod paper;
mod recommend;
mod references;
mod search;
mod snippets;
pub mod types;

use clap::{Args, Subcommand};
use eyre::Result;

use super::OutputFormat;
use client::S2Client;

#[derive(Debug, Clone, Args)]
pub struct CommonFilters {
    /// Year or range (e.g. 2020, 2020-2024, 2020-)
    #[arg(long)]
    pub year: Option<String>,

    /// Field of study (e.g. "Computer Science", "Medicine")
    #[arg(long)]
    pub field: Option<String>,

    /// Minimum citation count
    #[arg(long)]
    pub min_citations: Option<u32>,

    /// Only open access papers
    #[arg(long)]
    pub open_access: bool,

    /// Maximum results
    #[arg(short, long, default_value = "10")]
    pub limit: u32,

    /// Result offset for pagination
    #[arg(long, default_value = "0")]
    pub offset: u32,

    /// Output format
    #[arg(short = 'F', long, default_value = "plain")]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Subcommand)]
pub enum S2Cmd {
    /// Search for papers
    #[command(visible_alias = "s", arg_required_else_help = true)]
    Search {
        /// Search query
        query: String,

        /// Venue filter (e.g. "NeurIPS", "Nature")
        #[arg(long)]
        venue: Option<String>,

        /// Publication type (e.g. JournalArticle, Conference, Review)
        #[arg(long)]
        pub_type: Option<String>,

        /// Use semantic/embedding search
        #[arg(long)]
        semantic: bool,

        #[command(flatten)]
        filters: CommonFilters,
    },

    /// Get paper details by ID (S2 ID, DOI:..., ARXIV:..., URL)
    #[command(visible_alias = "p", arg_required_else_help = true)]
    Paper {
        /// Paper identifier
        id: String,

        #[arg(short = 'F', long, default_value = "plain")]
        format: OutputFormat,
    },

    /// Papers that cite this paper
    #[command(visible_alias = "c", arg_required_else_help = true)]
    Citations {
        /// Paper identifier
        id: String,

        #[command(flatten)]
        filters: CommonFilters,
    },

    /// Papers this paper references
    #[command(visible_alias = "r", arg_required_else_help = true)]
    References {
        /// Paper identifier
        id: String,

        #[command(flatten)]
        filters: CommonFilters,
    },

    /// Search or get author details
    #[command(visible_alias = "a", arg_required_else_help = true)]
    Author {
        /// Author name or ID
        query: String,

        #[arg(short = 'F', long, default_value = "plain")]
        format: OutputFormat,
    },

    /// Paper recommendations via SPECTER embeddings
    #[command(visible_alias = "rec", arg_required_else_help = true)]
    Recommend {
        /// Paper identifier to get recommendations for
        id: String,

        /// Recommendation pool
        #[arg(long, default_value = "recent")]
        pool: String,

        #[command(flatten)]
        filters: CommonFilters,
    },

    /// Full-text passage search across S2ORC
    #[command(visible_alias = "snip", arg_required_else_help = true)]
    Snippets {
        /// Search query
        query: String,

        #[arg(short, long, default_value = "5")]
        limit: u32,

        #[arg(short = 'F', long, default_value = "plain")]
        format: OutputFormat,
    },

    /// Find closest paper by exact title match
    #[command(visible_alias = "m", arg_required_else_help = true)]
    Match {
        /// Paper title
        title: String,

        #[arg(short = 'F', long, default_value = "plain")]
        format: OutputFormat,
    },
}

pub async fn run_async(cmd: S2Cmd) -> Result<()> {
    let client = S2Client::new()?;

    match cmd {
        S2Cmd::Search {
            query,
            venue,
            pub_type,
            semantic: _,
            filters,
        } => search::run(&client, &query, &filters, &venue, &pub_type).await,

        S2Cmd::Paper { id, format } => paper::run(&client, &id, format).await,

        S2Cmd::Citations { id, filters } => citations::run(&client, &id, &filters).await,

        S2Cmd::References { id, filters } => references::run(&client, &id, &filters).await,

        S2Cmd::Author { query, format } => author::run(&client, &query, format).await,

        S2Cmd::Recommend { id, pool, filters } => {
            recommend::run(&client, &id, &pool, &filters).await
        }

        S2Cmd::Snippets {
            query,
            limit,
            format,
        } => snippets::run(&client, &query, limit, format).await,

        S2Cmd::Match { title, format } => match_title::run(&client, &title, format).await,
    }
}
