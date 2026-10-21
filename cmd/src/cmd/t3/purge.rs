use std::fs::{self, File};
use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use clap::Args;
use colored::Colorize;
use eyre::{bail, eyre, Context, Result};
use rusqlite::{Connection, OpenFlags, TransactionBehavior};

use super::server::Server;

/// Launchd label of the background service `t3 service install` sets up
const SERVICE_LABEL: &str = "com.t3tools.t3code.service";

/// Newest T3 migration the purge statements were checked against (T3 0.1.0).
/// A later schema can add thread tables the purge would miss, so it is refused
/// until the statements are checked again
const SUPPORTED_MIGRATION: i64 = 62;

/// Set on the detached copy so it does not detach again
const DETACHED_ENV: &str = "CMD_T3_PURGE_DETACHED";

pub const DEFAULT_IDLE_TIMEOUT_MINUTES: u64 = 60;

const POLL: Duration = Duration::from_secs(5);

/// Launchd allows the service 90s to exit, so this covers a slow shutdown
const STOP_TIMEOUT: Duration = Duration::from_secs(120);

/// Turns and side effects still in flight. Stopping T3 interrupts them, and a
/// deleted thread's pending cleanup effects need their outbox rows
const BUSY: &str = "SELECT
       (SELECT COUNT(*) FROM orchestration_v2_projection_runs
        WHERE status IN ('queued', 'preparing', 'starting', 'running', 'waiting'))
     + (SELECT COUNT(*) FROM orchestration_v2_effect_outbox
        WHERE status IN ('pending', 'running'))";

/// Deleted threads that nothing live still points at. T3 0.1.0 has no foreign
/// keys, so a live fork parent, subagent link, context transfer, or scheduled
/// task naming a purged thread would dangle; those threads are kept
const PURGE_SET: &str = "WITH live AS (
       SELECT thread_id, payload_json FROM orchestration_v2_projection_threads
       WHERE deleted_at IS NULL
     ),
     link(thread_id, other_id) AS (
       SELECT thread_id, child_thread_id FROM orchestration_v2_projection_subagents
       UNION ALL SELECT child_thread_id, thread_id FROM orchestration_v2_projection_subagents
       UNION ALL SELECT source_thread_id, target_thread_id FROM orchestration_v2_projection_context_transfers
       UNION ALL SELECT target_thread_id, source_thread_id FROM orchestration_v2_projection_context_transfers
     )
     SELECT thread.thread_id FROM orchestration_v2_projection_threads AS thread
     WHERE thread.deleted_at IS NOT NULL
       AND NOT EXISTS (SELECT 1 FROM live WHERE instr(live.payload_json, thread.thread_id) > 0)
       AND NOT EXISTS (
         SELECT 1 FROM link JOIN live ON live.thread_id = link.other_id
         WHERE link.thread_id = thread.thread_id
       )
       AND NOT EXISTS (SELECT 1 FROM scheduled_tasks AS task WHERE task.thread_id = thread.thread_id)
       AND NOT EXISTS (
         SELECT 1 FROM orchestration_v2_effect_outbox AS effect
         WHERE effect.thread_id = thread.thread_id AND effect.status IN ('pending', 'running')
       )";

/// Kept events whose `promptSequence` points at an event of a purged thread
const DANGLING_PROMPTS: &str = "SELECT COUNT(*) FROM orchestration_events AS event
     WHERE event.application_event_version = 2
       AND event.aggregate_kind = 'thread'
       AND event.event_type IN ('subagent.updated', 'turn-item.updated')
       AND event.stream_id NOT IN (SELECT thread_id FROM temp.purge)
       AND json_valid(event.payload_json)
       AND json_type(event.payload_json, '$.promptSequence') = 'integer'
       AND (SELECT prompt.stream_id FROM orchestration_events AS prompt
            WHERE prompt.sequence = json_extract(event.payload_json, '$.promptSequence'))
           IN (SELECT thread_id FROM temp.purge)";

