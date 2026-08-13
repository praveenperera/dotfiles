use clap::{Parser, Subcommand};
use eyre::Result;

use crate::runtime;

mod billing;

pub use billing::BillingArgs;

/// Modal command arguments
#[derive(Debug, Clone, Parser)]
pub struct Modal {
    /// Modal operation to run
    #[command(subcommand)]
    pub subcommand: ModalCmd,
}

/// Modal operations
#[derive(Debug, Clone, Subcommand)]
pub enum ModalCmd {
    /// Show workspace costs for the current billing month
    Billing(#[command(flatten)] BillingArgs),
}

/// Runs a Modal command with parsed arguments
pub fn run_with_flags(flags: Modal) -> Result<()> {
    runtime::block_on(run_async(flags))?
}

async fn run_async(flags: Modal) -> Result<()> {
    match flags.subcommand {
        ModalCmd::Billing(args) => run_billing(args).await,
    }
}

pub(crate) async fn run_billing(args: BillingArgs) -> Result<()> {
    billing::run(args).await
}

pub(crate) async fn billing_summary() -> Result<crate::cmd::billing::ProviderBillingSummary> {
    billing::summary().await
}
