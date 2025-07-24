use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
pub struct Gcloud {
    #[command(subcommand)]
    pub subcommand: GcloudCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum GcloudCmd {
    /// Google Cloud login
    Login {
        /// Project to login to
        project: String,
    },

    /// Google Cloud switch project
    #[command(name = "switch-project", visible_alias = "sp")]
    SwitchProject {
        /// Project to switch to
        project: String,
    },

    /// Google Cloud switch cluster
    #[command(name = "switch-cluster", visible_alias = "sc")]
    SwitchCluster {
        /// Project containing the cluster
        project: String,
        /// Cluster name to switch to
        cluster: String,
    },
}

