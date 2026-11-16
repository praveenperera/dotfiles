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

/// The machines and visit span T3 thread-view records cover
#[derive(Debug, Clone, Serialize)]
pub struct ViewCoverage {
    pub first_visit: Option<DateTime<Utc>>,
    pub last_visit: Option<DateTime<Utc>>,
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
    /// ClickHouse messages in the days from sessions no machine reported,
    /// which no estimate counts
    pub unverified: Unverified,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Unverified {
    pub sessions: usize,
    pub messages: usize,
}

pub struct Selection<'a> {
    pub prompts: Vec<&'a Tagged<Prompt>>,
    pub views: Vec<&'a Tagged<View>>,
    pub unverified: Vec<&'a Tagged<Prompt>>,
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

    // the selection reaches past the days for context; estimates use all of
    // it and clip to the days, and counts take only what falls inside them
    let start = Days::start_of(days.first);
    let end = Days::start_of(days.last + Duration::days(1));
    let estimate = selection.estimate(&params, start, end);
    let shown = selection.within(start, end);

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
        .map(|(project, part)| {
            let shown = part.within(start, end);
            ProjectRow {
                project: project.label().to_owned(),
                totals: Totals::new(&part.estimate(&params, start, end), shown.prompts.len()),
                from: shown.counts(),
            }
        })
        .filter(|row| row.totals.high_minutes > 0 || row.totals.prompts > 0)
        .collect();
    by_project.sort_by_key(|row| std::cmp::Reverse(row.totals.high_minutes));

    Report {
        days,
        time_zone: "America/Chicago",
        project_filter,
        params,
        total: Totals::new(&estimate, shown.prompts.len()),
        from: shown.counts(),
        by_day,
        by_project,
        machines,
        views: view_coverage(&shown.views),
        covered: merged.covered.clone(),
        unverified: Unverified {
            sessions: shown
                .unverified
                .iter()
                .map(|p| &p.item.conversation)
                .collect::<BTreeSet<_>>()
                .len(),
            messages: shown.unverified.len(),
        },
    }
}

impl<'a> Selection<'a> {
    fn empty() -> Self {
        Self {
            prompts: Vec::new(),
            views: Vec::new(),
            unverified: Vec::new(),
        }
    }

    /// The prompts and views in `[start, end)`
    fn within(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let inside = |at: DateTime<Utc>| start <= at && at < end;
        Self {
            prompts: self
                .prompts
                .iter()
                .copied()
                .filter(|p| inside(p.item.at))
                .collect(),
            views: self
                .views
                .iter()
                .copied()
                .filter(|v| inside(v.item.at))
                .collect(),
            unverified: self
                .unverified
                .iter()
                .copied()
                .filter(|p| inside(p.item.at))
                .collect(),
        }
    }
}

fn view_coverage(views: &[&Tagged<View>]) -> Option<ViewCoverage> {
    let visits: Vec<_> = views
        .iter()
        .filter(|view| view.item.kind == ViewKind::Visit)
        .collect();
    if views.is_empty() {
        return None;
    }

    let span = bounds(visits.iter().map(|view| &view.item.at));
    let machines: BTreeSet<String> = views.iter().map(|v| v.origin.machine.clone()).collect();
    Some(ViewCoverage {
        first_visit: span.map(|(first, _)| first),
        last_visit: span.map(|(_, last)| last),
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
        // ClickHouse cannot tell typed prompts from other messages
        SourceOutcome::Read { prompts, .. } if report.source == Source::ClickHouse => {
            format!("{label} {prompts} messages")
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
            "  Left out as already covered by T3 or a transcript: {} messages",
            covered.join(", ")
        );
    }

    let unverified = report.unverified;
    if unverified.messages > 0 {
        println!(
            "  Not counted: ClickHouse has {} messages in {} T3 sessions no machine reported, such \
             as from a machine that was not read. It cannot tell a typed prompt from a message an \
             agent, another thread, or Homebased sent",
            unverified.messages, unverified.sessions
        );
    }

    match &report.views {
        Some(views) => println!(
            "  T3 thread views on {}: {} opens, {}. Opens and visits add to the high estimate \
             only, since T3 also records them for a page reload or a hidden tab. T3 keeps every \
             visit for only about a day and one per thread before that, so earlier days rest on \
             typed prompts",
            views.machines.join(", "),
            views.opens,
            match (views.first_visit, views.last_visit) {
                (Some(first), Some(last)) => format!(
                    "{} visits (from {} to {})",
                    views.visits,
                    central(first),
                    central(last)
                ),
                _ => "no visits".to_owned(),
            },
        ),
        None => {
            println!("  No T3 thread views in this range; the estimates rest on typed prompts")
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

#[cfg(test)]
mod tests {
    use super::super::model::Conversation;
    use super::*;

    fn prompt(at: DateTime<Utc>) -> Tagged<Prompt> {
        Tagged {
            item: Prompt {
                at,
                source: Source::T3,
                conversation: Conversation::Thread("a".to_owned()),
                place: None,
                reply_done: None,
            },
            origin: Origin {
                machine: "code".to_owned(),
                source: Source::T3,
            },
            project: Project::Known("~/code/app".to_owned()),
        }
    }

    fn params() -> Params {
        Params {
            low_break: Duration::minutes(10),
            high_break: Duration::minutes(30),
            burst_tail: Duration::minutes(5),
            read_cap: Duration::minutes(15),
        }
    }

    #[test]
    fn a_burst_across_midnight_counts_its_part_inside_the_days() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 13).unwrap();
        let midnight = Days::start_of(day);
        // 11:55 PM the night before, read for context, and 12:03 AM
        let prompts = [
            prompt(midnight - Duration::minutes(5)),
            prompt(midnight + Duration::minutes(3)),
        ];

        let report = build(Inputs {
            days: Days {
                first: day,
                last: day,
            },
            params: params(),
            project_filter: None,
            merged: &Merged::default(),
            selection: Selection {
                prompts: prompts.iter().collect(),
                views: Vec::new(),
                unverified: Vec::new(),
            },
            machines: Vec::new(),
        });

        // the burst runs from 11:55 PM to 12:08 AM, so 8 minutes fall on the day
        assert_eq!(report.total.low_minutes, 8);
        assert_eq!(report.total.prompts, 1);
        assert_eq!(report.by_project[0].totals.prompts, 1);
        assert_eq!(report.from[0].prompts, 1);
    }

    #[test]
    fn a_reply_gap_across_midnight_counts_its_part_inside_the_days() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 13).unwrap();
        let midnight = Days::start_of(day);
        // a prompt at 5:55 PM whose reply ends at 11:58 PM, then one at 12:03 AM
        let mut before = prompt(midnight - Duration::minutes(365));
        before.item.reply_done = Some(midnight - Duration::minutes(2));
        let prompts = [before, prompt(midnight + Duration::minutes(3))];

        let report = build(Inputs {
            days: Days {
                first: day,
                last: day,
            },
            params: params(),
            project_filter: None,
            merged: &Merged::default(),
            selection: Selection {
                prompts: prompts.iter().collect(),
                views: Vec::new(),
                unverified: Vec::new(),
            },
            machines: Vec::new(),
        });

        // reading from 12:00 to 12:03 and the burst from 12:03 to 12:08
        assert_eq!(report.total.middle_minutes, 8);
        assert_eq!(report.total.prompts, 1);
    }
}
