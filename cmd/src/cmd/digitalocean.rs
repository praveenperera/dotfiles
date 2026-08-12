use clap::{Parser, Subcommand};
use eyre::Result;

use crate::runtime;

mod billing;

pub use billing::BillingArgs;

/// DigitalOcean command arguments
#[derive(Debug, Clone, Parser)]
pub struct DigitalOcean {
    /// DigitalOcean operation to run
    #[command(subcommand)]
    pub subcommand: DigitalOceanCmd,
}

/// DigitalOcean operations
#[derive(Debug, Clone, Subcommand)]
pub enum DigitalOceanCmd {
    /// Show product charges for the current invoice preview
    Billing(#[command(flatten)] BillingArgs),
}

/// Runs a DigitalOcean command with parsed arguments
pub fn run_with_flags(flags: DigitalOcean) -> Result<()> {
    runtime::block_on(run_async(flags))?
}

async fn run_async(flags: DigitalOcean) -> Result<()> {
    match flags.subcommand {
        DigitalOceanCmd::Billing(args) => run_billing(args).await,
    }
}

pub(crate) async fn run_billing(args: BillingArgs) -> Result<()> {
    billing::run(args).await
}

pub(crate) async fn billing_summary() -> Result<crate::cmd::billing::ProviderBillingSummary> {
    billing::summary().await
}
