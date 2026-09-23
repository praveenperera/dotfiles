use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::path::PathBuf;
use std::str::FromStr;

macro_rules! tmux_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub(super) struct $name(String);

        impl FromStr for $name {
            type Err = eyre::Report;

            fn from_str(value: &str) -> Result<Self> {
                let suffix = value
                    .strip_prefix($prefix)
                    .ok_or_else(|| eyre!("invalid {}: {value}", stringify!($name)))?;
                if suffix.is_empty() || !suffix.chars().all(|character| character.is_ascii_digit())
                {
                    return Err(eyre!("invalid {}: {value}", stringify!($name)));
                }

                Ok(Self(value.to_owned()))
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

tmux_id!(SessionId, "$");
tmux_id!(WindowId, "@");
tmux_id!(PaneId, "%");

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct ClientId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub(super) enum MachineId {
    Mini,
    Code,
}

impl MachineId {
    /// Machines that host remote tmux sessions for the Mini workspace
    pub(super) const REMOTE: [Self; 1] = [Self::Code];

    pub(super) fn ssh_alias(self) -> Option<&'static str> {
        match self {
            Self::Mini => Some("praveen@Praveens-Mac-mini.local"),
            Self::Code => Some("code"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct ServerIdentity {
    pub(super) machine: MachineId,
    pub(super) socket: PathBuf,
    pub(super) generation: u32,
}

impl ServerIdentity {
    pub(super) fn new(machine: MachineId, socket: PathBuf, generation: u32) -> Result<Self> {
        if !socket.is_absolute() {
            return Err(eyre!("tmux socket identity must be an absolute path"));
        }
        if generation == 0 {
            return Err(eyre!("tmux server generation must not be zero"));
        }

        Ok(Self {
            machine,
            socket,
            generation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct PaneRef {
    pub(super) server: ServerIdentity,
    pub(super) pane: PaneId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct WindowRef {
    pub(super) server: ServerIdentity,
    pub(super) window: WindowId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct SessionRef {
    pub(super) server: ServerIdentity,
    pub(super) session: SessionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct ClientRef {
    pub(super) server: ServerIdentity,
    pub(super) client: ClientId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum Route {
    Local {
        pane: PaneRef,
    },
    Ssh {
        destination: SshDestination,
        pane: PaneRef,
    },
    ManagedHop {
        connection: ConnectionId,
        next: Box<Route>,
    },
}

impl Route {
    const MAX_HOPS: usize = 3;

    pub(super) fn validate(&self) -> Result<()> {
        let mut seen = Vec::new();
        self.validate_at(0, &mut seen)
    }

    fn validate_at(&self, depth: usize, seen: &mut Vec<ConnectionId>) -> Result<()> {
        if depth > Self::MAX_HOPS {
            return Err(eyre!("managed tmux route exceeds {} hops", Self::MAX_HOPS));
        }
        let Self::ManagedHop { connection, next } = self else {
            return Ok(());
        };
        if seen.contains(connection) {
            return Err(eyre!("managed tmux route contains a cycle"));
        }

        seen.push(connection.clone());
        next.validate_at(depth + 1, seen)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct ConnectionId(String);

impl ConnectionId {
    pub(super) fn new(value: String) -> Result<Self> {
        if value.is_empty()
            || value.len() > 128
            || !value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_-".contains(character))
        {
            return Err(eyre!("invalid connection id"));
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SshDestination {
    pub(super) machine: MachineId,
    pub(super) alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedPasteTarget {
    pub(super) source_tty: PathBuf,
    pub(super) source_pane: Option<PaneRef>,
    pub(super) route: Route,
    pub(super) machine: MachineId,
    pub(super) server: ServerIdentity,
    pub(super) pane: PaneId,
    pub(super) cwd: PathBuf,
    pub(super) ssh: Option<SshDestination>,
    pub(super) connections: Vec<ConnectionId>,
    pub(super) display_session: String,
    pub(super) display_window: String,
}

impl SshDestination {
    pub(super) fn approved(machine: MachineId) -> Result<Self> {
        let alias = machine
            .ssh_alias()
            .ok_or_else(|| eyre!("Mini is not an SSH execution target"))?;
        Ok(Self {
            machine,
            alias: alias.to_owned(),
        })
    }
}

pub(super) fn validate_name(label: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.starts_with('-') || value.contains([':', '\n', '\r', '\0']) {
        return Err(eyre!("invalid {label}: {value:?}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> ServerIdentity {
        ServerIdentity::new(MachineId::Code, PathBuf::from("/tmp/tmux-1000/default"), 42).unwrap()
    }

    #[test]
    fn rejects_invalid_stable_ids() {
        assert!("%12".parse::<PaneId>().is_ok());
        assert!("12".parse::<PaneId>().is_err());
        assert!("%old".parse::<PaneId>().is_err());
    }

    #[test]
    fn rejects_relative_socket_and_zero_generation() {
        assert!(ServerIdentity::new(MachineId::Code, PathBuf::from("default"), 1).is_err());
        assert!(ServerIdentity::new(MachineId::Code, PathBuf::from("/tmp/socket"), 0).is_err());
    }

    #[test]
    fn rejects_route_cycles() {
        let connection = ConnectionId("route-1".to_owned());
        let pane = PaneRef {
            server: server(),
            pane: "%1".parse().unwrap(),
        };
        let route = Route::ManagedHop {
            connection: connection.clone(),
            next: Box::new(Route::ManagedHop {
                connection,
                next: Box::new(Route::Local { pane }),
            }),
        };
        assert!(route.validate().is_err());
    }

    #[test]
    fn permits_approved_fleet_destinations_only() {
        assert_eq!(
            SshDestination::approved(MachineId::Code).unwrap().alias,
            "code"
        );
        assert_eq!(
            SshDestination::approved(MachineId::Mini).unwrap().alias,
            "praveen@Praveens-Mac-mini.local"
        );
    }
}
