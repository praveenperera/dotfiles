//! `cmd t3 time`: estimates the time spent working in agent threads across
//! every machine, from T3's message log, local Claude Code and Codex
//! transcripts, and the proxy's ClickHouse chat history

mod clickhouse;
mod estimate;
mod fleet;
mod merge;
mod model;
mod report;
mod t3db;
mod transcripts;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::Args;
use eyre::{bail, Result};

use estimate::Params;
use fleet::{Config, Machine, Reach};
use merge::{tilde, Project};
use model::{Collection, Source, Window};
use report::{Days, Inputs, MachineRow, MachineStatus, Selection};

#[derive(Debug, Clone, Args)]
pub struct TimeArgs {
    /// First US Central day to count, as YYYY-MM-DD [default: 6 days before --until]
    #[arg(long, value_name = "DATE")]
    since: Option<NaiveDate>,

    /// Last US Central day to count, inclusive [default: today]
    #[arg(long, value_name = "DATE")]
    until: Option<NaiveDate>,

    /// Count only projects whose root is this path or contains this text, such
    /// as `~/code/sequence/cloud-project-system` or `sequence`
    #[arg(long, short)]
    project: Option<String>,

    /// Read only these machines [default: every Homebased Fleet machine that
    /// runs T3 or an agent CLI, or the list in ~/.config/cmd/thread-time.toml]
    #[arg(long = "machine", short, value_name = "NAME")]
    machines: Vec<String>,

    /// Minutes of silence that end a burst in the low and middle estimates
    #[arg(long, default_value_t = 10, value_name = "MINUTES")]
    low_break: i64,

    /// Minutes of silence that end a burst in the high estimate
    #[arg(long, default_value_t = 30, value_name = "MINUTES")]
    high_break: i64,

    /// Minutes counted after each burst's last prompt
    #[arg(long, default_value_t = 5, value_name = "MINUTES")]
    burst_tail: i64,

    /// Longest gap, in minutes, from an agent reply to the next prompt that
    /// counts as reading it; longer means away
    #[arg(long, default_value_t = 15, value_name = "MINUTES")]
    read_cap: i64,

    /// Leave out the ClickHouse chat history
    #[arg(long)]
    no_clickhouse: bool,

    /// Print the report as JSON
    #[arg(long)]
    json: bool,
}

/// What one machine sends back to `cmd t3 time`
#[derive(Debug, Clone, Args)]
pub struct CollectArgs {
    /// Start of the window, RFC 3339
    #[arg(long)]
    since: DateTime<Utc>,

    /// End of the window, exclusive, RFC 3339
    #[arg(long)]
    until: DateTime<Utc>,

    /// Also read the ClickHouse chat history, whose credentials live here
    #[arg(long)]
    clickhouse: bool,
}

/// Print this machine's collection as JSON, for `cmd t3 time` on another one
pub fn collect(args: CollectArgs) -> Result<()> {
    let window = Window {
        since: args.since,
        until: args.until,
    };

    // `cmd t3 time` renames the collection after the machine it asked
    let collection = collect_local(short_hostname(), &window, args.clickhouse);
    serde_json::to_writer(std::io::stdout().lock(), &collection)?;
    Ok(())
}

fn short_hostname() -> String {
    std::process::Command::new("hostname")
        .arg("-s")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default()
}

fn collect_local(machine: String, window: &Window, clickhouse: bool) -> Collection {
    let mut collection = Collection::new(machine);
    collection.add(Source::T3, t3db::collect(window));
    collection.add(Source::ClaudeCode, transcripts::collect_claude(window));
    collection.add(Source::Codex, transcripts::collect_codex(window));
    if clickhouse {
        collection.add(Source::ClickHouse, clickhouse::collect(window));
    }

    collection
}

