/// Helpers for working with jj-lib directly instead of spawning jj CLI processes
///
/// Note: jj-lib is designed primarily for the jj CLI, so some operations
/// (especially git fetch/push) are easier to do via the CLI. This module
/// provides helpers for read operations and simple mutations.
use eyre::{Context, Result, bail};
use itertools::Itertools;
use jj_lib::backend::CommitId;
use jj_lib::commit::Commit;
use jj_lib::config::{ConfigLayer, ConfigSource, StackedConfig};
use jj_lib::id_prefix::IdPrefixContext;
use jj_lib::ref_name::{RefName, RefNameBuf, RemoteName};
use jj_lib::repo::{ReadonlyRepo, Repo, StoreFactories};
use jj_lib::revset::{
    self, RevsetDiagnostics, RevsetIteratorExt, RevsetParseContext, RevsetWorkspaceContext,
    SymbolResolver,
};
use jj_lib::settings::UserSettings;
use jj_lib::workspace::{Workspace, default_working_copy_factories};
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
        let mut aliases_map = revset::RevsetAliasesMap::new();

        // add default aliases that jj CLI provides
        let default_aliases = [
            ("trunk()", r#"latest(
              remote_bookmarks(exact:"main", exact:"origin") |
              remote_bookmarks(exact:"master", exact:"origin") |
              remote_bookmarks(exact:"trunk", exact:"origin") |
              remote_bookmarks(exact:"main", exact:"upstream") |
              remote_bookmarks(exact:"master", exact:"upstream") |
              remote_bookmarks(exact:"trunk", exact:"upstream") |
              root()
            )"#),
            ("builtin_immutable_heads()", "trunk() | tags() | untracked_remote_bookmarks()"),
            ("immutable_heads()", "builtin_immutable_heads()"),
            ("immutable()", "::(immutable_heads() | root())"),
            ("mutable()", "~immutable()"),
        ];

        for (name, def) in default_aliases {
            let _ = aliases_map.insert(name, def);
        }

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
        let symbol_resolver = SymbolResolver::new(self.repo.as_ref(), extensions.symbol_resolvers())
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
        let extensions = Arc::new(revset::RevsetExtensions::default());
        let id_prefix_context = IdPrefixContext::new(extensions);
        let index = id_prefix_context
            .populate(self.repo.as_ref())
            .unwrap_or_else(|_| jj_lib::id_prefix::IdPrefixIndex::empty());
        let prefix_len = index
            .shortest_change_prefix_len(self.repo.as_ref(), commit.change_id())
            .wrap_err("failed to get shortest prefix length")?;
        let full_id = commit.change_id().reverse_hex();
        let len = prefix_len.max(min_len).min(full_id.len());
        Ok(full_id[..len].to_string())
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
}
