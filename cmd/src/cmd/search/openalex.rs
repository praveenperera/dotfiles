mod author;
mod citations;
pub mod client;
mod display;
mod group_by;
mod institutions;
mod paper;
mod references;
mod search;
mod topics;
pub mod types;

use clap::{Args, Subcommand};
use eyre::Result;

use super::OutputFormat;
use client::OpenAlexClient;

#[derive(Debug, Clone, Args)]
pub struct CommonFilters {
    /// Year or range (e.g. 2020, 2020-2024, 2020-)
    #[arg(long)]
    pub year: Option<String>,

    /// Field/topic filter
    #[arg(long)]
    pub field: Option<String>,

    /// Minimum citation count
    #[arg(long)]
    pub min_citations: Option<u32>,

    /// Only open access works
    #[arg(long)]
    pub open_access: bool,

    /// Maximum results
    #[arg(short, long, default_value = "10")]
    pub limit: u32,

    /// Result offset for pagination
    #[arg(long, default_value = "1")]
    pub offset: u32,

    /// Output format
    #[arg(short = 'F', long, default_value = "plain")]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Subcommand)]
pub enum OpenAlexCmd {
    /// Search for works/papers
    #[command(visible_alias = "s", arg_required_else_help = true)]
    Search {
        /// Search query
        query: String,

        /// Use semantic/embedding search (max 50 results)
        #[arg(long)]
        semantic: bool,

        /// Sort results (e.g. cited_by_count:desc, publication_date:desc)
        #[arg(long)]
        sort: Option<String>,

        /// Work type filter (e.g. article, book, dissertation)
        #[arg(long)]
        work_type: Option<String>,

        /// Institution filter (OpenAlex ID)
        #[arg(long)]
        institution: Option<String>,

        /// Source/journal filter (OpenAlex ID)
        #[arg(long)]
        source: Option<String>,

        /// Topic filter (OpenAlex ID)
        #[arg(long)]
        topic: Option<String>,

        /// Raw filter string (e.g. "is_oa:true,language:en")
        #[arg(long)]
        filter: Option<String>,

        #[command(flatten)]
        filters: CommonFilters,
    },

    /// Get work details by ID (OpenAlex ID, DOI, PMID)
    #[command(visible_alias = "p", arg_required_else_help = true)]
    Paper {
        /// Work identifier
        id: String,

        #[arg(short = 'F', long, default_value = "plain")]
        format: OutputFormat,
    },

    /// Works that cite this work
    #[command(visible_alias = "c", arg_required_else_help = true)]
    Citations {
        /// Work identifier (OpenAlex ID)
        id: String,

        #[command(flatten)]
        filters: CommonFilters,
    },

    /// Works this work references
    #[command(visible_alias = "r", arg_required_else_help = true)]
    References {
        /// Work identifier (OpenAlex ID)
        id: String,

        #[command(flatten)]
        filters: CommonFilters,
    },

    /// Search or get author details
    #[command(visible_alias = "a", arg_required_else_help = true)]
    Author {
        /// Author name or OpenAlex ID
        query: String,

        #[arg(short = 'F', long, default_value = "plain")]
        format: OutputFormat,
    },

    /// Search institutions
    #[command(visible_alias = "i", arg_required_else_help = true)]
    Institutions {
        /// Search query
        query: String,

        #[arg(short, long, default_value = "10")]
        limit: u32,

        #[arg(short = 'F', long, default_value = "plain")]
        format: OutputFormat,
    },

    /// Search topics
    #[command(visible_alias = "t", arg_required_else_help = true)]
    Topics {
        /// Search query
        query: String,

        #[arg(short, long, default_value = "10")]
        limit: u32,

        #[arg(short = 'F', long, default_value = "plain")]
        format: OutputFormat,
    },

    /// Aggregate works by a field (e.g. oa_status, publication_year)
    #[command(visible_alias = "g", arg_required_else_help = true)]
    GroupBy {
        /// Field to group by
        field: String,

        /// Filter works before grouping
        #[arg(long)]
        filter: Option<String>,

        #[arg(short = 'F', long, default_value = "plain")]
        format: OutputFormat,
    },
}

pub async fn run_async(cmd: OpenAlexCmd) -> Result<()> {
    let client = OpenAlexClient::new()?;

    match cmd {
        OpenAlexCmd::Search {
            query,
            semantic,
            sort,
            work_type,
            institution,
            source,
            topic,
            filter,
            filters,
        } => {
            let params = types::WorkSearchParams {
                query,
                semantic,
                sort,
                extra_filter: filter,
                work_type,
                institution,
                source,
                topic,
                filters,
            };
            search::run(&client, params).await
        }

        OpenAlexCmd::Paper { id, format } => paper::run(&client, &id, format).await,

        OpenAlexCmd::Citations { id, filters } => citations::run(&client, &id, &filters).await,

        OpenAlexCmd::References { id, filters } => references::run(&client, &id, &filters).await,

        OpenAlexCmd::Author { query, format } => author::run(&client, &query, format).await,

        OpenAlexCmd::Institutions {
            query,
            limit,
            format,
        } => institutions::run(&client, &query, limit, format).await,

        OpenAlexCmd::Topics {
            query,
            limit,
            format,
        } => topics::run(&client, &query, limit, format).await,

        OpenAlexCmd::GroupBy {
            field,
            filter,
            format,
        } => group_by::run(&client, &field, &filter, format).await,
    }
}
