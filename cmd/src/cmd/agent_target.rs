use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum AgentTarget {
    #[default]
    Codex,
    Claude,
}

impl AgentTarget {
    /// All supported agent install targets, in stable order
    pub fn all() -> &'static [Self] {
        &[Self::Codex, Self::Claude]
    }

    pub fn project_skills_dir(self, git_root: &Path) -> PathBuf {
        match self {
            Self::Codex => git_root.join(".agents/skills"),
            Self::Claude => git_root.join(".claude/skills"),
        }
    }

    pub fn project_mcp_config_path(self, git_root: &Path) -> PathBuf {
        match self {
            Self::Codex => git_root.join(".codex/config.toml"),
            Self::Claude => git_root.join(".mcp.json"),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude",
        }
    }
}

/// Resolve CLI agent selection: an explicit target, or every supported agent
pub fn selected_agents(agent: Option<AgentTarget>) -> Vec<AgentTarget> {
    match agent {
        Some(agent) => vec![agent],
        None => AgentTarget::all().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::{selected_agents, AgentTarget};

    #[test]
    fn selected_agents_defaults_to_all() {
        assert_eq!(
            selected_agents(None),
            [AgentTarget::Codex, AgentTarget::Claude]
        );
    }

    #[test]
    fn selected_agents_respects_explicit_target() {
        assert_eq!(
            selected_agents(Some(AgentTarget::Claude)),
            [AgentTarget::Claude]
        );
    }
}
