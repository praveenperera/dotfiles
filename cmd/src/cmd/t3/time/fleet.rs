//! Which machines to read and how to reach them. Each one runs its own
//! `cmd t3 time-collect` over ssh, since T3's database can be several GB and
//! not every machine has the sqlite3 CLI

use std::path::PathBuf;
use std::process::{Command, Stdio};

use eyre::{bail, eyre, Result, WrapErr};
use serde::Deserialize;
use serde_json::Value;

use super::model::{Collection, Window, SCHEMA_VERSION};
use crate::fsutil;

/// Where the optional machine list lives
pub fn config_path() -> Result<PathBuf> {
    Ok(fsutil::home_dir()?.join(".config/cmd/thread-time.toml"))
}

/// The machine `code` runs ClickHouse, so by default its collector reads it
const DEFAULT_CLICKHOUSE_MACHINE: &str = "code";

/// `~/.config/cmd/thread-time.toml`, which overrides Homebased discovery
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Machines to read instead of the Homebased Fleet
    machines: Option<Vec<MachineConfig>>,
    /// Discovered machines to leave out
    #[serde(default)]
    skip: Vec<String>,
    /// Machine whose collector reads ClickHouse [default: code]
    clickhouse: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MachineConfig {
    name: String,
    /// ssh destination [default: the name]
    ssh: Option<String>,
    /// Path to cmd on that machine [default: cmd on PATH, with ~/.local/bin first]
    cmd: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Ok(Self::default());
        };

        toml::from_str(&text).wrap_err_with(|| format!("parse {}", path.display()))
    }

    pub fn clickhouse_machine(&self) -> &str {
        self.clickhouse
            .as_deref()
            .unwrap_or(DEFAULT_CLICKHOUSE_MACHINE)
    }
}

/// A machine to read, and how
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    pub name: String,
    pub reach: Reach,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reach {
    /// The machine this runs on, read in-process
    Local,
    Ssh(Remote),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    pub destination: String,
    /// cmd on the remote machine, when not the one on its PATH
    pub cmd: Option<String>,
}

/// The machines to read: the config's list, or every Homebased Fleet machine
/// that runs T3 or an agent CLI
pub fn machines(config: &Config) -> Result<Vec<Machine>> {
    let fleet = Fleet::discover();
    let local = match &fleet {
        Ok(fleet) => fleet.local.clone(),
        Err(_) => short_hostname()?,
    };

    let named: Vec<MachineConfig> = match &config.machines {
        Some(machines) => machines.clone(),
        None => {
            let fleet = fleet.wrap_err_with(|| {
                format!(
                    "list machines with `homebased fleet machines`; or list them in {}",
                    config_path()
                        .map_or_else(|_| "the config".to_owned(), |p| p.display().to_string())
                )
            })?;

            std::iter::once(fleet.local.clone())
                .chain(fleet.remote)
                .map(|name| MachineConfig {
                    name,
                    ssh: None,
                    cmd: None,
                })
                .collect()
        }
    };

    let mut machines: Vec<Machine> = Vec::new();
    for MachineConfig { name, ssh, cmd } in named {
        if config.skip.contains(&name) || machines.iter().any(|m| m.name == name) {
            continue;
        }

        let reach = match name == local {
            true => Reach::Local,
            false => Reach::Ssh(Remote {
                destination: ssh.unwrap_or_else(|| name.clone()),
                cmd,
            }),
        };

        machines.push(Machine { name, reach });
    }

    // this machine is always read, even when the list leaves it out
    if !machines.iter().any(|machine| machine.reach == Reach::Local) {
        machines.insert(
            0,
            Machine {
                name: local,
                reach: Reach::Local,
            },
        );
    }

    Ok(machines)
}

/// This machine's name and the agent machines Homebased knows about
struct Fleet {
    local: String,
    remote: Vec<String>,
}

impl Fleet {
    fn discover() -> Result<Self> {
        let output = Command::new("homebased")
            .args(["--json", "fleet", "machines"])
            .stderr(Stdio::null())
            .output()
            .wrap_err("run homebased")?;

        if !output.status.success() {
            bail!("homebased fleet machines exited with {}", output.status);
        }

        let body: Value =
            serde_json::from_slice(&output.stdout).wrap_err("parse homebased fleet machines")?;

        let local = body
            .pointer("/local/name")
            .and_then(Value::as_str)
            .ok_or_else(|| eyre!("homebased fleet machines names no local machine"))?
            .to_owned();

        let remote = body
            .get("machines")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|machine| runs_agents(machine))
            .filter_map(|machine| machine.get("name").and_then(Value::as_str))
            .map(str::to_owned)
            .collect();

        Ok(Self { local, remote })
    }
}

/// A machine with a T3 environment or an agent CLI can hold agent threads
fn runs_agents(machine: &Value) -> bool {
    let has_t3 = machine
        .get("t3_environment")
        .is_some_and(|env| !env.is_null());
    let has_agents = machine
        .pointer("/capabilities/agents")
        .and_then(Value::as_array)
        .is_some_and(|agents| !agents.is_empty());

    has_t3 || has_agents
}

fn short_hostname() -> Result<String> {
    let output = Command::new("hostname")
        .arg("-s")
        .output()
        .wrap_err("run hostname")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Run `cmd t3 time-collect` on a remote machine and read its JSON
pub fn collect_remote(remote: &Remote, window: &Window, clickhouse: bool) -> Result<Collection> {
    // a non-interactive ssh shell has no ~/.local/bin on PATH, where
    // `cmd release` installs cmd on every machine
    let program = remote
        .cmd
        .as_deref()
        .unwrap_or("PATH=\"$HOME/.local/bin:$PATH\" cmd");
    let mut command = format!(
        "{program} t3 time-collect --since {} --until {}",
        window.since.to_rfc3339(),
        window.until.to_rfc3339(),
    );

    if clickhouse {
        command.push_str(" --clickhouse");
    }

    // the timeouts keep an offline or hung machine from stalling the report
    let output = Command::new("ssh")
        .args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=10"])
        .args([
            "-o",
            "ServerAliveInterval=15",
            "-o",
            "ServerAliveCountMax=4",
        ])
        .arg(&remote.destination)
        .arg(command)
        .stdin(Stdio::null())
        .output()
        .wrap_err("run ssh")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        bail!("{}", remote_failure(output.status.code(), &stderr));
    }

    let collection: Collection = serde_json::from_slice(&output.stdout)
        .wrap_err("parse its collector output; its cmd may be out of date")?;

    if collection.schema != SCHEMA_VERSION {
        bail!(
            "its cmd sends schema {} but this one reads {SCHEMA_VERSION}; run `cmd fleet update`",
            collection.schema
        );
    }

    Ok(collection)
}

fn remote_failure(code: Option<i32>, stderr: &str) -> String {
    let last = stderr
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or("no error output");

    if stderr.contains("unrecognized subcommand") {
        return "its cmd has no `t3 time-collect`; run `cmd fleet update`".to_owned();
    }

    // ssh itself exits 255 when it cannot connect
    match code {
        Some(255) => format!("unreachable: {last}"),
        _ => last.to_owned(),
    }
}