pub fn run(args: TimeArgs) -> Result<()> {
    let params = params(&args)?;
    let days = days(&args)?;
    let window = Window {
        since: Days::start_of(days.first),
        until: Days::start_of(days.last + Duration::days(1)),
    };

    let config = Config::load()?;
    let mut machines = fleet::machines(&config)?;
    if !args.machines.is_empty() {
        machines.retain(|machine| args.machines.contains(&machine.name));
        if machines.is_empty() {
            bail!("none of {:?} is a known machine", args.machines);
        }
    }

    let clickhouse_machine = (!args.no_clickhouse).then(|| config.clickhouse_machine());
    let results = collect_all(&machines, &window, clickhouse_machine);

    let mut rows = Vec::new();
    let mut collections = Vec::new();
    for (machine, result) in results {
        let status = match result {
            Ok(collection) => {
                let sources = collection.sources.clone();
                collections.push(collection);
                MachineStatus::Read { sources }
            }
            Err(error) => MachineStatus::Missing {
                reason: format!("{error:#}"),
            },
        };

        rows.push(MachineRow {
            name: machine.name.clone(),
            status,
        });
    }

    if let Some(name) = clickhouse_machine {
        if !machines.iter().any(|machine| machine.name == name) {
            rows.push(MachineRow {
                name: name.to_owned(),
                status: MachineStatus::Missing {
                    reason: "not read, so ClickHouse was not read either".to_owned(),
                },
            });
        }
    }

    let merged = merge::merge(&collections);
    let selection = select(&merged, args.project.as_deref());
    let report = report::build(Inputs {
        days,
        params,
        project_filter: args.project.clone(),
        merged: &merged,
        selection,
        machines: rows,
    });

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        report::print(&report);
    }

    Ok(())
}

fn params(args: &TimeArgs) -> Result<Params> {
    let values = [
        ("--low-break", args.low_break),
        ("--high-break", args.high_break),
        ("--burst-tail", args.burst_tail),
        ("--read-cap", args.read_cap),
    ];

    if let Some((flag, _)) = values.iter().find(|(_, value)| *value < 0) {
        bail!("{flag} must not be negative");
    }

    if args.high_break < args.low_break {
        bail!("--high-break must be at least --low-break");
    }

    Ok(Params {
        low_break: Duration::minutes(args.low_break),
        high_break: Duration::minutes(args.high_break),
        burst_tail: Duration::minutes(args.burst_tail),
        read_cap: Duration::minutes(args.read_cap),
    })
}

fn days(args: &TimeArgs) -> Result<Days> {
    let today = Utc::now().with_timezone(&report::TIME_ZONE).date_naive();
    let last = args.until.unwrap_or(today);
    let first = args.since.unwrap_or(last - Duration::days(6));
    if first > last {
        bail!("--since {first} is after --until {last}");
    }

    Ok(Days { first, last })
}

/// Read every machine at once; a machine that fails is reported, not fatal
fn collect_all<'a>(
    machines: &'a [Machine],
    window: &Window,
    clickhouse_machine: Option<&str>,
) -> Vec<(&'a Machine, Result<Collection>)> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = machines
            .iter()
            .map(|machine| {
                let clickhouse = clickhouse_machine == Some(machine.name.as_str());
                let handle = scope.spawn(move || match &machine.reach {
                    Reach::Local => Ok(collect_local(machine.name.clone(), window, clickhouse)),
                    Reach::Ssh(remote) => {
                        let mut collection = fleet::collect_remote(remote, window, clickhouse)?;
                        // the remote names itself by hostname; the report uses Fleet names
                        collection.machine = machine.name.clone();
                        Ok(collection)
                    }
                });

                (machine, handle)
            })
            .collect();

        handles
            .into_iter()
            .map(|(machine, handle)| {
                let result = handle
                    .join()
                    .unwrap_or_else(|_| Err(eyre::eyre!("collector panicked")));
                (machine, result)
            })
            .collect()
    })
}

/// Prompts and views in projects matching the filter, or all of them
fn select<'a>(merged: &'a merge::Merged, filter: Option<&str>) -> Selection<'a> {
    let matches = |project: &Project| match (filter, project) {
        (None, _) => true,
        (Some(_), Project::Unknown) => false,
        (Some(filter), Project::Known(root)) => {
            let filter = tilde(filter.trim_end_matches('/'));
            root == &filter || root.contains(filter.as_str())
        }
    };

    Selection {
        prompts: merged
            .prompts
            .iter()
            .filter(|prompt| matches(&prompt.project))
            .collect(),
        views: merged
            .views
            .iter()
            .filter(|view| matches(&view.project))
            .collect(),
    }
}
