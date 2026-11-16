//! Turns typed prompts and thread views into a low, middle, and high estimate
//! of the time spent working. Every estimate is a union of time spans, so
//! parallel threads on any number of machines never count the same minute
//! twice

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use super::model::{Conversation, Prompt, View, ViewKind};

/// Thresholds for the three estimates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Params {
    /// Silence that ends a burst of activity in the low and middle estimates
    #[serde(with = "minutes")]
    pub low_break: Duration,
    /// Silence that ends a burst in the high estimate
    #[serde(with = "minutes")]
    pub high_break: Duration,
    /// Time counted after a burst's last prompt, for writing it
    #[serde(with = "minutes")]
    pub burst_tail: Duration,
    /// Longest gap from an agent reply to the next prompt that still counts
    /// as reading the reply; a longer one means the user was away. Also how
    /// long thread visits keep counting after the user's last action
    #[serde(with = "minutes")]
    pub read_cap: Duration,
}

impl Params {
    /// How far past the report's days activity can still change them: a
    /// burst joins actions up to `high_break` apart and runs `burst_tail`
    /// past each, and a reply gap or a run of visits lasts up to `read_cap`.
    /// Reading this much more on each side makes a day's estimate the same
    /// whatever range the report covers; the reply that starts a gap can end
    /// any time after its prompt, so each source also keeps the prompt before
    /// the window when its reply lands inside
    pub fn context(&self) -> Duration {
        self.high_break.max(self.burst_tail).max(self.read_cap)
    }
}

mod minutes {
    use chrono::Duration;
    use serde::Serializer;

    pub fn serialize<S: Serializer>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(value.num_minutes())
    }
}

/// Two visits to a thread this close together mean it stayed on screen in
/// between; T3 sends at most one every 10 s while the thread changes
const VISIT_CHAIN: Duration = Duration::minutes(2);

pub type Span = (DateTime<Utc>, DateTime<Utc>);

/// Sorted, non-overlapping time spans
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Spans(Vec<Span>);

impl Spans {
    pub fn new(mut spans: Vec<Span>) -> Self {
        spans.retain(|(start, end)| start < end);
        spans.sort();

        let mut merged: Vec<Span> = Vec::with_capacity(spans.len());
        for (start, end) in spans {
            match merged.last_mut() {
                Some(last) if start <= last.1 => last.1 = last.1.max(end),
                _ => merged.push((start, end)),
            }
        }

        Self(merged)
    }

    pub fn union(&self, other: &Self) -> Self {
        Self::new(self.0.iter().chain(&other.0).copied().collect())
    }

    /// The part of these spans inside `[start, end)`
    pub fn clip(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self::new(
            self.0
                .iter()
                .map(|(from, to)| ((*from).max(start), (*to).min(end)))
                .collect(),
        )
    }

    pub fn total(&self) -> Duration {
        self.0
            .iter()
            .fold(Duration::zero(), |sum, (start, end)| sum + (*end - *start))
    }
}

/// The three estimates as time spans
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Estimate {
    pub low: Spans,
    pub middle: Spans,
    pub high: Spans,
}

impl Estimate {
    pub fn clip(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            low: self.low.clip(start, end),
            middle: self.middle.clip(start, end),
            high: self.high.clip(start, end),
        }
    }
}

/// Estimate the time behind these prompts and views
///
/// - low: bursts of typed prompts, a new burst after `low_break` of silence,
///   each running `burst_tail` past its last prompt
/// - middle: low, plus each gap from an agent reply to the next prompt in the
///   same conversation when it is at most `read_cap`
/// - high: bursts of prompts and thread opens split at `high_break`, the reply
///   gaps, and the time T3 shows a thread on screen within `read_cap` of the
///   last prompt or open. Opens stay out of the lower two because T3 also
///   records one when a page reloads onto a thread
pub fn estimate(prompts: &[Prompt], views: &[View], params: &Params) -> Estimate {
    let mut typed: Vec<DateTime<Utc>> = prompts.iter().map(|prompt| prompt.at).collect();
    typed.sort();

    let mut actions = typed.clone();
    actions.extend(
        views
            .iter()
            .filter(|view| view.kind == ViewKind::Open)
            .map(|view| view.at),
    );
    actions.sort();

    let low = bursts(&typed, params.low_break, params.burst_tail);
    let reading = reading(prompts, params.read_cap);
    let middle = low.union(&reading);
    let high = bursts(&actions, params.high_break, params.burst_tail)
        .union(&reading)
        .union(&viewing(views, &actions, params.read_cap));

    Estimate { low, middle, high }
}

/// Sorted action times grouped into bursts
fn bursts(actions: &[DateTime<Utc>], gap: Duration, tail: Duration) -> Spans {
    let joins = actions
        .windows(2)
        .filter(|pair| pair[1] - pair[0] <= gap)
        .map(|pair| (pair[0], pair[1]));
    let tails = actions.iter().map(|at| (*at, *at + tail));

    Spans::new(joins.chain(tails).collect())
}

