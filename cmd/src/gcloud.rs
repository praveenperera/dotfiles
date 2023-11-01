use eyre::Result;
use eyre::WrapErr;

use xshell::cmd;
use xshell::Shell;

use crate::SECRETS_DIR;

type Cluster = (&'static str, Vec<GcloudCluster>);

#[derive(serde::Deserialize)]
struct GcloudCluster {
    name: String,
    region: String,
    project: String,
}

fn clusters(projects: &[&'static str]) -> Result<Vec<Cluster>> {
    let mut clusters = Vec::new();

    for project in projects {
        let file = format!("{project}.yaml");

        let file_yaml = SECRETS_DIR
            .get_file(&file)
            .unwrap()
            .contents_utf8()
            .unwrap();

        let file_secrets: serde_yaml::Value =
            serde_yaml::from_str(file_yaml).wrap_err_with(|| format!("failed to parse {file}"))?;

        let file_clusters = file_secrets["clusters"].clone();
        let file_clusters: Vec<GcloudCluster> = serde_yaml::from_value(file_clusters).unwrap();

        clusters.push((*project, file_clusters));
    }

    Ok(clusters)
}

pub fn switch(sh: &Shell, args: &[&str]) -> Result<()> {
    let clusters = clusters(&["ln", "sq"])?;

    let project = args
        .first()
        .ok_or_else(|| eyre::eyre!("project not specified"))?;

    let clusters = clusters
        .iter()
        .find(|(p, _)| p == project)
        .map(|(_, c)| c)
        .ok_or_else(|| eyre::eyre!("{project} not found in clusters"))?;

    cmd!(sh, "gcloud auth login").run()?;

    for cluster in clusters {
        let name = &cluster.name;
        let region = &cluster.region;
        let project = &cluster.project;

        cmd!(sh, "gcloud config set project {project}").run()?;
        cmd!(sh, "gcloud container clusters get-credentials {name} --region {region} --project {project}").run()?;
    }

    Ok(())
}