/// Every table holding a purged thread's rows, in the order of T3's own
/// projection rebuild. The v1 `projection_threads` rows must go too: T3 imports
/// any v1 thread without a v2 `thread.created` event on every start, and most
/// v1 rows of threads deleted in v2 are still live, so they would come back
const PURGE_STATEMENTS: &[&str] = &[
    // project events stay, since T3 still reads them
    "DELETE FROM orchestration_events
     WHERE aggregate_kind = 'thread' AND stream_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_command_receipts
     WHERE aggregate_kind = 'thread' AND aggregate_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_effect_outbox WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_context_transfers
     WHERE source_thread_id IN (SELECT thread_id FROM temp.purge)
        OR target_thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_context_handoffs WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_checkpoints WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_checkpoint_scopes WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_turn_items WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_plans WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_messages WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_runtime_requests WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_provider_turns WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_provider_threads WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_provider_session_bindings
     WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    // a session still bound to a kept thread stays with it
    "DELETE FROM orchestration_v2_projection_provider_sessions
     WHERE thread_id IN (SELECT thread_id FROM temp.purge)
       AND provider_session_id NOT IN (
         SELECT provider_session_id FROM orchestration_v2_projection_provider_session_bindings
       )",
    "DELETE FROM orchestration_v2_projection_subagents
     WHERE thread_id IN (SELECT thread_id FROM temp.purge)
        OR child_thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_nodes WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_run_attempts WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_runs WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_turn_item_positions WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_projection_threads WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_thread_launch_workflows WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM orchestration_v2_legacy_imports WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM mcp_app_model_context WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM projection_threads WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM projection_thread_messages WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM projection_thread_pull_requests WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM projection_thread_activities WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM projection_turns WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM projection_thread_proposed_plans WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM projection_pending_approvals WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM projection_thread_sessions WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM checkpoint_diff_blobs WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
    "DELETE FROM provider_session_runtime WHERE thread_id IN (SELECT thread_id FROM temp.purge)",
];

/// The projection high-water mark T3's verify step compares with the newest v2
/// thread event, which may have belonged to a purged thread
const RESET_HIGH_WATER: &str = "UPDATE orchestration_v2_projection_metadata
     SET last_sequence = (
           SELECT COALESCE(MAX(sequence), 0) FROM orchestration_events
           WHERE application_event_version = 2 AND aggregate_kind = 'thread'
         ),
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
     WHERE projection_name = 'thread-projections'";

/// Invariants T3's verify step and v1 importer rely on, each counting violations
const CHECKS: &[(&str, &str)] = &[
    (
        "thread.created events without a thread",
        "SELECT COUNT(*) FROM (
           SELECT stream_id FROM orchestration_events
           WHERE application_event_version = 2 AND aggregate_kind = 'thread' AND event_type = 'thread.created'
           EXCEPT SELECT thread_id FROM orchestration_v2_projection_threads
         )",
    ),
    (
        "threads without a thread.created event",
        "SELECT COUNT(*) FROM (
           SELECT thread_id FROM orchestration_v2_projection_threads
           EXCEPT SELECT stream_id FROM orchestration_events
           WHERE application_event_version = 2 AND aggregate_kind = 'thread' AND event_type = 'thread.created'
         )",
    ),
    (
        "v1 threads T3 would import again",
        "SELECT COUNT(*) FROM projection_threads AS thread
         WHERE NOT EXISTS (
           SELECT 1 FROM orchestration_events AS event
           WHERE event.application_event_version = 2 AND event.aggregate_kind = 'thread'
             AND event.stream_id = thread.thread_id AND event.event_type = 'thread.created'
         )",
    ),
];

#[derive(Debug, Clone, Args)]
pub struct PurgeArgs {
    /// Minutes to wait for running turns to finish before giving up
    #[arg(long, default_value_t = DEFAULT_IDLE_TIMEOUT_MINUTES)]
    pub idle_timeout_minutes: u64,

    /// Count the deleted threads that would be purged without stopping T3
    #[arg(long)]
    pub dry_run: bool,
}

