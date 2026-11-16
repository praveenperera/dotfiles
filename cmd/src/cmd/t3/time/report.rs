//! Builds the per-day and per-project report in US Central time and prints
//! it as text or JSON

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use chrono::{DateTime, Duration, NaiveDate, TimeZone as _, Utc};
use chrono_tz::{America::Chicago, Tz};
use colored::Colorize;
use serde::Serialize;

use super::estimate::{estimate, Estimate, Params};
use super::merge::{bounds, Merged, Origin, Project, Tagged};
use super::model::{Prompt, Source, SourceOutcome, SourceReport, View, ViewKind};

pub const TIME_ZONE: Tz = Chicago;

/// Inclusive US Central days the report covers
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Days {
    pub first: NaiveDate,
    pub last: NaiveDate,
}

impl Days {
    /// Midnight in US Central at the start of `day`, in UTC
    pub fn start_of(day: NaiveDate) -> DateTime<Utc> {
        let midnight = day.and_hms_opt(0, 0, 0).unwrap_or_default();
        // US Central changes clocks at 2 AM, so midnight always exists once
        TIME_ZONE
            .from_local_datetime(&midnight)
            .earliest()
            .map_or_else(|| midnight.and_utc(), |at| at.with_timezone(&Utc))
    }

    pub fn iter(self) -> impl Iterator<Item = NaiveDate> {
        self.first
            .iter_days()
            .take_while(move |day| *day <= self.last)
    }
}

