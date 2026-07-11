use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Clone, Args)]
pub struct BetterContextArgs {
    /// Repository: owner/repo, URL, or local path
    pub repo: String,

    /// Force fresh clone
    #[arg(short, long)]
    pub fresh: bool,

    /// Checkout specific git ref (branch, tag, or SHA)
    #[arg(short, long)]
    pub r#ref: Option<String>,

    /// Clone complete history instead of single-branch
    #[arg(long)]
    pub full: bool,

    /// Suppress progress logs
    #[arg(short, long)]
    pub quiet: bool,
}

#[derive(Debug, Clone, Args)]
pub struct PrContextArgs {
    /// GitHub PR URL or repository in format "owner/repo"
    pub repo_or_url: String,

    /// Pull request number (optional if URL is provided)
    pub pr_number: Option<u64>,

    /// GitHub token (defaults to GITHUB_TOKEN, GH_TOKEN, or gh auth token)
    #[arg(short, long)]
    pub token: Option<String>,

    /// Only include comments with code references
    #[arg(short = 'c', long)]
    pub code_only: bool,

    /// Compact output (only author, body, and code_reference)
    #[arg(short = 'C', long)]
    pub compact: bool,

    /// Output format
    #[arg(short = 'f', long, default_value = "markdown")]
    pub format: crate::pr_context::OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct ReleaseArgs {
    /// Local project name (e.g., "jju")
    pub project: Option<String>,

    /// Install the given built cmd binary and refresh hardlinks
    #[arg(long, hide = true, value_name = "PATH")]
    pub install_built: Option<PathBuf>,

    /// Refresh hardlinks for the installed cmd binary
    #[arg(long, hide = true)]
    pub link_installed: bool,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "cmd",
    about = "Command line utilities",
    arg_required_else_help = true
)]
pub struct Cmd {
    /// Print version information
    #[arg(long, short = 'V')]
    pub version: bool,

    #[command(subcommand)]
    pub subcommand: MainCmd,
}

#[derive(Debug, Clone, Subcommand)]
#[command(subcommand_value_name = "COMMAND")]
pub enum MainCmd {
    /// Bootstrap dotfiles
    #[command(arg_required_else_help = true)]
    Bootstrap {
        mode: crate::cmd::bootstrap::BootstrapMode,
    },

