/// Helpers for working with jj-lib directly instead of spawning jj CLI processes
///
/// Note: jj-lib is designed primarily for the jj CLI, so some operations
/// (especially git fetch/push) are easier to do via the CLI. This module
/// provides helpers for read operations and simple mutations.
use eyre::{bail, Context, Result};
use itertools::Itertools;
use jj_lib::backend::CommitId;
use jj_lib::commit::Commit;
use jj_lib::config::{ConfigLayer, ConfigSource, StackedConfig};
use jj_lib::id_prefix::IdPrefixContext;
use jj_lib::ref_name::{RefName, RefNameBuf, RemoteName, RemoteRefSymbol};
use jj_lib::repo::{ReadonlyRepo, Repo, StoreFactories};
use jj_lib::revset::{
    self, RevsetDiagnostics, RevsetIteratorExt, RevsetParseContext, RevsetWorkspaceContext,
    SymbolResolver,
};
use jj_lib::settings::UserSettings;
use jj_lib::workspace::{default_working_copy_factories, Workspace};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Creates a minimal UserSettings for jj operations
pub fn create_user_settings() -> Result<UserSettings> {
    let config_text = r#"
        user.name = "jj-lib user"
        user.email = "jj-lib@localhost"
        operation.username = "jj-lib"
        operation.hostname = "localhost"
    "#;
    let mut config = StackedConfig::with_defaults();
    config.add_layer(ConfigLayer::parse(ConfigSource::User, config_text)?);
    UserSettings::from_config(config).wrap_err("failed to create user settings")
}

/// Wrapper around jj workspace and repo for convenient operations
pub struct JjRepo {
    workspace: Workspace,
    repo: Arc<ReadonlyRepo>,
    #[allow(dead_code)]
    settings: UserSettings,
}

impl JjRepo {
    /// Load a jj workspace from the given path (or current directory)
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let workspace_path = match path {
            Some(p) => p.to_path_buf(),
            None => std::env::current_dir().wrap_err("failed to get current directory")?,
        };

        let settings = create_user_settings()?;
        let store_factories = StoreFactories::default();
        let working_copy_factories = default_working_copy_factories();

        let workspace = Workspace::load(
            &settings,
            &workspace_path,
            &store_factories,
            &working_copy_factories,
        )
        .wrap_err("failed to load workspace")?;

        let repo = workspace
            .repo_loader()
            .load_at_head()
            .wrap_err("failed to load repo at head")?;

