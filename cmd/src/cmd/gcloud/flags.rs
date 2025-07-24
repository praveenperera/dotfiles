use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
pub struct Gcloud {
    #[command(subcommand)]
    pub subcommand: GcloudCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum GcloudCmd {
    /// Google Cloud login
    #[command(arg_required_else_help = true)]
    Login {
        /// Project to login to
        project: String,
    },

    /// Google Cloud switch project
    #[command(name = "switch-project", visible_alias = "sp", arg_required_else_help = true)]
    SwitchProject {
        /// Project to switch to
        project: String,
    },

    /// Google Cloud switch cluster
    #[command(name = "switch-cluster", visible_alias = "sc", arg_required_else_help = true)]
    SwitchCluster {
        /// Project containing the cluster
        project: String,
        /// Cluster name to switch to
        cluster: String,
    },
}

