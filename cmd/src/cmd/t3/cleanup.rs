use chrono::{Duration, SecondsFormat, Utc};
use clap::Args;
use colored::Colorize;
use eyre::{bail, Result};

use super::server::Server;

#[derive(Debug, Clone, Args)]
pub struct CleanupArgs {
    /// Delete settled threads with no activity in this many days
    #[arg(long, default_value_t = 14)]
    older_than_days: i64,

    /// List the threads that would be deleted without changing T3
    #[arg(long)]
    dry_run: bool,
}

pub fn cleanup(args: CleanupArgs) -> Result<()> {
    if args.older_than_days < 1 {
        bail!("--older-than-days must be at least 1");
    }

    let cutoff = (Utc::now() - Duration::days(args.older_than_days))
        .to_rfc3339_opts(SecondsFormat::Millis, true);

    let server = Server::locate()?;
    let threads = server.db(true)?.stale_settled_threads(&cutoff)?;
    if threads.is_empty() {
        println!("no settled threads idle since {cutoff}");
        return Ok(());
    }

    for thread in &threads {
        let project = thread.project.as_deref().unwrap_or("?");
        println!(
            "{}  {}  {project}  {}",
            thread.updated_at, thread.id, thread.title
        );
    }

    let verb = if args.dry_run {
        "would delete"
    } else {
        "deleting"
    };

    println!(
        "{verb} {} settled threads idle since {cutoff}",
        threads.len()
    );

    if args.dry_run {
        return Ok(());
    }

    // every delete opens its own socket, so a long list needs more than the
    // import's 5 minute token
    let client = server.client("cmd t3 cleanup", "30m")?;
    let mut failed = 0;
    for thread in &threads {
        if let Err(error) = client.delete_thread(&thread.id) {
            failed += 1;
            eprintln!(
                "{}",
                format!("failed to delete {}: {error}", thread.id).red()
            );
        }
    }

    let deleted = threads.len() - failed;
    println!("{}", format!("deleted {deleted} threads").green());
    if failed > 0 {
        bail!("{failed} threads could not be deleted");
    }

    Ok(())
}
