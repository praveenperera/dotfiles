use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;

const DESIRED_STATE_PATH: &str = "codex/plugins.toml";
const MARKETPLACE_MANIFEST_PATH: &str = ".agents/plugins/marketplace.json";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DesiredState {
    version: u32,
    plugins: Vec<DesiredPlugin>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DesiredPlugin {
    selector: PluginSelector,
    marketplace_source: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
struct PluginSelector(String);

impl PluginSelector {
    fn marketplace_name(&self) -> &str {
        self.0.split_once('@').expect("validated plugin selector").1
    }
}

impl TryFrom<String> for PluginSelector {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let Some((plugin, marketplace)) = value.split_once('@') else {
            return Err(format!(
                "plugin selector must be `plugin@marketplace`: {value}"
            ));
        };
        if plugin.is_empty() || marketplace.is_empty() || marketplace.contains('@') {
            return Err(format!(
                "plugin selector must be `plugin@marketplace`: {value}"
            ));
        }

        Ok(Self(value))
    }
}

#[derive(Debug, Deserialize)]
struct MarketplaceManifest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MarketplaceList {
    marketplaces: Vec<MarketplaceListEntry>,
}

#[derive(Debug, Deserialize)]
struct MarketplaceListEntry {
    name: String,
}

#[derive(Debug, Deserialize)]
struct PluginList {
    installed: Vec<PluginListEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginListEntry {
    plugin_id: String,
}

struct Reconciler {
    dotfiles_dir: PathBuf,
    desired_state_path: PathBuf,
    codex: OsString,
    home: PathBuf,
    codex_home: PathBuf,
    command_env: Vec<(OsString, OsString)>,
}

struct ResolvedPlugin<'a> {
    desired: &'a DesiredPlugin,
    marketplace_source: PathBuf,
}

pub(super) fn reconcile() -> Result<()> {
    let home = crate::fsutil::home_dir()?;
    let dotfiles_dir = crate::dotfiles_dir()?;
    let codex_home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"));
    Reconciler {
        desired_state_path: dotfiles_dir.join(DESIRED_STATE_PATH),
        dotfiles_dir,
        codex: OsString::from("codex"),
        home,
        codex_home,
        command_env: Vec::new(),
    }
    .reconcile()
}

impl Reconciler {
    fn reconcile(&self) -> Result<()> {
        if !self.desired_state_path.exists() {
            return Ok(());
        }

        let desired = self.read_desired_state()?;
        let resolved = desired
            .plugins
            .iter()
            .map(|plugin| self.resolve_plugin(plugin))
            .collect::<Result<Vec<_>>>()?;

        let installed_marketplaces: MarketplaceList =
            self.read_json(&["plugin", "marketplace", "list", "--json"])?;
        let mut installed_marketplaces = installed_marketplaces
            .marketplaces
            .into_iter()
            .map(|marketplace| marketplace.name)
            .collect::<HashSet<_>>();
        for plugin in &resolved {
            let marketplace_name = plugin.desired.selector.marketplace_name();
            if installed_marketplaces.contains(marketplace_name) {
                continue;
            }

            println!("adding Codex plugin marketplace {}", marketplace_name);
            self.run_mutation(&[
                OsStr::new("plugin"),
                OsStr::new("marketplace"),
                OsStr::new("add"),
                plugin.marketplace_source.as_os_str(),
            ])?;
            installed_marketplaces.insert(marketplace_name.to_owned());
        }

        let installed_plugins: PluginList = self.read_json(&["plugin", "list", "--json"])?;
        let mut installed_plugins = installed_plugins
            .installed
            .into_iter()
            .map(|plugin| plugin.plugin_id)
            .collect::<HashSet<_>>();
        for plugin in resolved {
            if installed_plugins.contains(&plugin.desired.selector.0) {
                continue;
            }

            println!("adding Codex plugin {}", plugin.desired.selector.0);
            self.run_mutation(&[
                OsStr::new("plugin"),
                OsStr::new("add"),
                OsStr::new(&plugin.desired.selector.0),
            ])?;
            installed_plugins.insert(plugin.desired.selector.0.clone());
        }

        Ok(())
    }

    fn read_desired_state(&self) -> Result<DesiredState> {
        let contents = fs::read_to_string(&self.desired_state_path).wrap_err_with(|| {
            format!(
                "failed to read Codex plugin desired state {}",
                self.desired_state_path.display()
            )
        })?;
        let desired: DesiredState = toml::from_str(&contents).wrap_err_with(|| {
            format!(
                "failed to parse Codex plugin desired state {}",
                self.desired_state_path.display()
            )
        })?;
        if desired.version != 1 {
            return Err(eyre!(
                "unsupported Codex plugin desired-state version {} in {}",
                desired.version,
                self.desired_state_path.display()
            ));
        }

        Ok(desired)
    }

    fn resolve_plugin<'a>(&self, desired: &'a DesiredPlugin) -> Result<ResolvedPlugin<'a>> {
        let source = if desired.marketplace_source.is_absolute() {
            desired.marketplace_source.clone()
        } else {
            self.dotfiles_dir.join(&desired.marketplace_source)
        };
        let source = fs::canonicalize(&source).wrap_err_with(|| {
            format!(
                "Codex marketplace source {} for {} is missing; clone the product repository next to the dotfiles repository",
                source.display(),
                desired.selector.0
            )
        })?;
        if !source.is_dir() {
            return Err(eyre!(
                "Codex marketplace source for {} is not a directory: {}",
                desired.selector.0,
                source.display()
            ));
        }

        let manifest_path = source.join(MARKETPLACE_MANIFEST_PATH);
        let manifest_contents = fs::read(&manifest_path).wrap_err_with(|| {
            format!(
                "Codex marketplace source {} for {} is missing {}; build or restore the product marketplace",
                source.display(),
                desired.selector.0,
                MARKETPLACE_MANIFEST_PATH
            )
        })?;
        let manifest: MarketplaceManifest = serde_json::from_slice(&manifest_contents)
            .wrap_err_with(|| {
                format!(
                    "invalid Codex marketplace manifest {}",
                    manifest_path.display()
                )
            })?;
        if manifest.name != desired.selector.marketplace_name() {
            return Err(eyre!(
                "Codex marketplace {} does not match selector {}",
                manifest.name,
                desired.selector.0
            ));
        }

        Ok(ResolvedPlugin {
            desired,
            marketplace_source: source,
        })
    }

    fn read_json<T: for<'de> Deserialize<'de>>(&self, args: &[&str]) -> Result<T> {
        let output = self.run(args.iter().map(OsStr::new))?;
        serde_json::from_slice(&output.stdout).wrap_err_with(|| {
            format!(
                "failed to parse JSON from `{}`",
                self.display_command(args.iter().map(OsStr::new))
            )
        })
    }

    fn run_mutation(&self, args: &[&OsStr]) -> Result<()> {
        self.run(args.iter().copied())?;
        Ok(())
    }

    fn run<'a>(&self, args: impl IntoIterator<Item = &'a OsStr> + Clone) -> Result<Output> {
        let display = self.display_command(args.clone());
        let mut command = Command::new(&self.codex);
        command
            .args(args)
            .env("HOME", &self.home)
            .env("CODEX_HOME", &self.codex_home)
            .envs(self.command_env.iter().cloned());
        let output = command.output().wrap_err_with(|| {
            format!("failed to run `{display}`; install Codex CLI and ensure it is on PATH")
        })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(eyre!(
                "`{display}` failed with status {}: {stderr}",
                output.status
            ));
        }

        Ok(output)
    }

    fn display_command<'a>(&self, args: impl IntoIterator<Item = &'a OsStr>) -> String {
        std::iter::once(self.codex.to_string_lossy().into_owned())
            .chain(
                args.into_iter()
                    .map(|arg| arg.to_string_lossy().into_owned()),
            )
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    use tempfile::{tempdir, TempDir};

    use super::Reconciler;

    struct Fixture {
        _root: TempDir,
        reconciler: Reconciler,
        calls: PathBuf,
    }

    impl Fixture {
        fn new(marketplaces: &str, plugins: &str) -> Self {
            let root = tempdir().unwrap();
            let home = root.path().join("home");
            let codex_home = root.path().join("codex-home");
            let dotfiles = root.path().join("code/dotfiles");
            let product = root.path().join("code/codex-supervisor");
            let desired_state_path = dotfiles.join("codex/plugins.toml");
            let marketplace = product.join(".agents/plugins/marketplace.json");
            let codex = root.path().join("bin/codex");
            let calls = root.path().join("calls");

            fs::create_dir_all(&home).unwrap();
            fs::create_dir_all(&codex_home).unwrap();
            fs::create_dir_all(desired_state_path.parent().unwrap()).unwrap();
            fs::create_dir_all(marketplace.parent().unwrap()).unwrap();
            fs::create_dir_all(codex.parent().unwrap()).unwrap();
            fs::write(
                &desired_state_path,
                r#"version = 1

[[plugins]]
selector = "codex-supervisor@codex-supervisor-local"
marketplace_source = "../codex-supervisor"
"#,
            )
            .unwrap();
            fs::write(
                &marketplace,
                r#"{"name":"codex-supervisor-local","plugins":[]}"#,
            )
            .unwrap();
            fs::write(
                &codex,
                r#"#!/bin/sh
printf '%s\n' "$*" >> "$CALLS"
case "$*" in
  "plugin marketplace list --json") printf '%s\n' "$MARKETPLACES_JSON" ;;
  "plugin list --json") printf '%s\n' "$PLUGINS_JSON" ;;
esac
"#,
            )
            .unwrap();
            let mut permissions = fs::metadata(&codex).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&codex, permissions).unwrap();

            let reconciler = Reconciler {
                dotfiles_dir: dotfiles,
                desired_state_path,
                codex: codex.into_os_string(),
                home,
                codex_home,
                command_env: vec![
                    ("CALLS".into(), calls.clone().into_os_string()),
                    ("MARKETPLACES_JSON".into(), marketplaces.into()),
                    ("PLUGINS_JSON".into(), plugins.into()),
                ],
            };

            Self {
                _root: root,
                reconciler,
                calls,
            }
        }

        fn calls(&self) -> Vec<String> {
            fs::read_to_string(&self.calls)
                .unwrap_or_default()
                .lines()
                .map(str::to_owned)
                .collect()
        }
    }

    #[test]
    fn adds_absent_marketplace_and_plugin() {
        let fixture = Fixture::new(r#"{"marketplaces":[]}"#, r#"{"installed":[]}"#);

        fixture.reconciler.reconcile().unwrap();

        let calls = fixture.calls();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0], "plugin marketplace list --json");
        assert!(calls[1].starts_with("plugin marketplace add /"));
        assert!(calls[1].ends_with("/codex-supervisor"));
        assert_eq!(calls[2], "plugin list --json");
        assert_eq!(
            calls[3],
            "plugin add codex-supervisor@codex-supervisor-local"
        );
        assert!(calls.iter().all(|call| !call.contains("mcp")));
    }

    #[test]
    fn leaves_present_marketplace_and_plugin_unchanged() {
        let fixture = Fixture::new(
            r#"{"marketplaces":[{"name":"codex-supervisor-local"}]}"#,
            r#"{"installed":[{"pluginId":"codex-supervisor@codex-supervisor-local"}]}"#,
        );

        fixture.reconciler.reconcile().unwrap();

        assert_eq!(
            fixture.calls(),
            vec!["plugin marketplace list --json", "plugin list --json"]
        );
    }

    #[test]
    fn reports_missing_marketplace_source_without_running_codex() {
        let fixture = Fixture::new(r#"{"marketplaces":[]}"#, r#"{"installed":[]}"#);
        let source = fixture.reconciler.dotfiles_dir.join("../codex-supervisor");
        fs::remove_dir_all(source).unwrap();

        let error = fixture.reconciler.reconcile().unwrap_err().to_string();

        assert!(error.contains("marketplace source"));
        assert!(error.contains("clone the product repository next to the dotfiles repository"));
        assert!(fixture.calls().is_empty());
    }

    #[test]
    fn reports_missing_codex_with_actionable_error() {
        let mut fixture = Fixture::new(r#"{"marketplaces":[]}"#, r#"{"installed":[]}"#);
        fixture.reconciler.codex = fixture
            .reconciler
            .home
            .join("missing-codex")
            .into_os_string();

        let error = fixture.reconciler.reconcile().unwrap_err().to_string();

        assert!(error.contains("install Codex CLI and ensure it is on PATH"));
    }

    #[test]
    fn desired_state_does_not_link_codex_config_or_define_mcp_servers() {
        let fixture = Fixture::new(r#"{"marketplaces":[]}"#, r#"{"installed":[]}"#);
        let desired = fs::read_to_string(&fixture.reconciler.desired_state_path).unwrap();

        assert!(!desired.contains("config.toml"));
        assert!(!desired.contains("config-groups"));
        assert!(!desired.contains("mcp_servers"));
    }
}
