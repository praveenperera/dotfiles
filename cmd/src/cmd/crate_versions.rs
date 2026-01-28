use clap::{Parser, ValueEnum};
use eyre::Result;
use futures::future::join_all;
use xshell::Shell;

use crate::crates_io::CratesIoClient;

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Toml,
    Json,
    Plain,
}

#[derive(Debug, Clone, Parser)]
pub struct CrateVersions {
    /// Crate names to look up
    #[arg(required = true)]
    pub crates: Vec<String>,

    /// Output format
    #[arg(short, long, default_value = "toml")]
    pub format: OutputFormat,

    /// Use exact version pinning (=1.0.0 instead of 1.0.0)
    #[arg(short, long)]
    pub exact: bool,
}

pub fn run_with_flags(_sh: &Shell, flags: CrateVersions) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_async(flags))
}

async fn run_async(flags: CrateVersions) -> Result<()> {
    let client = CratesIoClient::new()?;

    let futures = flags
        .crates
        .iter()
        .map(|name| fetch_version(&client, name));

    let results: Vec<_> = join_all(futures).await;

    let mut versions: Vec<(String, String)> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for (name, result) in flags.crates.iter().zip(results) {
        match result {
            Ok(version) => versions.push((name.clone(), version)),
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }

    match flags.format {
        OutputFormat::Toml => print_toml(&versions, flags.exact),
        OutputFormat::Json => print_json(&versions, flags.exact),
        OutputFormat::Plain => print_plain(&versions, flags.exact),
    }

    if !errors.is_empty() {
        eprintln!("\nErrors:");
        for error in errors {
            eprintln!("  {error}");
        }
    }

    Ok(())
}

async fn fetch_version(client: &CratesIoClient, name: &str) -> Result<String> {
    client.get_latest_version(name).await
}

fn format_version(version: &str, exact: bool) -> String {
    if exact {
        format!("={version}")
    } else {
        version.to_string()
    }
}

fn print_toml(versions: &[(String, String)], exact: bool) {
    for (name, version) in versions {
        println!("{name} = \"{}\"", format_version(version, exact));
    }
}

fn print_json(versions: &[(String, String)], exact: bool) {
    let map: serde_json::Map<String, serde_json::Value> = versions
        .iter()
        .map(|(name, version)| {
            (
                name.clone(),
                serde_json::Value::String(format_version(version, exact)),
            )
        })
        .collect();

    let json = serde_json::to_string_pretty(&serde_json::Value::Object(map)).unwrap();
    println!("{json}");
}

fn print_plain(versions: &[(String, String)], exact: bool) {
    for (name, version) in versions {
        println!("{name} {}", format_version(version, exact));
    }
}
