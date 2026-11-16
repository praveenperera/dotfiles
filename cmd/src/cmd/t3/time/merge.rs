//! Combines every machine's collection into one set of prompts and views,
//! keeping each message once and naming the project it belongs to

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Utc};

use super::model::{Collection, Conversation, Driver, Prompt, SessionId, Source, View};

/// Where a kept record came from
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Origin {
    pub machine: String,
    pub source: Source,
}

#[derive(Debug, Clone)]
pub struct Tagged<T> {
    pub item: T,
    pub origin: Origin,
    pub project: Project,
}

/// The project a record belongs to, or why none is known
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Project {
    Known(String),
    Unknown,
}

impl Project {
    pub fn label(&self) -> &str {
        match self {
            Self::Known(path) => path,
            Self::Unknown => "(unknown)",
        }
    }
}

#[derive(Debug, Default)]
pub struct Merged {
    pub prompts: Vec<Tagged<Prompt>>,
    pub views: Vec<Tagged<View>>,
    /// Prompts left out because a source with more context covers their
    /// session, by the source they came from
    pub covered: BTreeMap<Source, usize>,
}

/// Merge collections. A session's prompts come from the most authoritative
/// source that knows it: T3, which records who typed each message, then the
/// session's own transcript, then ClickHouse, which is used only for sessions
/// no reachable machine reported
pub fn merge(collections: &[Collection]) -> Merged {
    let drivers = drivers(collections);
    let projects = ProjectMap::new(collections);

    let mut merged = Merged::default();
    let mut seen_prompts = HashSet::new();
    let mut seen_views = HashSet::new();

    for collection in collections {
        for prompt in &collection.prompts {
            let origin = Origin {
                machine: origin_machine(collection, prompt.source),
                source: prompt.source,
            };

            let keep = keep_prompt(prompt, &drivers)
                && seen_prompts.insert((prompt.conversation.clone(), prompt.at));
            if !keep {
                *merged.covered.entry(prompt.source).or_default() += 1;
                continue;
            }

            merged.prompts.push(Tagged {
                project: projects.project(prompt.place.as_deref()),
                item: prompt.clone(),
                origin,
            });
        }

        for view in &collection.views {
            if !seen_views.insert((view.thread.clone(), view.at, view.kind)) {
                continue;
            }

            merged.views.push(Tagged {
                project: projects.project(view.place.as_deref()),
                item: view.clone(),
                origin: Origin {
                    machine: collection.machine.clone(),
                    source: Source::T3,
                },
            });
        }
    }

    merged
}

/// ClickHouse is one database for every machine, so its records are not
/// credited to the machine that happened to read it
fn origin_machine(collection: &Collection, source: Source) -> String {
    match source {
        Source::ClickHouse => "proxy".to_owned(),
        _ => collection.machine.clone(),
    }
}

/// The best-ranked driver any machine reported for each session
fn drivers(collections: &[Collection]) -> HashMap<SessionId, Driver> {
    let mut drivers: HashMap<SessionId, Driver> = HashMap::new();
    for known in collections.iter().flat_map(|c| &c.sessions) {
        let better = drivers
            .get(&known.session)
            .is_none_or(|current| known.driver.rank() > current.rank());
        if better {
            drivers.insert(known.session.clone(), known.driver.clone());
        }
    }

    drivers
}

fn keep_prompt(prompt: &Prompt, drivers: &HashMap<SessionId, Driver>) -> bool {
    let Conversation::Session(session) = &prompt.conversation else {
        // T3 prompts are what every other source is checked against
        return true;
    };

    match (prompt.source, drivers.get(session)) {
        // only ClickHouse knows this session, such as one from an offline machine
        (Source::ClickHouse, None) => true,
        // the machine that has the session reported its own prompts, or found
        // that no one typed into it
        (Source::ClickHouse, Some(_)) => false,
        // a terminal session imported into T3 keeps the prompts typed before
        // the import only in its transcript
        (_, Some(Driver::T3 { created, .. })) => prompt.at < *created,
        (_, _) => true,
    }
}

/// Maps working directories onto project roots
struct ProjectMap {
    /// Every T3 project root, with the home directory written as `~`
    roots: BTreeSet<String>,
}

impl ProjectMap {
    fn new(collections: &[Collection]) -> Self {
        Self {
            roots: collections
                .iter()
                .flat_map(|c| &c.projects)
                .map(|root| tilde(root))
                .collect(),
        }
    }

