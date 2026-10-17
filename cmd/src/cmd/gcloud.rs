use clap::{Parser, Subcommand};
use eyre::Result;
use eyre::WrapErr;

use eyre::eyre;
use xshell::cmd;
use xshell::Shell;

use crate::SECRETS_DIR;

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
    #[command(
        name = "switch-project",
        visible_alias = "sp",
        arg_required_else_help = true
    )]
    SwitchProject {
        /// Project to switch to
        project: String,
    },

    /// Google Cloud switch cluster
    #[command(
        name = "switch-cluster",
        visible_alias = "sc",
        arg_required_else_help = true
    )]
    SwitchCluster {
        /// Project containing the cluster
        project: String,
        /// Cluster name to switch to
        cluster: String,
    },
}

#[derive(serde::Deserialize)]
struct GcloudSecrets {
    account: String,
    clusters: Vec<GcloudCluster>,
}

#[derive(Debug, serde::Deserialize)]
struct GcloudCluster {
    name: String,
    region: String,
    project: String,
}

/// Projects with a `<project>.yaml` file in the embedded secrets dir
fn projects() -> Vec<&'static str> {
    SECRETS_DIR
        .files()
        .map(|file| file.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yaml")
        })
        .filter_map(|path| path.file_stem()?.to_str())
        .collect()
}

fn gcloud_secret(project: &str) -> Result<GcloudSecrets> {
    let file = format!("{project}.yaml");

    let Some(file_contents) = SECRETS_DIR.get_file(&file) else {
        // CI release builds embed an empty secrets dir, so point at the local build instead
        let projects = projects();
        if projects.is_empty() {
            return Err(eyre!(
                "no secrets for {project}: this binary was built without secrets, build it locally with `./release` in cmd/"
            ));
        }

        return Err(eyre!(
            "no secrets for {project}, available projects: {}",
            projects.join(", ")
        ));
    };

    let file_yaml = file_contents
        .contents_utf8()
        .ok_or_else(|| eyre!("failed to read {file}"))?;

    let secret = serde_yaml::from_str::<GcloudSecrets>(file_yaml)
        .wrap_err_with(|| format!("failed to parse {file}"))?;

    Ok(secret)
}

pub fn run_with_flags(sh: &Shell, flags: Gcloud) -> Result<()> {
    match flags.subcommand {
        GcloudCmd::Login { project } => {
            login(sh, &project)?;
        }
        GcloudCmd::SwitchProject { project } => {
            switch_project(sh, &project)?;
        }
        GcloudCmd::SwitchCluster { project, cluster } => {
            switch_cluster(sh, &project, &cluster)?;
        }
    }

    Ok(())
}

pub fn login(sh: &Shell, project: &str) -> Result<()> {
    login_with_secret(sh, &gcloud_secret(project)?)
}

fn login_with_secret(sh: &Shell, secret: &GcloudSecrets) -> Result<()> {
    let account = &secret.account;
    cmd!(sh, "gcloud config set account {account}").run()?;

    Ok(())
}

pub fn switch_project(sh: &Shell, project: &str) -> Result<()> {
    let secret = gcloud_secret(project)?;
    login_with_secret(sh, &secret)?;

    let clusters = &secret.clusters;
    if clusters.is_empty() {
        return Err(eyre!("{project} has no clusters"));
    }

    for cluster in clusters {
        switch_to_single_cluster(sh, cluster)?;
    }

    let cluster = clusters
        .last()
        .ok_or_else(|| eyre!("{project} has no clusters"))?;
    let cluster_name = &cluster.name;

    cmd!(sh, "gcloud config set container/cluster {cluster_name}").run()?;

    Ok(())
}

pub fn switch_cluster(sh: &Shell, project: &str, cluster_name: &str) -> Result<()> {
    let secret = gcloud_secret(project)?;
    login_with_secret(sh, &secret)?;

    let cluster = find_cluster(&secret.clusters, cluster_name)
        .wrap_err_with(|| format!("could not pick a cluster in {project}"))?;

    switch_to_single_cluster(sh, cluster)?;

    let cluster_name = &cluster.name;

    cmd!(sh, "gcloud config set container/cluster {cluster_name}").run()?;

    Ok(())
}

/// Picks an exact name match, else the only cluster whose name contains `cluster_name`
fn find_cluster<'a>(
    clusters: &'a [GcloudCluster],
    cluster_name: &str,
) -> Result<&'a GcloudCluster> {
    if let Some(cluster) = clusters.iter().find(|cluster| cluster.name == cluster_name) {
        return Ok(cluster);
    }

    let matches = clusters
        .iter()
        .filter(|cluster| cluster.name.contains(cluster_name))
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [cluster] => Ok(cluster),
        [] => {
            let names = clusters.iter().map(|cluster| cluster.name.as_str());
            Err(eyre!(
                "no cluster matches {cluster_name}, clusters: {}",
                names.collect::<Vec<_>>().join(", ")
            ))
        }
        _ => {
            let names = matches.iter().map(|cluster| cluster.name.as_str());
            Err(eyre!(
                "{cluster_name} matches more than one cluster: {}",
                names.collect::<Vec<_>>().join(", ")
            ))
        }
    }
}

fn switch_to_single_cluster(sh: &Shell, cluster: &GcloudCluster) -> Result<(), eyre::Error> {
    let name = &cluster.name;
    let region = &cluster.region;
    let project = &cluster.project;

    cmd!(sh, "gcloud config set project {project}").run()?;
    cmd!(
        sh,
        "gcloud container clusters get-credentials {name} --region {region} --project {project}"
    )
    .run()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{find_cluster, GcloudCluster};

    fn cluster(name: &str) -> GcloudCluster {
        GcloudCluster {
            name: name.to_string(),
            region: "us-central1".to_string(),
            project: "project".to_string(),
        }
    }

    #[test]
    fn find_cluster_prefers_exact_then_unique_substring_and_rejects_ambiguity() {
        let clusters = [cluster("prod"), cluster("prod-eu"), cluster("staging")];

        assert_eq!(find_cluster(&clusters, "prod").unwrap().name, "prod");
        assert_eq!(find_cluster(&clusters, "stag").unwrap().name, "staging");

        let ambiguous = find_cluster(&clusters, "pro").unwrap_err().to_string();
        assert!(ambiguous.contains("prod, prod-eu"), "{ambiguous}");

        assert!(find_cluster(&clusters, "dev").is_err());
    }
}