        Ok(Self {
            workspace,
            repo,
            settings,
        })
    }

    #[allow(dead_code)]
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn repo(&self) -> &Arc<ReadonlyRepo> {
        &self.repo
    }

    /// Start a transaction for making changes
    pub fn start_transaction(&self) -> jj_lib::transaction::Transaction {
        self.repo.start_transaction()
    }

    /// Evaluate a revset string and return matching commits
    pub fn eval_revset(&self, revset_str: &str) -> Result<Vec<Commit>> {
        let mut diagnostics = RevsetDiagnostics::new();
        let aliases_map = self.aliases_map();
        let extensions = Arc::new(revset::RevsetExtensions::default());

        let workspace_ctx = RevsetWorkspaceContext {
            path_converter: &jj_lib::repo_path::RepoPathUiConverter::Fs {
                cwd: self.workspace.workspace_root().to_path_buf(),
                base: self.workspace.workspace_root().to_path_buf(),
            },
            workspace_name: self.workspace.workspace_name(),
        };

        let context = RevsetParseContext {
            aliases_map: &aliases_map,
            local_variables: HashMap::new(),
            user_email: "jj-lib@localhost",
            date_pattern_context: chrono::Utc::now().fixed_offset().into(),
            default_ignored_remote: Some(RemoteName::new("git")),
            workspace: Some(workspace_ctx),
            extensions: &extensions,
            use_glob_by_default: false,
        };

        let expression = revset::parse(&mut diagnostics, revset_str, &context)
            .wrap_err_with(|| format!("failed to parse revset: {revset_str}"))?;

        let id_prefix_context = IdPrefixContext::default();
        let symbol_resolver =
            SymbolResolver::new(self.repo.as_ref(), extensions.symbol_resolvers())
                .with_id_prefix_context(&id_prefix_context);

        let resolved = expression
            .resolve_user_expression(self.repo.as_ref(), &symbol_resolver)
            .wrap_err("failed to resolve revset expression")?;

        let evaluated = resolved
            .evaluate(self.repo.as_ref())
            .wrap_err("failed to evaluate revset")?;

        let commits: Vec<Commit> = evaluated
            .iter()
            .commits(self.repo.store())
            .try_collect()
            .wrap_err("failed to collect commits")?;

        Ok(commits)
    }

    /// Evaluate a revset and return a single commit (error if 0 or >1 results)
    pub fn eval_revset_single(&self, revset_str: &str) -> Result<Commit> {
        let commits = self.eval_revset(revset_str)?;
        match commits.len() {
            0 => bail!("revset '{}' matched no commits", revset_str),
            1 => Ok(commits.into_iter().next().unwrap()),
            n => bail!("revset '{}' matched {} commits, expected 1", revset_str, n),
        }
    }

    /// Get the working copy commit
    pub fn working_copy_commit(&self) -> Result<Commit> {
        self.eval_revset_single("@")
    }

    /// Get all local bookmarks as (name, commit_id) pairs
    pub fn local_bookmarks(&self) -> Vec<(&RefName, CommitId)> {
        self.repo
            .view()
            .local_bookmarks()
            .filter_map(|(name, target)| {
                target
                    .as_resolved()
                    .and_then(|opt| opt.as_ref().map(|id| (name, id.clone())))
            })
            .collect()
    }

    /// Get local bookmarks on a specific commit
    pub fn bookmarks_at(&self, commit: &Commit) -> Vec<String> {
        self.repo
            .view()
            .local_bookmarks()
            .filter_map(|(name, target)| {
                let resolved = target.as_resolved().and_then(|opt| opt.as_ref());
                if resolved == Some(commit.id()) {
                    Some(name.as_str().to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get local bookmarks on a specific commit with divergence status
    /// A bookmark is diverged if local differs from origin
    pub fn bookmarks_with_state(&self, commit: &Commit) -> Vec<(String, bool)> {
        let origin = RemoteName::new("origin");
        self.repo
            .view()
            .local_bookmarks()
            .filter_map(|(name, target)| {
                let resolved = target.as_resolved().and_then(|opt| opt.as_ref());
                if resolved == Some(commit.id()) {
                    let local_id = commit.id();

                    // check if origin remote bookmark exists and differs
                    let symbol = RemoteRefSymbol { name, remote: origin };
                    let is_diverged = self.repo
                        .view()
                        .get_remote_bookmark(symbol)
                        .target
                        .as_resolved()
                        .and_then(|opt| opt.as_ref())
                        .map(|remote_id| remote_id != local_id)
                        .unwrap_or(false);

                    Some((name.as_str().to_string(), is_diverged))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get tracked bookmarks that are deleted on remote (target is absent)
    pub fn deleted_remote_bookmarks(&self, remote: &str) -> Vec<String> {
        let remote_name = RemoteName::new(remote);
        self.repo
            .view()
            .all_remote_bookmarks()
            .filter_map(|(symbol, remote_ref)| {
                // A bookmark is "deleted" on remote if it's tracked but the target is absent
                if symbol.remote == remote_name && remote_ref.target.is_absent() {
                    Some(symbol.name.as_str().to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Set a local bookmark to point to a commit
    pub fn set_bookmark(&self, name: &str, commit_id: &CommitId) -> Result<Arc<ReadonlyRepo>> {
        let mut tx = self.start_transaction();
        let target = jj_lib::op_store::RefTarget::resolved(Some(commit_id.clone()));
        tx.repo_mut()
            .set_local_bookmark_target(&RefNameBuf::from(name.to_string()), target);
        let repo = tx
            .commit(format!("set bookmark {name}"))
            .wrap_err("failed to commit transaction")?;
        Ok(repo)
    }

    /// Delete a local bookmark
    pub fn delete_bookmark(&self, name: &str) -> Result<Arc<ReadonlyRepo>> {
        let mut tx = self.start_transaction();
        tx.repo_mut().set_local_bookmark_target(
            &RefNameBuf::from(name.to_string()),
            jj_lib::op_store::RefTarget::absent(),
        );
        let repo = tx
            .commit(format!("delete bookmark {name}"))
            .wrap_err("failed to commit transaction")?;
        Ok(repo)
    }

    /// Abandon a commit
    pub fn abandon(&self, commit: &Commit) -> Result<Arc<ReadonlyRepo>> {
        let mut tx = self.start_transaction();
        tx.repo_mut().record_abandoned_commit(commit);
        let repo = tx
            .commit("abandon commit")
            .wrap_err("failed to commit transaction")?;
        Ok(repo)
    }

    /// Check if a commit is empty (no changes from parent)
    pub fn is_commit_empty(&self, commit: &Commit) -> Result<bool> {
        let parent_tree = commit.parent_tree(self.repo.as_ref())?;
        let commit_tree = commit.tree();
        Ok(commit_tree.tree_ids() == parent_tree.tree_ids())
    }

    /// Get the shortest unique change_id prefix for a commit (minimum `min_len` chars)
    pub fn shortest_change_id(&self, commit: &Commit, min_len: usize) -> Result<String> {
        let (display, _) = self.change_id_with_prefix_len(commit, min_len)?;
        Ok(display)
    }

    /// Get change_id display string and the actual unique prefix length from the repository index
    ///
    /// Returns (display_string, unique_prefix_len) where display_string is at least `min_len` chars
    /// and unique_prefix_len is the minimum length needed to uniquely identify this commit
    pub fn change_id_with_prefix_len(
        &self,
        commit: &Commit,
        min_len: usize,
    ) -> Result<(String, usize)> {
        let extensions = Arc::new(revset::RevsetExtensions::default());

        // use the same disambiguation context as jj CLI (revsets.log default)
        // this is: present(@) | ancestors(immutable_heads().., 2) | trunk()
        let mut diagnostics = RevsetDiagnostics::new();
        let context = RevsetParseContext {
            aliases_map: &self.aliases_map(),
            local_variables: HashMap::new(),
            user_email: "",
            date_pattern_context: chrono::Utc::now().fixed_offset().into(),
            default_ignored_remote: Some(RemoteName::new("git")),
            workspace: Some(RevsetWorkspaceContext {
                path_converter: &jj_lib::repo_path::RepoPathUiConverter::Fs {
                    cwd: self.workspace.workspace_root().to_path_buf(),
                    base: self.workspace.workspace_root().to_path_buf(),
                },
                workspace_name: self.workspace.workspace_name(),
            }),
            extensions: &extensions,
            use_glob_by_default: false,
        };
        let short_prefixes_revset =
            "present(@) | ancestors(immutable_heads().., 2) | present(trunk())";
        let disambiguate_expr = revset::parse(&mut diagnostics, short_prefixes_revset, &context)
            .wrap_err("failed to parse short-prefixes revset")?;

        let id_prefix_context =
            IdPrefixContext::new(extensions.clone()).disambiguate_within(disambiguate_expr);
        let index = id_prefix_context
            .populate(self.repo.as_ref())
            .wrap_err("failed to populate id prefix index")?;
        let unique_prefix_len = index
            .shortest_change_prefix_len(self.repo.as_ref(), commit.change_id())
            .wrap_err("failed to get shortest prefix length")?;
        let full_id = commit.change_id().reverse_hex();
        let display_len = unique_prefix_len.max(min_len).min(full_id.len());
        Ok((full_id[..display_len].to_string(), unique_prefix_len))
    }

    /// Get the revset aliases map with jj's default aliases
    fn aliases_map(&self) -> revset::RevsetAliasesMap {
        let mut aliases_map = revset::RevsetAliasesMap::new();

        let default_aliases = [
            (
                "trunk()",
                r#"latest(
              remote_bookmarks(exact:"main", exact:"origin") |
              remote_bookmarks(exact:"master", exact:"origin") |
              remote_bookmarks(exact:"trunk", exact:"origin") |
              remote_bookmarks(exact:"main", exact:"upstream") |
              remote_bookmarks(exact:"master", exact:"upstream") |
              remote_bookmarks(exact:"trunk", exact:"upstream") |
              root()
            )"#,
            ),
            (
                "builtin_immutable_heads()",
                "trunk() | tags() | untracked_remote_bookmarks()",
            ),
            ("immutable_heads()", "builtin_immutable_heads()"),
            ("immutable()", "::(immutable_heads() | root())"),
            ("mutable()", "~immutable()"),
        ];

        for (name, def) in default_aliases {
            let _ = aliases_map.insert(name, def);
        }
        aliases_map
    }

    /// Get parent commits for a commit
    pub fn parent_commits(&self, commit: &Commit) -> Result<Vec<Commit>> {
        commit
            .parents()
            .try_collect()
            .wrap_err("failed to get parent commits")
    }

    /// Get the first line of a commit's description
    pub fn description_first_line(commit: &Commit) -> String {
        commit
            .description()
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    }

    /// Get the author name for a commit
    pub fn author_name(commit: &Commit) -> String {
        commit.author().name.clone()
    }

    /// Get the author email for a commit
    pub fn author_email(commit: &Commit) -> String {
        commit.author().email.clone()
    }

    /// Get the author timestamp formatted as a relative time string with absolute date
    pub fn author_timestamp_relative(commit: &Commit) -> String {
        let ts = commit.author().timestamp;
        let millis = ts.timestamp.0;
        let secs = millis / 1000;
        let datetime = chrono::DateTime::from_timestamp(secs, 0);
        match datetime {
            Some(dt) => {
                let now = chrono::Utc::now();
                let diff = now.signed_duration_since(dt);
                let absolute = dt.format("%Y-%m-%d %H:%M");
                let relative = if diff.num_days() > 365 {
                    format!("{} years ago", diff.num_days() / 365)
                } else if diff.num_days() > 30 {
                    format!("{} months ago", diff.num_days() / 30)
                } else if diff.num_days() > 0 {
                    format!("{} days ago", diff.num_days())
                } else if diff.num_hours() > 0 {
                    format!("{} hours ago", diff.num_hours())
                } else if diff.num_minutes() > 0 {
                    format!("{} minutes ago", diff.num_minutes())
                } else {
                    "just now".to_string()
                };
                format!("{relative} ({absolute})")
            }
            None => "unknown".to_string(),
        }
    }
}
