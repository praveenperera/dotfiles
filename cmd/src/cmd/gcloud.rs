use eyre::Result;
use eyre::WrapErr;

use eyre::eyre;
use xshell::cmd;
use xshell::Shell;

use crate::SECRETS_DIR;

type Cluster = (&'static str, Vec<GcloudCluster>);

#[derive(serde::Deserialize)]
struct GcloudSecrets {
    account: String,
    clusters: Vec<GcloudCluster>,
}

#[derive(serde::Deserialize)]
struct GcloudCluster {
    name: String,
    region: String,
    project: String,
}

fn gcloud_secret(project: &str) -> Result<GcloudSecrets> {
    let file = format!("{project}.yaml");

    let file_yaml = SECRETS_DIR
        .get_file(&file)
        .ok_or_else(|| eyre::eyre!("failed to find {file}"))?
        .contents_utf8()
        .ok_or_else(|| eyre!("failed to read {file}"))?;

    let secret = serde_yaml::from_str::<GcloudSecrets>(file_yaml)
        .wrap_err_with(|| format!("failed to parse {file}"))?;

    Ok(secret)
}

fn clusters() -> Result<Vec<Cluster>> {
    let projects = &["sq"];

    let mut clusters = Vec::new();

    for project in projects {
        let gcloud_secret = gcloud_secret(project)?;
        clusters.push((*project, gcloud_secret.clusters));
    }

    Ok(clusters)
}

pub fn login(sh: &Shell, args: &[&str]) -> Result<()> {
    let project = args.first().ok_or_else(|| eyre!("project not specified"))?;
    let account = gcloud_secret(project)?.account;

    cmd!(sh, "gcloud config set account {account}").run()?;

    Ok(())
}

pub fn switch_project(sh: &Shell, args: &[&str]) -> Result<()> {
    login(sh, args)?;

    let project = args.first().ok_or_else(|| eyre!("project not specified"))?;

    let clusters = clusters()?;
    let clusters = clusters
        .iter()
        .find(|(p, _)| p == project)
        .map(|(_, c)| c)
        .ok_or_else(|| eyre!("{project} not found in clusters"))?;

    if clusters.is_empty() {
        return Err(eyre!("{project} has no clusters"));
    }

    for cluster in clusters {
        switch_to_single_cluster(sh, cluster)?;
    }

    let cluster = clusters.last().expect("already checked");
    let cluster_name = &cluster.name;

    cmd!(sh, "gcloud config set container/cluster {cluster_name}").run()?;

    Ok(())
}

pub fn switch_cluster(sh: &Shell, args: &[&str]) -> Result<()> {
    login(sh, args)?;

    let project = args.first().ok_or_else(|| eyre!("project not specified"))?;
    let cluster = args.get(1).ok_or_else(|| eyre!("cluster not specified"))?;

    let clusters = clusters()?;
    let clusters = clusters
        .iter()
        .find(|(p, _)| p == project)
        .map(|(_, c)| c)
        .ok_or_else(|| eyre!("{project} not found in clusters"))?;

    let cluster = clusters
        .iter()
        .find(|c| c.name.contains(cluster))
        .ok_or_else(|| eyre!("cluster {cluster} not found in {project}"))?;

    switch_to_single_cluster(sh, cluster)?;

    let cluster_name = &cluster.name;

    cmd!(sh, "gcloud config set container/cluster {cluster_name}").run()?;

    Ok(())
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