    /// Release/update cmd binary, or release a local project
    Release(#[command(flatten)] ReleaseArgs),

    /// Configure dotfiles
    #[command(visible_alias = "cfg")]
    Config,

    /// Cloudflare API operations
    #[command(visible_alias = "cf", arg_required_else_help = true)]
    Cloudflare {
        #[command(subcommand)]
        subcommand: crate::cmd::cloudflare::CloudflareCmd,
    },

    /// Google Cloud operations
    #[command(arg_required_else_help = true)]
    Gcloud {
        #[command(subcommand)]
        subcommand: crate::cmd::gcloud::GcloudCmd,
    },

    /// Secret operations
    #[command(arg_required_else_help = true)]
    Secret {
        #[command(subcommand)]
        subcommand: crate::cmd::secrets::SecretsCmd,
    },

    /// Terraform operations
    #[command(visible_alias = "tf", arg_required_else_help = true)]
    Terraform {
        #[command(subcommand)]
        subcommand: crate::cmd::terraform::TerraformCmd,
    },

    /// Vault operations
    #[command(arg_required_else_help = true)]
    Vault {
        #[command(subcommand)]
        subcommand: crate::cmd::vault::VaultCmd,
    },

    /// Generate code/files
    #[command(visible_alias = "gen", arg_required_else_help = true)]
    Generate {
        #[command(subcommand)]
        subcommand: crate::cmd::generate::GenerateCmd,
    },

    /// Install a tool from a GitHub release
    #[command(visible_alias = "i")]
    Install(#[command(flatten)] crate::cmd::install::Install),

    /// Tmux operations
    #[command(arg_required_else_help = true)]
    Tmux {
        #[command(subcommand)]
        subcommand: crate::cmd::tmux::TmuxCmd,
    },

    /// Fetch PR comments and their code references from GitHub
    #[command(visible_alias = "prc")]
    PrContext(#[command(flatten)] PrContextArgs),

    /// Clone/update a repo for agent exploration
    #[command(visible_aliases = ["agent", "bc"])]
    BetterContext(#[command(flatten)] BetterContextArgs),

    /// Codex CLI profile management
    #[command(arg_required_else_help = true)]
    Codex {
        #[command(subcommand)]
        subcommand: crate::cmd::codex::CodexCmd,
    },

    /// Crate operations (crates.io)
    #[command(arg_required_else_help = true)]
    Crate {
        #[command(subcommand)]
        subcommand: CrateCmd,
    },

    /// File encryption/decryption operations
    #[command(arg_required_else_help = true)]
    File {
        #[command(subcommand)]
        subcommand: crate::cmd::file::FileCmd,
    },

    /// Manage reusable agent skills
    #[command(arg_required_else_help = true)]
    Skill {
        #[command(subcommand)]
        subcommand: crate::cmd::skill::SkillCmd,
    },

    /// Manage reusable project MCP servers
    #[command(arg_required_else_help = true)]
    Mcp {
        #[command(subcommand)]
        subcommand: crate::cmd::mcp::McpCmd,
    },

    /// Add reusable project packs of skills and MCPs
    #[command(arg_required_else_help = true)]
    Pack {
        #[command(subcommand)]
        subcommand: crate::cmd::pack::PackCmd,
    },

    /// Sync files and directories via iCloud
    #[command(arg_required_else_help = true)]
    Sync {
        #[command(subcommand)]
        subcommand: crate::cmd::sync::SyncCmd,
    },

    /// Symlink folders onto CacheDisk (dev-cache)
    #[command(arg_required_else_help = true)]
    Cache {
        #[command(subcommand)]
        subcommand: crate::cmd::cache::CacheCmd,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum CrateCmd {
    /// Fetch latest versions for crates from crates.io
    Versions(#[command(flatten)] crate::cmd::crate_versions::CrateVersions),
}

impl Cmd {
    pub fn from_args(args: &[std::ffi::OsString]) -> eyre::Result<Self> {
        use clap::Parser;
        let mut full_args = vec![std::ffi::OsString::from("cmd")];
        full_args.extend_from_slice(args);
        match Self::try_parse_from(full_args) {
            Ok(cmd) => Ok(cmd),
            Err(err) => {
                let _ = err.print();
                std::process::exit(err.exit_code());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{Cmd, MainCmd};
    use crate::cmd::agent_target::AgentTarget;
    use crate::cmd::cloudflare::{CloudflareCmd, RedirectCmd};
    use crate::cmd::mcp::McpCmd;
    use crate::cmd::pack::PackCmd;
    use crate::cmd::skill::SkillCmd;

    #[test]
    fn parses_skill_add_without_names() {
        let cmd = Cmd::from_args(&[OsString::from("skill"), OsString::from("add")]).unwrap();

        let MainCmd::Skill { subcommand } = cmd.subcommand else {
            panic!("expected skill command");
        };

        assert!(
            matches!(subcommand, SkillCmd::Add { agent: AgentTarget::Codex, skills } if skills.is_empty())
        );
    }

    #[test]
    fn parses_skill_add_with_names() {
        let cmd = Cmd::from_args(&[
            OsString::from("skill"),
            OsString::from("add"),
            OsString::from("alpha"),
            OsString::from("beta"),
        ])
        .unwrap();

        let MainCmd::Skill { subcommand } = cmd.subcommand else {
            panic!("expected skill command");
        };

        assert!(
            matches!(subcommand, SkillCmd::Add { agent: AgentTarget::Codex, skills } if skills == ["alpha", "beta"])
        );
    }

    #[test]
    fn parses_cloudflare_redirect_www_to_apex() {
        let cmd = Cmd::from_args(&[
            OsString::from("cloudflare"),
            OsString::from("redirect"),
            OsString::from("www-to-apex"),
            OsString::from("example.com"),
            OsString::from("--zone-id"),
            OsString::from("zone-id"),
        ])
        .unwrap();

        let MainCmd::Cloudflare { subcommand } = cmd.subcommand else {
            panic!("expected cloudflare command");
        };

        let CloudflareCmd::Redirect { subcommand } = subcommand;

        assert!(
            matches!(subcommand, RedirectCmd::WwwToApex(args) if args.zone == "example.com" && args.zone_id.as_deref() == Some("zone-id"))
        );
    }

    #[test]
    fn parses_cloudflare_redirect_list() {
        let cmd = Cmd::from_args(&[
            OsString::from("cf"),
            OsString::from("redirect"),
            OsString::from("list"),
            OsString::from("www.example.com"),
        ])
        .unwrap();

        let MainCmd::Cloudflare { subcommand } = cmd.subcommand else {
            panic!("expected cloudflare command");
        };

        let CloudflareCmd::Redirect { subcommand } = subcommand;

        assert!(matches!(subcommand, RedirectCmd::List(args) if args.zone == "www.example.com"));
    }

    #[test]
    fn parses_skill_add_for_claude() {
        let cmd = Cmd::from_args(&[
            OsString::from("skill"),
            OsString::from("add"),
            OsString::from("--agent"),
            OsString::from("claude"),
            OsString::from("alpha"),
        ])
        .unwrap();

        let MainCmd::Skill { subcommand } = cmd.subcommand else {
            panic!("expected skill command");
        };

        assert!(
            matches!(subcommand, SkillCmd::Add { agent: AgentTarget::Claude, skills } if skills == ["alpha"])
        );
    }

    #[test]
    fn parses_mcp_add_without_names() {
        let cmd = Cmd::from_args(&[OsString::from("mcp"), OsString::from("add")]).unwrap();

        let MainCmd::Mcp { subcommand } = cmd.subcommand else {
            panic!("expected mcp command");
        };

        assert!(matches!(subcommand, McpCmd::Add { mcps } if mcps.is_empty()));
    }

    #[test]
    fn parses_install_tool() {
        let cmd = Cmd::from_args(&[OsString::from("install"), OsString::from("smrze")]).unwrap();

        let MainCmd::Install(args) = cmd.subcommand else {
            panic!("expected install command");
        };

        assert_eq!(args.tool, "smrze");
        assert!(!args.force);
    }

    #[test]
    fn parses_install_alias() {
        let cmd = Cmd::from_args(&[OsString::from("i"), OsString::from("rustywind")]).unwrap();

        let MainCmd::Install(args) = cmd.subcommand else {
            panic!("expected install command");
        };

        assert_eq!(args.tool, "rustywind");
    }

    #[test]
    fn parses_mcp_add_with_names() {
        let cmd = Cmd::from_args(&[
            OsString::from("mcp"),
            OsString::from("add"),
            OsString::from("xcodebuildmcp"),
        ])
        .unwrap();

        let MainCmd::Mcp { subcommand } = cmd.subcommand else {
            panic!("expected mcp command");
        };

        assert!(matches!(subcommand, McpCmd::Add { mcps } if mcps == ["xcodebuildmcp"]));
    }

    #[test]
    fn parses_pack_add_without_names() {
        let cmd = Cmd::from_args(&[OsString::from("pack"), OsString::from("add")]).unwrap();

        let MainCmd::Pack { subcommand } = cmd.subcommand else {
            panic!("expected pack command");
        };

        assert!(
            matches!(subcommand, PackCmd::Add { agent: AgentTarget::Codex, packs } if packs.is_empty())
        );
    }

    #[test]
    fn parses_pack_add_with_names() {
        let cmd = Cmd::from_args(&[
            OsString::from("pack"),
            OsString::from("add"),
            OsString::from("web"),
            OsString::from("native"),
        ])
        .unwrap();

        let MainCmd::Pack { subcommand } = cmd.subcommand else {
            panic!("expected pack command");
        };

        assert!(
            matches!(subcommand, PackCmd::Add { agent: AgentTarget::Codex, packs } if packs == ["web", "native"])
        );
    }

    #[test]
    fn parses_pack_add_for_claude() {
        let cmd = Cmd::from_args(&[
            OsString::from("pack"),
            OsString::from("add"),
            OsString::from("--agent"),
            OsString::from("claude"),
            OsString::from("web"),
        ])
        .unwrap();

        let MainCmd::Pack { subcommand } = cmd.subcommand else {
            panic!("expected pack command");
        };

        assert!(
            matches!(subcommand, PackCmd::Add { agent: AgentTarget::Claude, packs } if packs == ["web"])
        );
    }

    #[test]
    fn parses_pack_refresh_all() {
        let cmd = Cmd::from_args(&[
            OsString::from("pack"),
            OsString::from("refresh"),
            OsString::from("--all"),
        ])
        .unwrap();

        let MainCmd::Pack { subcommand } = cmd.subcommand else {
            panic!("expected pack command");
        };

        assert!(matches!(subcommand, PackCmd::Refresh { all: true }));
    }

    #[test]
    fn parses_pack_refresh_current() {
        let cmd = Cmd::from_args(&[OsString::from("pack"), OsString::from("refresh")]).unwrap();

        let MainCmd::Pack { subcommand } = cmd.subcommand else {
            panic!("expected pack command");
        };

        assert!(matches!(subcommand, PackCmd::Refresh { all: false }));
    }
}