/// How reading one machine went
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum MachineStatus {
    Read { sources: Vec<SourceReport> },
    Missing { reason: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct MachineRow {
    pub name: String,
    #[serde(flatten)]
    pub status: MachineStatus,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Totals {
    pub low_minutes: i64,
    pub middle_minutes: i64,
    pub high_minutes: i64,
    pub prompts: usize,
}

impl Totals {
    fn new(estimate: &Estimate, prompts: usize) -> Self {
        Self {
            low_minutes: estimate.low.total().num_minutes(),
            middle_minutes: estimate.middle.total().num_minutes(),
            high_minutes: estimate.high.total().num_minutes(),
            prompts,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Count {
    pub machine: String,
    pub source: Source,
    pub prompts: usize,
    pub views: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DayRow {
    pub day: NaiveDate,
    #[serde(flatten)]
    pub totals: Totals,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectRow {
    pub project: String,
    #[serde(flatten)]
    pub totals: Totals,
    pub from: Vec<Count>,
}

/// The span and machines T3 thread-view records cover
#[derive(Debug, Clone, Serialize)]
pub struct ViewCoverage {
    pub first: DateTime<Utc>,
    pub last: DateTime<Utc>,
    pub machines: Vec<String>,
    pub opens: usize,
    pub visits: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub days: Days,
    pub time_zone: &'static str,
    pub project_filter: Option<String>,
    pub params: Params,
    pub total: Totals,
    pub from: Vec<Count>,
    pub by_day: Vec<DayRow>,
    pub by_project: Vec<ProjectRow>,
    pub machines: Vec<MachineRow>,
    pub views: Option<ViewCoverage>,
    /// Prompts left out because a source with more context covers their session
    pub covered: BTreeMap<Source, usize>,
}

pub struct Selection<'a> {
    pub prompts: Vec<&'a Tagged<Prompt>>,
    pub views: Vec<&'a Tagged<View>>,
}

impl Selection<'_> {
    fn estimate(&self, params: &Params, start: DateTime<Utc>, end: DateTime<Utc>) -> Estimate {
        let prompts: Vec<Prompt> = self.prompts.iter().map(|p| p.item.clone()).collect();
        let views: Vec<View> = self.views.iter().map(|v| v.item.clone()).collect();
        estimate(&prompts, &views, params).clip(start, end)
    }

    fn counts(&self) -> Vec<Count> {
        let mut counts: BTreeMap<&Origin, (usize, usize)> = BTreeMap::new();
        for prompt in &self.prompts {
            counts.entry(&prompt.origin).or_default().0 += 1;
        }

        for view in &self.views {
            counts.entry(&view.origin).or_default().1 += 1;
        }

        counts
            .into_iter()
            .map(|(origin, (prompts, views))| Count {
                machine: origin.machine.clone(),
                source: origin.source,
                prompts,
                views,
            })
            .collect()
    }
}

pub struct Inputs<'a> {
    pub days: Days,
    pub params: Params,
    pub project_filter: Option<String>,
    pub merged: &'a Merged,
    pub selection: Selection<'a>,
    pub machines: Vec<MachineRow>,
}

pub fn build(inputs: Inputs<'_>) -> Report {
    let Inputs {
        days,
        params,
        project_filter,
        merged,
        selection,
        machines,
    } = inputs;

    let start = Days::start_of(days.first);
    let end = Days::start_of(days.last + Duration::days(1));
    let estimate = selection.estimate(&params, start, end);

    let by_day = days
        .iter()
        .filter_map(|day| {
            let day_start = Days::start_of(day);
            let day_end = Days::start_of(day + Duration::days(1));
            let prompts = selection
                .prompts
                .iter()
                .filter(|p| day_start <= p.item.at && p.item.at < day_end)
                .count();

            let totals = Totals::new(&estimate.clip(day_start, day_end), prompts);
            (totals.high_minutes > 0 || prompts > 0).then_some(DayRow { day, totals })
        })
        .collect();

    let mut projects: BTreeMap<&Project, Selection> = BTreeMap::new();
    for prompt in &selection.prompts {
        projects
            .entry(&prompt.project)
            .or_insert_with(Selection::empty)
            .prompts
            .push(prompt);
    }

    for view in &selection.views {
        projects
            .entry(&view.project)
            .or_insert_with(Selection::empty)
            .views
            .push(view);
    }

    let mut by_project: Vec<ProjectRow> = projects
        .into_iter()
        .map(|(project, part)| ProjectRow {
            project: project.label().to_owned(),
            totals: Totals::new(&part.estimate(&params, start, end), part.prompts.len()),
            from: part.counts(),
        })
        .collect();
    by_project.sort_by_key(|row| std::cmp::Reverse(row.totals.high_minutes));

    Report {
        days,
        time_zone: "America/Chicago",
        project_filter,
        params,
        total: Totals::new(&estimate, selection.prompts.len()),
        from: selection.counts(),
        by_day,
        by_project,
        machines,
        views: view_coverage(&selection.views),
        covered: merged.covered.clone(),
    }
}

impl<'a> Selection<'a> {
    fn empty() -> Self {
        Self {
            prompts: Vec::new(),
            views: Vec::new(),
        }
    }
}

fn view_coverage(views: &[&Tagged<View>]) -> Option<ViewCoverage> {
    let visits: Vec<_> = views
        .iter()
        .filter(|view| view.item.kind == ViewKind::Visit)
        .collect();
    let (first, last) = bounds(visits.iter().map(|view| &view.item.at))?;

    let machines: BTreeSet<String> = views.iter().map(|v| v.origin.machine.clone()).collect();
    Some(ViewCoverage {
        first,
        last,
        machines: machines.into_iter().collect(),
        opens: views.len() - visits.len(),
        visits: visits.len(),
    })
}

fn hours(minutes: i64) -> String {
    format!("{:.1}h", minutes as f64 / 60.0)
}

fn central(at: DateTime<Utc>) -> String {
    at.with_timezone(&TIME_ZONE)
        .format("%b %-d %-I:%M %p")
        .to_string()
}

fn counts_text(counts: &[Count]) -> String {
    counts
        .iter()
        .map(|count| {
            let mut text = format!(
                "{} {} {}",
                count.machine,
                count.source.label(),
                count.prompts
            );
            if count.views > 0 {
                let _ = write!(text, " (+{} views)", count.views);
            }

            text
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn row(label: &str, width: usize, totals: &Totals) -> String {
    format!(
        "{label:<width$}  {:>7}  {:>7}  {:>7}  {:>7}",
        hours(totals.low_minutes),
        hours(totals.middle_minutes),
        hours(totals.high_minutes),
        totals.prompts,
    )
}

fn header(label: &str, width: usize) -> String {
    format!(
        "{label:<width$}  {:>7}  {:>7}  {:>7}  {:>7}",
        "low", "middle", "high", "prompts"
    )
    .bold()
    .to_string()
}

fn outcome_text(report: &SourceReport) -> String {
    let label = report.source.label();
    match &report.outcome {
        SourceOutcome::Read { prompts, views, .. } if *views > 0 => {
            format!("{label} {prompts} prompts, {views} views")
        }
        SourceOutcome::Read { prompts, .. } => format!("{label} {prompts} prompts"),
        SourceOutcome::Absent { reason } => format!("{label} {reason}"),
        SourceOutcome::Failed { reason } => {
            format!("{label} {}", format!("failed: {reason}").red())
        }
    }
}

pub fn print(report: &Report) {
    let days = report.days;
    println!(
        "{}",
        format!(
            "Agent thread time, {} to {} (US Central)",
            days.first.format("%a %b %-d"),
            days.last.format("%a %b %-d, %Y")
        )
        .bold()
    );

    if let Some(filter) = &report.project_filter {
        let matched: Vec<&str> = report
            .by_project
            .iter()
            .map(|row| row.project.as_str())
            .collect();
        let matched = match matched.is_empty() {
            true => "no project".to_owned(),
            false => matched.join(", "),
        };

        println!("Project filter `{filter}` matched {matched}");
    }

    println!();
    println!("{}", header("", 12));
    println!("{}", row("Total", 12, &report.total));
    if !report.from.is_empty() {
        println!("  from {}", counts_text(&report.from));
    }

    println!();
    println!("{}", header("Day", 12));
    for day in &report.by_day {
        println!(
            "{}",
            row(&day.day.format("%a %b %-d").to_string(), 12, &day.totals)
        );
    }

    let width = report
        .by_project
        .iter()
        .map(|row| row.project.chars().count())
        .max()
        .unwrap_or(7)
        .clamp(7, 60);

    println!();
    println!("{}", header("Project", width));
    for project in &report.by_project {
        println!("{}", row(&project.project, width, &project.totals));
        println!("  from {}", counts_text(&project.from).dimmed());
    }

    println!();
    println!("{}", "Machines".bold());
    for machine in &report.machines {
        let status = match &machine.status {
            MachineStatus::Read { sources } => sources
                .iter()
                .map(outcome_text)
                .collect::<Vec<_>>()
                .join(" · "),
            MachineStatus::Missing { reason } => format!("missing: {reason}").red().to_string(),
        };

        println!("  {:<8} {status}", machine.name);
    }

    print_notes(report);
}

fn print_notes(report: &Report) {
    let params = &report.params;
    println!();
    println!("{}", "Notes".bold());

    let covered: Vec<String> = report
        .covered
        .iter()
        .map(|(source, count)| format!("{count} {}", source.label()))
        .collect();
    if !covered.is_empty() {
        println!(
            "  Left out as already covered by T3 or a transcript: {} prompts",
            covered.join(", ")
        );
    }

    match &report.views {
        Some(views) => println!(
            "  T3 thread views on {}: {} opens, {} visits (from {} to {}). Opens and visits add to \
             the high estimate only, since T3 also records them for a page reload or a hidden tab. \
             T3 keeps every visit for only about a day and one per thread before that, so earlier \
             days rest on typed prompts",
            views.machines.join(", "),
            views.opens,
            views.visits,
            central(views.first),
            central(views.last),
        ),
        None => {
            println!("  No T3 thread visits in this range; the estimates rest on typed prompts")
        }
    }

    println!(
        "  low: bursts of typed prompts, split after {} min of silence, +{} min after each burst",
        params.low_break.num_minutes(),
        params.burst_tail.num_minutes()
    );
    println!(
        "  middle: low + the gap from an agent reply to your next prompt, when it is at most {} min",
        params.read_cap.num_minutes()
    );
    println!(
        "  high: bursts of prompts and thread opens split after {} min + reply gaps + time a thread stayed on screen within {} min of your last action",
        params.high_break.num_minutes(),
        params.read_cap.num_minutes()
    );
    println!("  Projects overlap in time, so project rows can add up to more than the total");
}