/// Each conversation's gaps from an agent reply to the next prompt
fn reading(prompts: &[Prompt], cap: Duration) -> Spans {
    let mut conversations: BTreeMap<&Conversation, Vec<&Prompt>> = BTreeMap::new();
    for prompt in prompts {
        conversations
            .entry(&prompt.conversation)
            .or_default()
            .push(prompt);
    }

    let mut spans = Vec::new();
    for mut conversation in conversations.into_values() {
        conversation.sort_by_key(|prompt| prompt.at);
        for pair in conversation.windows(2) {
            let (Some(reply), next) = (pair[0].reply_done, pair[1].at) else {
                continue;
            };

            if reply < next && next - reply <= cap {
                spans.push((reply, next));
            }
        }
    }

    Spans::new(spans)
}

/// Time a thread stayed on screen, from runs of T3 visits. T3 also records
/// visits for a thread left open in a hidden tab while its agent works, so a
/// visit counts only within `cap` of the user's last action
fn viewing(views: &[View], actions: &[DateTime<Utc>], cap: Duration) -> Spans {
    let mut threads: BTreeMap<&str, Vec<DateTime<Utc>>> = BTreeMap::new();
    for view in views {
        threads.entry(&view.thread).or_default().push(view.at);
    }

    let mut spans = Vec::new();
    for mut times in threads.into_values() {
        times.sort();
        for pair in times.windows(2) {
            let (start, end) = (pair[0], pair[1]);
            if end - start > VISIT_CHAIN {
                continue;
            }

            // the user's last action before the run's end bounds it
            let index = actions.partition_point(|at| *at <= end);
            let Some(last_action) = index.checked_sub(1).map(|i| actions[i]) else {
                continue;
            };

            spans.push((start, end.min(last_action + cap)));
        }
    }

    Spans::new(spans)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;

    use super::super::model::Source;
    use super::*;

    fn at(minutes: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 2, 0, 0, 0).unwrap() + Duration::minutes(minutes)
    }

    fn prompt(thread: &str, minute: i64, reply: Option<i64>) -> Prompt {
        Prompt {
            at: at(minute),
            source: Source::T3,
            conversation: Conversation::Thread(thread.to_owned()),
            place: None,
            reply_done: reply.map(at),
        }
    }

    fn view(thread: &str, minute: i64, kind: ViewKind) -> View {
        View {
            at: at(minute),
            thread: thread.to_owned(),
            place: None,
            kind,
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

    fn minutes(spans: &Spans) -> i64 {
        spans.total().num_minutes()
    }

    #[test]
    fn parallel_threads_are_merged_not_summed() {
        // two threads typed into over the same stretch count once
        let prompts = [
            prompt("a", 0, None),
            prompt("b", 2, None),
            prompt("a", 6, None),
            prompt("b", 8, None),
        ];

        let estimate = estimate(&prompts, &[], &params());
        assert_eq!(minutes(&estimate.low), 13);
    }

    #[test]
    fn break_thresholds_split_bursts() {
        // 20 minutes of silence splits the low burst but not the high one
        let prompts = [prompt("a", 0, None), prompt("a", 20, None)];

        let estimate = estimate(&prompts, &[], &params());
        assert_eq!(minutes(&estimate.low), 10);
        assert_eq!(minutes(&estimate.middle), 10);
        assert_eq!(minutes(&estimate.high), 25);
    }

    #[test]
    fn reply_gaps_count_up_to_the_cap() {
        let prompts = [
            // reply at 12, next prompt at 24: 12 minutes of reading counts
            prompt("a", 0, Some(12)),
            prompt("a", 24, Some(30)),
            // reply at 30, next prompt at 60: longer than the cap, so away
            prompt("a", 60, None),
        ];

        let estimate = estimate(&prompts, &[], &params());
        // bursts: 0-5, 24-29, 60-65; reading: 12-24
        assert_eq!(minutes(&estimate.low), 15);
        assert_eq!(minutes(&estimate.middle), 27);
    }

    #[test]
    fn visits_count_only_near_the_users_last_action() {
        let prompts = [prompt("a", 0, None)];
        // a thread left open while its agent works keeps recording visits
        let views: Vec<View> = (0..=60)
            .map(|minute| view("a", minute, ViewKind::Visit))
            .collect();

        let estimate = estimate(&prompts, &views, &params());
        assert_eq!(minutes(&estimate.low), 5);
        assert_eq!(minutes(&estimate.high), 15);
    }

    #[test]
    fn opens_count_only_in_the_high_estimate() {
        let prompts = [prompt("a", 0, None)];
        let views = [view("b", 20, ViewKind::Open)];

        let estimate = estimate(&prompts, &views, &params());
        assert_eq!(minutes(&estimate.low), 5);
        assert_eq!(minutes(&estimate.middle), 5);
        assert_eq!(minutes(&estimate.high), 25);
    }
}