/// Permanently remove deleted threads from T3's database and VACUUM it. T3's
/// `thread.delete` only marks a thread deleted and keeps every row, so the
/// file never shrinks otherwise. T3 must be stopped for this, so the command
/// waits for running turns to finish, stops the service, and starts it again
pub fn purge(args: PurgeArgs) -> Result<()> {
    let server = Server::locate()?;
    let db_path = server.db_path();
    let threads = {
        let db = open(&db_path, true)?;
        check_schema(&db)?;
        db.query_row(&format!("SELECT COUNT(*) FROM ({PURGE_SET})"), [], |row| {
            row.get::<_, i64>(0)
        })?
    };

    let size = db_size(&db_path);
    if threads == 0 {
        println!("no deleted threads to purge ({})", human(size));
        return Ok(());
    }

    if args.dry_run {
        println!(
            "would stop T3, purge {threads} deleted threads, and VACUUM {}",
            human(size)
        );
        return Ok(());
    }

    if !server.service_managed {
        bail!("T3 is not running as its background service, so it cannot be restarted after the purge");
    }

    let service = Service::new()?;
    if std::env::var_os(DETACHED_ENV).is_none() && descends_from(server.pid) {
        return detach(&server.userdata, &args);
    }

    wait_for_idle(
        &db_path,
        Duration::from_secs(args.idle_timeout_minutes * 60),
    )?;
    let stopped = service.stop(server.pid)?;
    let report = purge_db(&db_path);
    stopped.start()?;

    let (purged, rows) = report?;
    println!(
        "{}",
        format!(
            "purged {purged} deleted threads ({rows} rows), {} -> {}",
            human(size),
            human(db_size(&db_path))
        )
        .green()
    );

    Ok(())
}

fn open(path: &Path, read_only: bool) -> Result<Connection> {
    let flags = if read_only {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    };

    let conn = Connection::open_with_flags(path, flags)
        .wrap_err_with(|| format!("open {}", path.display()))?;

    conn.busy_timeout(Duration::from_secs(10))?;
    Ok(conn)
}

fn check_schema(conn: &Connection) -> Result<()> {
    let migration: i64 = conn.query_row(
        "SELECT MAX(migration_id) FROM effect_sql_migrations",
        [],
        |row| row.get(0),
    )?;

    if migration != SUPPORTED_MIGRATION {
        bail!(
            "T3's schema is at migration {migration}, but the purge was checked against {SUPPORTED_MIGRATION}; check the new migrations for thread tables and update SUPPORTED_MIGRATION"
        );
    }

    Ok(())
}

/// Delete the purge set in one transaction that rolls back unless T3's
/// invariants still hold, then VACUUM. Returns the thread and row counts
fn purge_db(path: &Path) -> Result<(i64, usize)> {
    let mut conn = open(path, false)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    check_schema(&tx)?;
    tx.execute_batch(&format!(
        "CREATE TEMP TABLE purge(thread_id TEXT PRIMARY KEY); INSERT INTO temp.purge {PURGE_SET};"
    ))?;

    let threads: i64 = tx.query_row("SELECT COUNT(*) FROM temp.purge", [], |row| row.get(0))?;
    let dangling: i64 = tx.query_row(DANGLING_PROMPTS, [], |row| row.get(0))?;
    if dangling > 0 {
        bail!("{dangling} kept events point at prompts of deleted threads; nothing was purged");
    }

    let mut rows = 0;
    for statement in PURGE_STATEMENTS {
        rows += tx
            .execute(statement, [])
            .wrap_err_with(|| format!("purge statement failed: {statement}"))?;
    }

    tx.execute(RESET_HIGH_WATER, [])?;
    for (name, check) in CHECKS {
        let violations: i64 = tx.query_row(check, [], |row| row.get(0))?;
        if violations > 0 {
            bail!("the purge would leave {violations} {name}; rolled back");
        }
    }

    tx.commit()?;

    // VACUUM rewrites the whole file through the WAL, so the checkpoint is what
    // hands the space back. It is also the only way to switch an existing
    // database to incremental auto_vacuum, which lets T3 return freed pages to
    // disk on its own afterwards (the same switch `t3 doctor` makes)
    conn.execute_batch("DROP TABLE temp.purge; PRAGMA auto_vacuum = INCREMENTAL; VACUUM;")?;
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
    let integrity: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        bail!("quick_check after VACUUM reported: {integrity}");
    }

    Ok((threads, rows))
}

fn wait_for_idle(db_path: &Path, timeout: Duration) -> Result<()> {
    let conn = open(db_path, true)?;
    let deadline = Instant::now() + timeout;
    let mut reported = None;
    loop {
        let busy: i64 = conn.query_row(BUSY, [], |row| row.get(0))?;
        if busy == 0 {
            return Ok(());
        }

        if Instant::now() >= deadline {
            bail!(
                "T3 still has {busy} running turns or effects after {}m; nothing was stopped",
                timeout.as_secs() / 60
            );
        }

        if reported != Some(busy) {
            println!("waiting for {busy} running turns or effects to finish before stopping T3");
            reported = Some(busy);
        }

        thread::sleep(POLL);
    }
}

