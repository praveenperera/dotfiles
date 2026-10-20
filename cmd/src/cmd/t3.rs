mod background;
mod cleanup;
mod picker;
mod purge;
mod server;
mod transcript;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Args, Subcommand};
use colored::Colorize;
use eyre::{bail, eyre, Result};

use background::BackgroundAgent;
use cleanup::CleanupArgs;
use purge::PurgeArgs;
use server::{Db, NewThread, Server};
use transcript::{same_dir, Provider, Session};

#[derive(Debug, Clone)]
pub struct T3 {
    pub subcommand: T3Cmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum T3Cmd {
    /// Continue a Claude Code or Codex CLI session as a T3 Code thread
    Import(ImportArgs),

    /// Delete settled threads that have been idle for a while
    Cleanup(CleanupArgs),

    /// Stop T3, permanently remove deleted threads from its database, VACUUM
    /// it, and start T3 again
    Purge(PurgeArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ImportArgs {
    /// Session id or transcript path [default: pick from sessions in --cwd with
    /// fzf, or the newest one without a terminal]
    session: Option<String>,

    /// Only consider sessions from this CLI
    #[arg(long, short)]
    provider: Option<Provider>,

    /// Directory the session ran in [default: current directory]
    #[arg(long)]
    cwd: Option<PathBuf>,

    /// Show what would be imported without changing T3
    #[arg(long)]
    dry_run: bool,

    /// Stop the Claude background agent holding the session even mid-turn,
    /// cancelling its current run
    #[arg(long, short)]
    force: bool,
}

pub fn run_with_flags(flags: T3) -> Result<()> {
    match flags.subcommand {
        T3Cmd::Import(args) => import(args),
        T3Cmd::Cleanup(args) => cleanup::cleanup(args),
        T3Cmd::Purge(args) => purge::purge(args),
    }
}

enum Outcome {
    Imported {
        thread_id: String,
        title: String,
        root: PathBuf,
    },
    Planned {
        title: String,
        root: PathBuf,
        new_project: bool,
    },
    AlreadyInT3 {
        thread_id: String,
    },
    Skipped(&'static str),
}

impl Outcome {
    fn describe(&self, session: &Session) -> String {
        let source = format!("{} session {}", session.provider.label(), session.id);
        match self {
            Self::Imported {
                thread_id,
                title,
                root,
            } => format!(
                "imported {source} as T3 thread {thread_id} \"{title}\" in {}",
                root.display()
            ),
            Self::Planned {
                title,
                root,
                new_project,
            } => {
                let project = if *new_project {
                    "new project"
                } else {
                    "project"
                };
                format!(
                    "would import {source} as \"{title}\" in {project} {}",
                    root.display()
                )
            }
            Self::AlreadyInT3 { thread_id } => {
                format!("{source} is already T3 thread {thread_id}")
            }
            Self::Skipped(reason) => format!("skipped {source}: {reason}"),
        }
    }
}

fn import(args: ImportArgs) -> Result<()> {
    let cwd = match args.cwd {
        Some(cwd) => cwd,
        None => std::env::current_dir()?,
    };

    let session = match args.session.as_deref() {
        Some(value) if Path::new(value).is_file() => Session::read_path(Path::new(value))?,
        Some(id) => Session::find_by_id(id, args.provider)?,
        None if std::io::stdin().is_terminal() && std::io::stderr().is_terminal() => {
            match pick_session(&cwd, args.provider)? {
                Some(session) => session,
                None => return Ok(()),
            }
        }
        None => Session::recent_in(&cwd, args.provider)?
            .next()
            .map(|(_, session)| session)
            .ok_or_else(|| no_sessions(&cwd))?,
    };

    let outcome = import_session(&session, args.dry_run, args.force)?;
    let line = outcome.describe(&session);
    match outcome {
        Outcome::Imported { .. } => println!("{}", line.green()),
        Outcome::Skipped(_) => println!("{}", line.yellow()),
        Outcome::Planned { .. } | Outcome::AlreadyInT3 { .. } => println!("{line}"),
    }

    Ok(())
}

fn no_sessions(cwd: &Path) -> eyre::Report {
    eyre!(
        "no interactive Claude or Codex session from the last 14 days ran in {}",
        cwd.display()
    )
}

/// Every recent session in `cwd`, so the picker can always toggle between CLIs
fn pick_session(cwd: &Path, start: Option<Provider>) -> Result<Option<Session>> {
    // the in-T3 marker is a hint, so a stopped server only loses the marker
    let db = Server::locate().and_then(|server| server.db(true)).ok();
    let entries: Vec<_> = Session::all_recent_in(cwd, None)?
        .into_iter()
        .map(|(modified, session)| {
            let in_t3 = db
                .as_ref()
                .is_some_and(|db| matches!(db.thread_for_session(&session), Ok(Some(_))));

            picker::Entry {
                modified,
                session,
                in_t3,
            }
        })
        .collect();

    if entries.is_empty() {
        return Err(no_sessions(cwd));
    }

    picker::pick(entries, start)
}

fn import_session(session: &Session, dry_run: bool, force: bool) -> Result<Outcome> {
    if !session.has_prompt {
        return Ok(Outcome::Skipped("no user prompt"));
    }

    let agent = BackgroundAgent::holding(session)?;
    if let Some(agent) = agent.as_ref().filter(|agent| agent.busy && !force) {
        bail!(
            "claude session {} is mid-turn in background agent {}; let it finish or pass --force to stop it",
            session.id,
            agent.id
        );
    }

    let server = Server::locate()?;
    let db = server.db(dry_run)?;
    if let Some(thread_id) = db.thread_for_session(session)? {
        // a thread imported while the agent ran fails every turn until it stops,
        // so importing again repairs it
        release(agent.as_ref(), dry_run)?;
        return Ok(Outcome::AlreadyInT3 { thread_id });
    }

    let target = Target::resolve(&db, &session.cwd)?;
    let model = match &session.model {
        Some(model) => model.clone(),
        None => db
            .latest_model(session.provider)?
            .ok_or_else(|| eyre!("no model recorded for {} in T3", session.provider.label()))?,
    };

    let title = session
        .title
        .clone()
        .unwrap_or_else(|| format!("{} CLI session", session.provider.label()));

    release(agent.as_ref(), dry_run)?;
    if dry_run {
        return Ok(Outcome::Planned {
            title,
            root: target.root,
            new_project: target.project_id.is_none(),
        });
    }

    let client = server.client("cmd t3 import", "5m")?;
    let project_id = match &target.project_id {
        Some(project_id) => project_id.clone(),
        None => client.create_project(&target.root)?,
    };

    let thread = NewThread {
        id: uuid::Uuid::new_v4().to_string(),
        project_id,
        provider: session.provider,
        title: title.clone(),
        model,
        branch: target.branch,
        worktree_path: target.worktree_path,
        runtime_mode: db.runtime_mode(session.provider)?,
    };

    let created = client.create_thread(&thread, session);
    if session.provider == Provider::Claude {
        // a timeout or dropped response can follow a committed create, and that
        // thread still needs its head, so the head is set whenever the row exists
        if let Err(error) = db.set_claude_head(&thread.id, session) {
            // without a head the first turn fails on Claude's session id check,
            // so a thread that can never run is worse than no import
            if created.is_ok() {
                let _ = client.delete_thread(&thread.id);
            }

            return Err(created.err().unwrap_or(error));
        }
    } else {
        created?;
    }

    Ok(Outcome::Imported {
        thread_id: thread.id,
        title,
        root: target.root,
    })
}

/// Stop the background agent holding the session so T3 can resume it. A busy
/// agent only gets here under --force, and stopping it cancels its run
fn release(agent: Option<&BackgroundAgent>, dry_run: bool) -> Result<()> {
    let Some(agent) = agent else {
        return Ok(());
    };

    if dry_run {
        let cancel = if agent.busy {
            " and cancel its current run"
        } else {
            ""
        };

        println!("would stop claude background agent {}{cancel}", agent.id);
        return Ok(());
    }

    agent.stop()?;
    let cancelled = if agent.busy {
        " and cancelled its current run"
    } else {
        ""
    };

    println!(
        "stopped claude background agent {}{cancelled} so T3 can resume it",
        agent.id
    );

    Ok(())
}

/// Where the T3 thread lives. Claude resumes by cwd, so the thread must run in
/// exactly the session's directory, either as a project root or a git worktree
struct Target {
    project_id: Option<String>,
    root: PathBuf,
    worktree_path: Option<PathBuf>,
    branch: Option<String>,
}

impl Target {
    fn resolve(db: &Db, cwd: &Path) -> Result<Self> {
        if let Some(project_id) = db.project_at(cwd)? {
            return Ok(Self {
                project_id: Some(project_id),
                root: cwd.to_owned(),
                worktree_path: None,
                branch: None,
            });
        }

        if let Some(main) = linked_worktree_main(cwd) {
            if let Some(project_id) = db.project_at(&main)? {
                return Ok(Self {
                    project_id: Some(project_id),
                    root: main,
                    worktree_path: Some(cwd.to_owned()),
                    branch: git(cwd, &["branch", "--show-current"]),
                });
            }
        }

        Ok(Self {
            project_id: None,
            root: cwd.to_owned(),
            worktree_path: None,
            branch: None,
        })
    }
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !text.is_empty()).then_some(text)
}

/// Main checkout of `dir` when `dir` is the root of a linked git worktree
fn linked_worktree_main(dir: &Path) -> Option<PathBuf> {
    let top = git(dir, &["rev-parse", "--show-toplevel"])?;
    if !same_dir(Path::new(&top), dir) {
        return None;
    }

    let common = git(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let main = Path::new(&common).parent()?.to_owned();
    (!same_dir(&main, dir)).then_some(main)
}
