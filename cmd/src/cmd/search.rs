pub mod config;
pub mod openalex;
pub mod s2;

use clap::{Parser, Subcommand, ValueEnum};
use eyre::Result;
use std::ffi::OsString;
use xshell::Shell;

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Plain,
    Json,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "aps",
    about = "Academic paper search (Semantic Scholar & OpenAlex)"
)]
pub struct Search {
    #[command(subcommand)]
    pub subcommand: SearchCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SearchCmd {
    /// Semantic Scholar search
    #[command(visible_alias = "s2", arg_required_else_help = true)]
    SemanticScholar {
        #[command(subcommand)]
        subcommand: s2::S2Cmd,
    },

    /// OpenAlex search
    #[command(visible_alias = "oa", arg_required_else_help = true)]
    Openalex {
        #[command(subcommand)]
        subcommand: openalex::OpenAlexCmd,
    },

    /// Save API keys from env vars to ~/.config/aps/
    Login,

    /// Show current auth status
    Status,
}

pub fn run(_sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = Search::parse_from(args);
    run_with_flags(flags)
}

pub fn run_with_flags(flags: Search) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_async(flags))
}

async fn run_async(flags: Search) -> Result<()> {
    match flags.subcommand {
        SearchCmd::SemanticScholar { subcommand } => s2::run_async(subcommand).await,
        SearchCmd::Openalex { subcommand } => openalex::run_async(subcommand).await,
        SearchCmd::Login => config::login(),
        SearchCmd::Status => config::status(),
    }
}

pub fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }

    match s[..max_len].rfind(' ') {
        Some(pos) => &s[..pos],
        None => &s[..max_len],
    }
}

pub fn format_authors(authors: &[String], max: usize) -> String {
    if authors.is_empty() {
        return "Unknown".to_string();
    }

    // extract last names
    let last_names: Vec<&str> = authors
        .iter()
        .take(max)
        .map(|a| a.rsplit(' ').next().unwrap_or(a))
        .collect();

    let mut result = last_names.join(", ");
    if authors.len() > max {
        result.push_str(" et al.");
    }

    result
}