    /// The T3 project a directory is in, else the directory itself
    fn project(&self, place: Option<&str>) -> Project {
        let Some(place) = place else {
            return Project::Unknown;
        };

        let place = tilde(place.trim_end_matches('/'));
        if let Some(root) = self.containing_root(&place) {
            return Project::Known(root.clone());
        }

        // T3 worktrees live under ~/.t3/worktrees/<project dir name>/<branch>
        if let Some(name) = worktree_project(&place) {
            let mut roots = self
                .roots
                .iter()
                .filter(|root| root.rsplit('/').next() == Some(name));
            if let (Some(root), None) = (roots.next(), roots.next()) {
                return Project::Known(root.clone());
            }
        }

        // each scratch thread gets its own directory, which would make every
        // one a project of its own
        if place.starts_with("~/.t3/scratch/") {
            return Project::Known("~/.t3/scratch".to_owned());
        }

        Project::Known(place)
    }

    fn containing_root(&self, place: &str) -> Option<&String> {
        self.roots
            .iter()
            .filter(|root| {
                place == root.as_str()
                    || place
                        .strip_prefix(root.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
            })
            .max_by_key(|root| root.len())
    }
}

fn worktree_project(place: &str) -> Option<&str> {
    let rest = place.strip_prefix("~/.t3/worktrees/")?;
    rest.split('/').next().filter(|name| !name.is_empty())
}

/// Write a home directory as `~`, so the same project on macOS and Linux
/// machines is one project
pub fn tilde(path: &str) -> String {
    for prefix in ["/Users/", "/home/"] {
        let Some(rest) = path.strip_prefix(prefix) else {
            continue;
        };

        return match rest.split_once('/') {
            Some((_user, inner)) => format!("~/{inner}"),
            None => "~".to_owned(),
        };
    }

    path.to_owned()
}

/// Earliest and latest time among records
pub fn bounds<'a>(
    times: impl Iterator<Item = &'a DateTime<Utc>>,
) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    times.fold(None, |acc, at| match acc {
        None => Some((*at, *at)),
        Some((first, last)) => Some((first.min(*at), last.max(*at))),
    })
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;

    use super::super::model::{Agent, KnownSession, SCHEMA_VERSION};
    use super::*;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 2, hour, 0, 0).unwrap()
    }

    fn session(id: &str) -> SessionId {
        SessionId {
            agent: Agent::Claude,
            id: id.to_owned(),
        }
    }

    fn prompt(source: Source, conversation: Conversation, hour: u32, place: &str) -> Prompt {
        Prompt {
            at: at(hour),
            source,
            conversation,
            place: Some(place.to_owned()),
            reply_done: None,
        }
    }

    #[test]
    fn each_message_comes_from_the_best_source_that_has_it() {
        let code = Collection {
            schema: SCHEMA_VERSION,
            machine: "code".to_owned(),
            projects: vec!["/home/praveen/code/seq".to_owned()],
            sessions: vec![
                KnownSession {
                    session: session("t3-run"),
                    driver: Driver::T3 {
                        thread: "thread".to_owned(),
                        created: at(5),
                    },
                },
                KnownSession {
                    session: session("worker"),
                    driver: Driver::Automated,
                },
            ],
            prompts: vec![
                prompt(
                    Source::T3,
                    Conversation::Thread("thread".to_owned()),
                    6,
                    "/home/praveen/code/seq",
                ),
                // typed in the terminal before the session was imported into T3
                prompt(
                    Source::ClaudeCode,
                    Conversation::Session(session("t3-run")),
                    4,
                    "/home/praveen/code/seq",
                ),
                // the same session after the import, which T3 already has
                prompt(
                    Source::ClaudeCode,
                    Conversation::Session(session("t3-run")),
                    6,
                    "/home/praveen/code/seq",
                ),
            ],
            ..Collection::default()
        };

        let proxy = Collection {
            schema: SCHEMA_VERSION,
            machine: "code".to_owned(),
            prompts: vec![
                prompt(
                    Source::ClickHouse,
                    Conversation::Session(session("t3-run")),
                    6,
                    "/x",
                ),
                prompt(
                    Source::ClickHouse,
                    Conversation::Session(session("worker")),
                    7,
                    "/x",
                ),
                // a session from a machine that was offline
                prompt(
                    Source::ClickHouse,
                    Conversation::Session(session("laptop")),
                    8,
                    "/Users/praveen/.t3/worktrees/seq/fix-ci",
                ),
            ],
            ..Collection::default()
        };

        let merged = merge(&[code, proxy]);
        let kept: Vec<_> = merged
            .prompts
            .iter()
            .map(|p| (p.item.source, p.item.at, p.project.label().to_owned()))
            .collect();

        assert_eq!(
            kept,
            [
                (Source::T3, at(6), "~/code/seq".to_owned()),
                (Source::ClaudeCode, at(4), "~/code/seq".to_owned()),
                (Source::ClickHouse, at(8), "~/code/seq".to_owned()),
            ]
        );
        assert_eq!(merged.covered.get(&Source::ClaudeCode), Some(&1));
        assert_eq!(merged.covered.get(&Source::ClickHouse), Some(&2));
    }
}
