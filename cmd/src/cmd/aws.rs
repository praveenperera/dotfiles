use clap::{Parser, Subcommand};
use eyre::Result;

use crate::runtime;

mod billing;

pub use billing::BillingArgs;

/// AWS command arguments
#[derive(Debug, Clone, Parser)]
pub struct Aws {
    /// AWS operation to run
    #[command(subcommand)]
    pub subcommand: AwsCmd,
}

/// AWS operations
#[derive(Debug, Clone, Subcommand)]
pub enum AwsCmd {
    /// Show service charges for the current UTC month
    Billing(#[command(flatten)] BillingArgs),
}

/// Runs an AWS command with parsed arguments
pub fn run_with_flags(flags: Aws) -> Result<()> {
    runtime::block_on(run_async(flags))?
}

async fn run_async(flags: Aws) -> Result<()> {
    match flags.subcommand {
        AwsCmd::Billing(args) => run_billing(args).await,
    }
}

pub(crate) async fn run_billing(args: BillingArgs) -> Result<()> {
    billing::run(args).await
}

pub(crate) async fn billing_summary() -> Result<crate::cmd::billing::ProviderBillingSummary> {
    billing::summary().await
}