/// Whether this process runs under `pid`, as it does when a T3 thread runs it
fn descends_from(pid: u32) -> bool {
    let mut current = std::process::id();
    while current > 1 {
        if current == pid {
            return true;
        }

        let parent = Command::new("ps")
            .args(["-o", "ppid=", "-p", &current.to_string()])
            .output()
            .ok()
            .and_then(|output| String::from_utf8_lossy(&output.stdout).trim().parse().ok());

        match parent {
            Some(parent) => current = parent,
            None => return false,
        }
    }

    false
}

/// Continue in a copy that outlives T3, since stopping T3 ends the thread that
/// ran this command along with its processes
fn detach(userdata: &Path, args: &PurgeArgs) -> Result<()> {
    let log_path = userdata.join("logs/cmd-t3-purge.log");
    let log = File::create(&log_path).wrap_err_with(|| format!("create {}", log_path.display()))?;
    let exe = std::env::current_exe()?;

    let mut command = Command::new(exe);
    command
        // the binary picks its tool from argv[0]
        .arg0("cmd")
        .args(["t3", "purge", "--idle-timeout-minutes"])
        .arg(args.idle_timeout_minutes.to_string())
        .env(DETACHED_ENV, "1")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);

    // a new session leaves T3's process group, and once this process exits the
    // copy is reparented to launchd, out of reach of a process-tree kill
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(())
        });
    }

    let child = command.spawn().wrap_err("start the detached purge")?;
    println!(
        "this runs inside T3, so the purge continues as pid {} and stops T3 once every turn finishes; log: {}",
        child.id(),
        log_path.display()
    );

    Ok(())
}

/// T3's launchd service. It is KeepAlive, so it is booted out rather than
/// killed, or launchd would restart it mid-purge
struct Service {
    domain: String,
    plist: PathBuf,
}

impl Service {
    fn new() -> Result<Self> {
        let plist = crate::fsutil::home_dir()?
            .join("Library/LaunchAgents")
            .join(format!("{SERVICE_LABEL}.plist"));

        if !plist.is_file() {
            bail!("no T3 service plist at {}", plist.display());
        }

        // SAFETY: getuid cannot fail
        let uid = unsafe { libc::getuid() };
        Ok(Self {
            domain: format!("gui/{uid}"),
            plist,
        })
    }

    fn stop(self, pid: u32) -> Result<Stopped> {
        launchctl(&["bootout", &format!("{}/{SERVICE_LABEL}", self.domain)])?;
        println!("stopped T3");

        // from here on the service comes back even if the purge fails
        let stopped = Stopped(Some(self));
        let deadline = Instant::now() + STOP_TIMEOUT;
        while alive(pid) {
            if Instant::now() >= deadline {
                bail!(
                    "T3 server pid {pid} did not exit within {}s",
                    STOP_TIMEOUT.as_secs()
                );
            }

            thread::sleep(Duration::from_millis(250));
        }

        Ok(stopped)
    }
}

/// A stopped service, started again when dropped so no error path leaves T3 down
struct Stopped(Option<Service>);

impl Stopped {
    fn start(mut self) -> Result<()> {
        let service = self
            .0
            .take()
            .ok_or_else(|| eyre!("T3 service already started"))?;
        bootstrap(&service)
    }
}

impl Drop for Stopped {
    fn drop(&mut self) {
        if let Some(service) = self.0.take() {
            if let Err(error) = bootstrap(&service) {
                eprintln!("{}", format!("could not start T3 again: {error}").red());
            }
        }
    }
}

fn bootstrap(service: &Service) -> Result<()> {
    launchctl(&[
        "bootstrap",
        &service.domain,
        &service.plist.to_string_lossy(),
    ])?;
    println!("started T3");
    Ok(())
}

fn launchctl(args: &[&str]) -> Result<()> {
    let output = Command::new("launchctl").args(args).output()?;
    if !output.status.success() {
        bail!(
            "launchctl {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

fn alive(pid: u32) -> bool {
    // SAFETY: signal 0 only checks that the pid exists
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

/// Database plus WAL, since a VACUUM is not done until the WAL is truncated
fn db_size(path: &Path) -> u64 {
    let wal = path.with_extension("sqlite-wal");
    [path, wal.as_path()]
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .map(|metadata| metadata.len())
        .sum()
}

fn human(bytes: u64) -> String {
    format!("{:.1}G", bytes as f64 / (1u64 << 30) as f64)
}
