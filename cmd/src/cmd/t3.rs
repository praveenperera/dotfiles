mod server;
mod transcript;

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::{Args, Subcommand};
use colored::Colorize;
use eyre::{eyre, Context, Result};
use serde_json::Value;

use crate::fsutil;
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
}

#[derive(Debug, Clone, Args)]
pub struct ImportArgs {
    /// Session id or transcript path [default: newest interactive session in --cwd]
    session: Option<String>,

    /// Only consider sessions from this CLI
    #[arg(long, short)]
    provider: Option<Provider>,

    /// Directory the session ran in [default: current directory]
    #[arg(long)]
    cwd: Option<PathBuf>,

    /// Read a SessionEnd hook payload on stdin, log the result, and always exit 0
    #[arg(
        long,
        value_name = "PROVIDER",
        conflicts_with_all = ["session", "provider", "cwd", "dry_run"]
    )]
    hook: Option<Provider>,

    /// Show what would be imported without changing T3
    #[arg(long)]
    dry_run: bool,

    /// Run the hook import in this process instead of a detached child
    #[arg(long, hide = true, requires = "hook")]
    foreground: bool,
}

pub fn run_with_flags(flags: T3) -> Result<()> {
    match flags.subcommand {
        T3Cmd::Import(args) => match args.hook {
            Some(provider) if args.foreground => {
                run_hook(provider);
                Ok(())
            }
            Some(provider) => {
                detach_hook(provider);
                Ok(())
            }
            None => import(args),
        },
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
    Deleted {
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
            Self::Deleted { thread_id } => {
                format!("skipped {source}: its T3 thread {thread_id} was deleted")
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
        None => Session::newest_in(&cwd, args.provider)?.ok_or_else(|| {
            eyre!(
                "no interactive Claude or Codex session from the last 14 days ran in {}",
                cwd.display()
            )
        })?,
    };

    // naming a session by hand is how a deleted import comes back
    let outcome = import_session(&session, args.dry_run, true)?;
    let line = outcome.describe(&session);
    match outcome {
        Outcome::Imported { .. } => println!("{}", line.green()),
        Outcome::Skipped(_) | Outcome::Deleted { .. } => println!("{}", line.yellow()),
        Outcome::Planned { .. } | Outcome::AlreadyInT3 { .. } => println!("{line}"),
    }

    Ok(())
}

fn import_session(session: &Session, dry_run: bool, revive_deleted: bool) -> Result<Outcome> {
    if !session.has_prompt {
        return Ok(Outcome::Skipped("no user prompt"));
    }

    let server = Server::locate()?;
    let db = server.db(dry_run)?;
    // Codex fires SessionEnd again whenever T3 itself finishes running an
    // imported thread, so a thread deleted in T3 must not reappear on its own
    match db.thread_for_session(session, !revive_deleted)? {
        Some(thread) if thread.deleted => {
            return Ok(Outcome::Deleted {
                thread_id: thread.id,
            })
        }
        Some(thread) => {
            return Ok(Outcome::AlreadyInT3 {
                thread_id: thread.id,
            })
        }
        None => {}
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

    if dry_run {
        return Ok(Outcome::Planned {
            title,
            root: target.root,
            new_project: target.project_id.is_none(),
        });
    }

    let client = server.client()?;
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

    // the binding goes in first so the thread never appears without its cursor
    db.bind(&thread, session)?;
    if let Err(error) = client.create_thread(&thread) {
        // a timeout or dropped response can follow a committed create, and that
        // thread still needs its cursor, so only an absent thread gets unbound
        if matches!(db.thread_exists(&thread.id), Ok(false)) {
            let _ = db.unbind(&thread.id);
        }

        return Err(error);
    }

    Ok(Outcome::Imported {
        thread_id: thread.id,
        title,
        root: target.root,
    })
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

/// Claude kills SessionEnd hooks after 1.5s by default, and a cold t3 start can
/// take longer, so the payload moves to a child in its own process group
fn detach_hook(provider: Provider) {
    let spawned = (|| -> Result<()> {
        let mut payload = Vec::new();
        std::io::stdin().read_to_end(&mut payload)?;

        let mut child = Command::new(std::env::current_exe()?)
            .args(["t3", "import", "--foreground", "--hook", provider.label()])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()?;

        child
            .stdin
            .take()
            .ok_or_else(|| eyre!("child has no stdin"))?
            .write_all(&payload)?;

        Ok(())
    })();

    if let Err(error) = spawned {
        let _ = append_log(&format!("{} error: {error:#}", provider.label()));
    }
}

/// Hooks run after the CLI has exited, so results go to a log instead of the terminal
fn run_hook(provider: Provider) {
    let line = match hook_outcome(provider) {
        Ok(line) => line,
        Err(error) => format!("error: {error:#}"),
    };

    let _ = append_log(&format!("{} {line}", provider.label()));
}

fn hook_outcome(provider: Provider) -> Result<String> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let payload: Value = serde_json::from_str(&input).wrap_err("parse hook payload")?;

    let field = |snake: &str, camel: &str| {
        payload
            .get(snake)
            .or_else(|| payload.get(camel))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };

    let transcript = field("transcript_path", "transcriptPath").map(PathBuf::from);
    let session = match (transcript, field("session_id", "sessionId")) {
        (Some(path), _) if path.is_file() => Session::read(provider, &path)?,
        (_, Some(id)) => Session::find_by_id(&id, Some(provider))?,
        _ => return Err(eyre!("hook payload has no transcript_path or session_id")),
    };

    // SDK, `-p`, `exec`, and T3's own sessions fire the same hook
    if !session.interactive {
        return Ok(Outcome::Skipped("not an interactive CLI session").describe(&session));
    }

    Ok(import_session(&session, false, false)?.describe(&session))
}

fn append_log(line: &str) -> Result<()> {
    let dir = fsutil::home_dir()?.join(".cache/cmd");
    std::fs::create_dir_all(&dir)?;

    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %Z");
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("t3-import.log"))?;

    writeln!(log, "{stamp} {line}")?;
    Ok(())
}
