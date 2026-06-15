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
