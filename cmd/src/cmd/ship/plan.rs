//! `cmd ship plan`: resolve /ship words and repos.toml into one explicit,
//! ordered step plan
//!
//! The model never decides what "end to end" means: this does, so the same words
//! in the same repo always produce the same plan, and anything it cannot resolve
//! is an error instead of a guess. The ship skill and plans saved by earlier runs
//! read its printed lines, its JSON, and its exit codes, so all three are stable

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Args, ValueEnum};
use eyre::{eyre, Context, Result};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as Json;
use toml::Value as Toml;

/// Where repos.toml lives inside the dotfiles checkout
const CONFIG_IN_DOTFILES: &str = "agents/skills/ship/repos.toml";

/// The placeholder a deploy or publish command takes the bump level in
const PART: &str = "<part>";

const SETTLE_SKIPPED: &str = "settle (not in a T3 thread)";

/// Resolve /ship words and repos.toml into an ordered step plan
#[derive(Debug, Clone, Args)]
#[command(after_help = "\
Examples:
  cmd ship plan -- [WORD ...]               plan for the repo in the current directory
  cmd ship plan --repo OWNER/NAME -- ...    plan for a named repo
  cmd ship plan --json -- ...               machine-readable plan
  cmd ship plan --task SLUG -- ...          also save the plan to _scratch/ship/SLUG/plan.json
  cmd ship plan --task SLUG --done STEP...  record finished steps in that plan
  cmd ship plan --check                     validate repos.toml
  cmd ship plan --host HOST -- ...          plan for t3-claude, t3-codex, or none instead of reading the process tree")]
pub struct PlanArgs {
    /// Ship words, such as `+sol -deploy minor`; one argument may hold several
    words: Vec<String>,

    /// owner/name; defaults to the origin remote of the current directory
    #[arg(long)]
    repo: Option<String>,

    /// The repo table; defaults to agents/skills/ship/repos.toml in ~/code/dotfiles
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Print the plan as JSON
    #[arg(long)]
    json: bool,

    /// Validate the repo table
    #[arg(long)]
    check: bool,

    /// Save the plan to _scratch/ship/TASK/plan.json in the checkout
    #[arg(long)]
    task: Option<String>,

    /// Record finished steps in the saved plan; needs --task
    #[arg(long, num_args = 1.., value_name = "STEP")]
    done: Vec<String>,

    /// Where the plan runs; auto reads the process tree
    #[arg(long, value_enum, default_value_t = HostArg::Auto)]
    host: HostArg,
}

pub fn run(args: PlanArgs) -> Result<()> {
    match args.execute() {
        Ok(true) => Ok(()),
        Ok(false) => std::process::exit(1),
        // an unresolvable word or value is the caller's to fix, so it gets one
        // line and exit 2, while a broken repos.toml keeps eyre's report and exit 1
        Err(error) => match error.downcast_ref::<PlanError>() {
            Some(plan_error) => {
                eprintln!("ship: {plan_error}");
                std::process::exit(2);
            }
            None => Err(error),
        },
    }
}

impl PlanArgs {
    /// Runs the command; `Ok(false)` means exit 1 after a failed `--check`
    fn execute(&self) -> Result<bool> {
        let config_path = match &self.config {
            Some(path) => path.clone(),
            None => crate::dotfiles_dir()?.join(CONFIG_IN_DOTFILES),
        };

        let config = Config::load(&config_path)?;

        if self.check {
            let problems = config.check()?;
            for problem in &problems {
                eprintln!("{problem}");
            }

            println!("{} repos, {} problems", config.repos.len(), problems.len());
            return Ok(problems.is_empty());
        }

        let checkout = Checkout(std::env::current_dir().context("no current directory")?);

        if !self.done.is_empty() {
            let Some(task) = &self.task else {
                return Err(PlanError::report("--done needs --task"));
            };

            let done = checkout.task_dir(task)?.mark_done(&self.done)?;
            println!("done: {}", done.join(", "));
            return Ok(true);
        }

        let origin = checkout.remote("origin");
        let repo = self
            .repo
            .as_deref()
            .map(str::to_lowercase)
            .or_else(|| origin.clone())
            .ok_or_else(|| {
                PlanError::report("no GitHub origin remote here; pass --repo OWNER/NAME")
            })?;

        // remotes and changed paths describe the current checkout, so use them only
        // when it is the repo
        let in_checkout = origin.as_deref() == Some(repo.as_str());
        let upstream = in_checkout.then(|| checkout.remote("upstream")).flatten();
        let changed = in_checkout.then(|| checkout.changed_paths()).flatten();

        // skills pass their arguments as one string, so split it here
        let words = self
            .words
            .iter()
            .flat_map(|arg| arg.split_whitespace())
            .collect::<Vec<_>>();

        let mut plan = config.resolve(&repo, &words, upstream.as_deref(), changed.as_deref())?;
        plan.apply_host(self.host.resolve());

        // status mode changes nothing, so it leaves any saved plan alone
        let saved = match &self.task {
            Some(task) if plan.mode != Mode::Status => Some(checkout.task_dir(task)?.save(&plan)?),
            _ => None,
        };

        if self.json {
            println!("{}", serde_json::to_string_pretty(&plan)?);
            return Ok(true);
        }

        plan.print(saved.as_deref());
        Ok(true)
    }
}

/// A word or config value that cannot be resolved without guessing
#[derive(Debug)]
struct PlanError(String);

impl PlanError {
    /// The error as a report, which `run` turns into exit 2
    fn report(message: impl Into<String>) -> eyre::Report {
        Self(message.into()).into()
    }
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PlanError {}

/// One step of a plan, declared in canonical order; typed order never matters
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
enum Step {
    Sol,
    Commit,
    Push,
    Pr,
    Greptile,
    Merge,
    Deploy,
    Publish,
    Clean,
    Settle,
}

impl Step {
    const ALL: [Self; 10] = [
        Self::Sol,
        Self::Commit,
        Self::Push,
        Self::Pr,
        Self::Greptile,
        Self::Merge,
        Self::Deploy,
        Self::Publish,
        Self::Clean,
        Self::Settle,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::Sol => "sol",
            Self::Commit => "commit",
            Self::Push => "push",
            Self::Pr => "pr",
            Self::Greptile => "greptile",
            Self::Merge => "merge",
            Self::Deploy => "deploy",
            Self::Publish => "publish",
            Self::Clean => "clean",
            Self::Settle => "settle",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|step| step.as_str() == name)
    }

    /// A step name or one of the spoken and transcribed variants seen in
    /// Praveen's dictated prompts
    fn from_word(word: &str) -> Option<Self> {
        let name = match word {
            "soul" | "review" | "codex" => "sol",
            "reptile" => "greptile",
            "release" => "deploy",
            "cleanup" | "clean-up" => "clean",
            "settled" | "s" | "set" => "settle",
            "squash" => "merge",
            other => other,
        };

        Self::from_name(name)
    }

    /// Like [`Step::from_word`], but an unknown word is an error naming `raw`,
    /// the word as typed
    fn parse(word: &str, raw: &str) -> Result<Self> {
        Self::from_word(word).ok_or_else(|| {
            let steps = Self::ALL.map(Self::as_str).join(", ");
            PlanError::report(format!("unknown word '{raw}'; steps are: {steps}"))
        })
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where the plan runs, which decides how sol, PR links, waits, and settle are done
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum Host {
    #[serde(rename = "t3-claude")]
    T3Claude,
    #[serde(rename = "t3-codex")]
    T3Codex,
    #[serde(rename = "none")]
    None,
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::T3Claude => "t3-claude",
            Self::T3Codex => "t3-codex",
            Self::None => "none",
        })
    }
}

/// `--host`: a fixed [`Host`], or `auto` to read it from the process tree
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum HostArg {
    Auto,
    #[value(name = "t3-claude")]
    T3Claude,
    #[value(name = "t3-codex")]
    T3Codex,
    None,
}

impl HostArg {
    fn resolve(self) -> Host {
        match self {
            Self::Auto => match crate::process::t3_agent() {
                Some(crate::process::T3Agent::Claude) => Host::T3Claude,
                Some(crate::process::T3Agent::Codex) => Host::T3Codex,
                Option::None => Host::None,
            },
            Self::T3Claude => Host::T3Claude,
            Self::T3Codex => Host::T3Codex,
            Self::None => Host::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Run,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Effort {
    High,
    Xhigh,
}

impl Effort {
    fn from_word(word: &str) -> Option<Self> {
        match word {
            "high" => Some(Self::High),
            "xhigh" | "xi" | "x-high" | "extra-high" => Some(Self::Xhigh),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Xhigh => "xhigh",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Bump {
    Patch,
    Minor,
    Major,
}

impl Bump {
    fn from_word(word: &str) -> Option<Self> {
        match word {
            "patch" => Some(Self::Patch),
            "minor" => Some(Self::Minor),
            "major" => Some(Self::Major),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Patch => "patch",
            Self::Minor => "minor",
            Self::Major => "major",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum MergeMethod {
    Squash,
    Never,
}

/// Which part of repos.toml a plan came from
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum Source {
    #[serde(rename = "repos.toml")]
    Listed,
    #[serde(rename = "external fallback")]
    ExternalFallback,
    #[serde(rename = "default fallback")]
    DefaultFallback,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Listed => "repos.toml",
            Self::ExternalFallback => "external fallback",
            Self::DefaultFallback => "default fallback",
        })
    }
}

/// The resolved plan for one repo; its fields, in this order, are the JSON that
/// `--json` prints and `plan.json` saves
#[derive(Debug, Clone, Serialize)]
struct Plan {
    repo: String,
    source: Source,
    external: bool,
    pr_repo: String,
    mode: Mode,
    confirm: bool,
    steps: Vec<Step>,
    implied: Vec<Step>,
    sol_effort: Effort,
    bump: Bump,
    merge_method: MergeMethod,
    pr_draft: bool,
    // keyed by step, so deploy prints before publish
    commands: BTreeMap<Step, Vec<String>>,
    notes: String,
    host: Host,
    skipped: Vec<String>,

    /// Whether a planned command takes the bump level, which shows the `bump:` line
    #[serde(skip)]
    takes_bump: bool,
}

/// A plan as `plan.json` saves it, with the steps finished so far
#[derive(Serialize)]
struct SavedPlan<'a> {
    #[serde(flatten)]
    plan: &'a Plan,
    done: [Step; 0],
}

impl Plan {
    fn summary(&self) -> String {
        self.steps
            .iter()
            .map(|step| match step {
                Step::Sol => format!("sol 6.1 {} review", self.sol_effort.as_str()),
                Step::Pr if self.pr_draft => format!("draft pr ({})", self.pr_repo),
                Step::Pr => format!("pr ({})", self.pr_repo),
                Step::Greptile => "greptile 5/5".to_owned(),
                Step::Merge => match self.merge_method {
                    MergeMethod::Squash => "merge (squash)".to_owned(),
                    MergeMethod::Never => "merge (never)".to_owned(),
                },
                step => step.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" → ")
    }

    /// Records where the plan runs, and drops settle outside T3, the only place
    /// it works
    fn apply_host(&mut self, host: Host) {
        self.host = host;

        if host == Host::None {
            if let Some(at) = self.steps.iter().position(|&step| step == Step::Settle) {
                self.steps.remove(at);
                self.skipped.push(SETTLE_SKIPPED.to_owned());
            }
        }
    }

    /// How this host does the steps that differ between T3 and elsewhere
    fn host_actions(&self) -> Vec<&'static str> {
        let t3 = self.host != Host::None;
        let has = |step| self.steps.contains(&step);
        let mut lines = Vec::new();

        if has(Step::Sol) {
            lines.push(if t3 {
                "sol via: delegate_task with the Codex provider and model gpt-6.1-sol from orchestrator_capabilities"
            } else {
                "sol via: the Codex review command in review-fix-loop's providers.md#codex-review, through homebased when it will run long"
            });
        }

        if has(Step::Pr) && t3 {
            lines.push("pr link: call link_pull_request with the PR URL");
        }

        if [Step::Greptile, Step::Deploy, Step::Publish]
            .into_iter()
            .any(has)
        {
            lines.push(if self.host == Host::T3Claude {
                "wait: run `homebased wait --next <id>` as a background Bash command with timeout 3300000 (55 minutes, under the hour-long prompt cache), pass its task id to t3_background_wait, and end the turn; if it times out with no event, start it again"
            } else {
                "wait: submit through homebased and end the turn; its event starts the next turn"
            });
        }

        lines
    }

    fn print(&self, saved: Option<&Path>) {
        let tag = if self.external && self.source == Source::Listed {
            ", external"
        } else {
            ""
        };

        println!("ship: {} ({}{tag})", self.repo, self.source);

        if self.mode == Mode::Status {
            println!("mode: status only, change nothing");
        } else if self.confirm {
            println!("confirm: wait for go-ahead");
        }

        println!("plan: {}", self.summary());

        if !self.implied.is_empty() {
            let implied = self
                .implied
                .iter()
                .map(|step| step.as_str())
                .collect::<Vec<_>>();

            println!("implied: {}", implied.join(", "));
        }

        if !self.skipped.is_empty() {
            println!("skipped: {}", self.skipped.join(", "));
        }

        println!("host: {}", self.host);
        for line in self.host_actions() {
            println!("{line}");
        }

        if self.takes_bump {
            println!("bump: {}", self.bump.as_str());
        }

        for (step, commands) in &self.commands {
            println!("{step}:");
            for command in commands {
                println!("  {command}");
            }
        }

        if !self.notes.is_empty() {
            println!("notes: {}", self.notes);
        }

        if let Some(path) = saved {
            println!("saved: {}", path.display());
        }
    }
}

/// One repos.toml table, typed once `--check` would pass it
#[derive(Debug, Default, Deserialize)]
struct Section {
    steps: Vec<String>,
    #[serde(default)]
    deploy: Vec<String>,
    #[serde(default)]
    deploy_paths: Vec<String>,
    #[serde(default)]
    deploy_otherwise: Vec<String>,
    #[serde(default)]
    publish: Vec<String>,
    #[serde(default)]
    greptile: bool,
    #[serde(default)]
    pr_draft: bool,
    merge: Option<String>,
    pr_repo: Option<String>,
    #[serde(default)]
    notes: String,
}

impl Section {
    fn from_toml(name: &str, value: &Toml) -> Result<Self> {
        value
            .clone()
            .try_into()
            .wrap_err_with(|| format!("invalid {name} in repos.toml; run cmd ship plan --check"))
    }

    fn commands(&self, step: Step) -> &[String] {
        match step {
            Step::Deploy => &self.deploy,
            Step::Publish => &self.publish,
            _ => &[],
        }
    }
}

/// Per-field type rules `--check` enforces
#[derive(Debug, Clone, Copy)]
enum FieldKind {
    List,
    Bool,
    Str,
}

impl FieldKind {
    fn of(key: &str) -> Option<Self> {
        match key {
            "steps" | "deploy" | "deploy_paths" | "deploy_otherwise" | "publish" => {
                Some(Self::List)
            }
            "greptile" | "pr_draft" => Some(Self::Bool),
            "merge" | "pr_repo" | "notes" => Some(Self::Str),
            _ => None,
        }
    }

    /// The problem with `value` for this kind of field, if any
    fn problem(self, value: &Toml) -> Option<&'static str> {
        let fits = match self {
            Self::List => value
                .as_array()
                .is_some_and(|items| items.iter().all(Toml::is_str)),
            Self::Bool => value.is_bool(),
            Self::Str => value.is_str(),
        };

        (!fits).then_some(match self {
            Self::List => "must be a list of strings",
            Self::Bool => "must be true or false",
            Self::Str => "must be a string",
        })
    }
}

/// repos.toml, kept loosely typed so `--check` can report each bad value
#[derive(Debug, Default, Deserialize)]
struct Config {
    owners: Option<Toml>,
    default: Option<Toml>,
    external: Option<Toml>,
    #[serde(default)]
    repos: Ordered<Toml>,
}

impl Config {
    fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .wrap_err_with(|| format!("could not read {}", path.display()))?;

        Self::parse(&text).wrap_err_with(|| format!("could not parse {}", path.display()))
    }

    fn parse(text: &str) -> Result<Self> {
        Ok(toml::from_str(text)?)
    }

    fn owners(&self) -> HashSet<String> {
        self.owners
            .as_ref()
            .and_then(Toml::as_array)
            .into_iter()
            .flatten()
            .filter_map(Toml::as_str)
            .map(str::to_lowercase)
            .collect()
    }

    fn fallback(&self, name: &str) -> Result<Section> {
        let table = match name {
            "external" => &self.external,
            _ => &self.default,
        };

        let table = table
            .as_ref()
            .ok_or_else(|| eyre!("repos.toml has no [{name}] table"))?;

        Section::from_toml(&format!("[{name}]"), table)
    }

    /// Builds the plan for `repo` from its config entry and the typed words
    ///
    /// `upstream` is the upstream remote, used as the PR target only for
    /// unlisted repos; `changed` lists the paths this work touches, needed only
    /// by entries whose deploy depends on them
    fn resolve(
        &self,
        repo: &str,
        words: &[&str],
        upstream: Option<&str>,
        changed: Option<&[String]>,
    ) -> Result<Plan> {
        let repo = repo.to_lowercase();
        let entry = self
            .repos
            .get(&repo)
            .map(|value| Section::from_toml(&repo, value))
            .transpose()?;

        // a listed repo states its PR target; an upstream remote must not silently
        // redirect it
        let pr_repo = match &entry {
            Some(section) => section
                .pr_repo
                .as_deref()
                .filter(|target| !target.is_empty()),
            None => upstream,
        }
        .unwrap_or(&repo)
        .to_lowercase();

        let owner = pr_repo.split('/').next().unwrap_or_default();
        let external = !self.owners().contains(owner);

        let (source, base) = match entry {
            Some(section) => (Source::Listed, section),
            None if external => (Source::ExternalFallback, self.fallback("external")?),
            None => (Source::DefaultFallback, self.fallback("default")?),
        };

        let mut plan = Plan {
            repo: repo.clone(),
            source,
            external,
            pr_repo: pr_repo.clone(),
            mode: Mode::Run,
            confirm: false,
            steps: Vec::new(),
            implied: Vec::new(),
            sol_effort: Effort::Xhigh,
            bump: Bump::Patch,
            merge_method: match base.merge.as_deref() {
                Some("never") => MergeMethod::Never,
                _ => MergeMethod::Squash,
            },
            pr_draft: base.pr_draft,
            commands: BTreeMap::new(),
            notes: base.notes.clone(),
            host: Host::None,
            skipped: Vec::new(),
            takes_bump: false,
        };

        let mut added = BTreeSet::new();
        let mut removed = BTreeSet::new();
        let mut only = false;
        let mut effort_word = None;
        let mut bump_word = None;

        for &raw in words {
            let word = raw.to_lowercase();

            if word == "?" || word == "status" {
                plan.mode = Mode::Status;
            } else if word == "+confirm" || word == "confirm" {
                // the skill starts on its own unless something is unclear, so this
                // opts into waiting
                plan.confirm = true;
            } else if word == "only" {
                only = true;
            } else if let Some(effort) = Effort::from_word(&word) {
                plan.sol_effort = effort;
                effort_word = Some(word);
            } else if let Some(bump) = Bump::from_word(&word) {
                plan.bump = bump;
                bump_word = Some(word);
            } else if let Some(step) = word.strip_prefix('-') {
                removed.insert(Step::parse(step, raw)?);
            } else {
                let step = word.strip_prefix('+').unwrap_or(&word);
                added.insert(Step::parse(step, raw)?);
            }
        }

        if only && added.is_empty() {
            return Err(PlanError::report("'only' needs at least one step"));
        }

        let mut steps = if only {
            added
        } else {
            let defaults = base.steps.iter().filter_map(|name| Step::from_name(name));
            defaults.chain(added).collect()
        };

        steps.retain(|step| !removed.contains(step));

        // a PR target outside Praveen's orgs always gets a Sol review unless he
        // removes it
        if external && !removed.contains(&Step::Sol) {
            steps.insert(Step::Sol);
        }

        // an effort word is a Sol request, e.g. `/ship high`
        if let Some(effort_word) = effort_word {
            if removed.contains(&Step::Sol) {
                return Err(PlanError::report(format!(
                    "'{effort_word}' sets the Sol effort, but '-sol' removes the review"
                )));
            }

            steps.insert(Step::Sol);
        }

        // where Greptile reviews, merging waits for its 5/5 unless he removes the gate
        let merge_after = if base.greptile && !removed.contains(&Step::Greptile) {
            Step::Greptile
        } else {
            Step::Pr
        };

        let requires = [
            (Step::Pr, Step::Push),
            (Step::Greptile, Step::Pr),
            (Step::Merge, merge_after),
        ];

        // repeat until stable, since an implied step can need another, e.g. merge →
        // greptile → pr → push
        loop {
            // with a PR in the plan, the code to release only reaches the default
            // branch by merging
            let mut needs = requires.to_vec();
            if steps.contains(&Step::Pr) {
                needs.extend([(Step::Deploy, Step::Merge), (Step::Publish, Step::Merge)]);
            }

            let mut missing = false;
            for (step, needed) in needs {
                if !steps.contains(&step) || steps.contains(&needed) {
                    continue;
                }

                if removed.contains(&needed) {
                    return Err(PlanError::report(format!(
                        "'{step}' needs '{needed}', which was removed"
                    )));
                }

                steps.insert(needed);
                plan.implied.push(needed);
                missing = true;
            }

            if !missing {
                break;
            }
        }

        if steps.contains(&Step::Merge) && plan.merge_method == MergeMethod::Never {
            return Err(PlanError::report(format!(
                "{repo} never merges from here; someone else merges {pr_repo} PRs"
            )));
        }

        if steps.contains(&Step::Greptile) && !base.greptile {
            return Err(PlanError::report(format!(
                "Greptile does not review {repo}; set greptile = true in repos.toml once it does"
            )));
        }

        for step in [Step::Deploy, Step::Publish] {
            if !steps.contains(&step) {
                continue;
            }

            let mut commands = base.commands(step);
            if commands.is_empty() {
                return Err(PlanError::report(format!(
                    "{repo} has no {step} commands in repos.toml"
                )));
            }

            if step == Step::Deploy && !base.deploy_paths.is_empty() {
                let Some(changed) = changed else {
                    return Err(PlanError::report(format!(
                        "{repo}'s deploy depends on the changed paths; run from its checkout"
                    )));
                };

                let touched = changed.iter().any(|path| {
                    let mut prefixes = base.deploy_paths.iter();
                    prefixes.any(|prefix| path.starts_with(prefix.as_str()))
                });

                if !touched {
                    commands = &base.deploy_otherwise;
                }
            }

            plan.takes_bump |= commands.iter().any(|command| command.contains(PART));

            let filled = commands
                .iter()
                .map(|command| command.replace(PART, plan.bump.as_str()))
                .collect();

            plan.commands.insert(step, filled);
        }

        if let Some(bump_word) = bump_word {
            let templates = base.deploy.iter().chain(&base.publish);
            if !templates.into_iter().any(|command| command.contains(PART)) {
                return Err(PlanError::report(format!(
                    "'{bump_word}' has no effect: no {repo} command takes a bump level"
                )));
            }
        }

        // the set is ordered by `Step`, which is the canonical order
        plan.steps = steps.into_iter().collect();
        Ok(plan)
    }

    /// Every problem in the config; empty means valid
    fn check(&self) -> Result<Vec<String>> {
        let mut problems = Vec::new();

        if !truthy(self.owners.as_ref()) {
            problems.push("owners must list at least one GitHub org or user".to_owned());
        }

        problems.extend(check_section("[default]", self.default.as_ref()));
        problems.extend(check_section("[external]", self.external.as_ref()));

        for (repo, entry) in self.repos.iter() {
            // remotes are lowercased before lookup, so a mixed-case key would never match
            if !is_repo_key(repo) {
                problems.push(format!("{repo}: key must be a lowercase owner/name"));
            }

            let section_problems = check_section(repo, Some(entry));
            if !section_problems.is_empty() {
                problems.extend(section_problems);
                continue;
            }

            match self.resolve(repo, &[], None, Some(&[])) {
                Ok(_) => {}
                Err(error) => match error.downcast_ref::<PlanError>() {
                    Some(plan_error) => problems.push(format!("{repo}: {plan_error}")),
                    None => return Err(error),
                },
            }
        }

        Ok(problems)
    }
}

/// The problems with one repos.toml table, reported under `name`
fn check_section(name: &str, section: Option<&Toml>) -> Vec<String> {
    let Some(section) = section.and_then(Toml::as_table) else {
        return vec![format!("{name}: must be a table")];
    };

    let mut problems = Vec::new();

    for (key, value) in section {
        match FieldKind::of(key) {
            Some(kind) => {
                if let Some(problem) = kind.problem(value) {
                    problems.push(format!("{name}: {key} {problem}"));
                }
            }
            None => problems.push(format!("{name}: unknown field {key}")),
        }
    }

    let Some(steps) = section.get("steps").and_then(Toml::as_array) else {
        problems.push(format!("{name}: needs a steps list"));
        return problems;
    };

    let names = steps.iter().filter_map(Toml::as_str).collect::<Vec<_>>();
    let unknown = names
        .iter()
        .filter(|name| Step::from_name(name).is_none())
        .collect::<BTreeSet<_>>();

    if !unknown.is_empty() {
        let list = unknown
            .iter()
            .map(|step| python_repr(step))
            .collect::<Vec<_>>()
            .join(", ");
        problems.push(format!("{name}: unknown steps [{list}]"));
    }

    // publishing can't be undone, so it has to be asked for each time
    if names.contains(&"publish") {
        problems.push(format!("{name}: publish may not be a default step"));
    }

    if section
        .get("merge")
        .is_some_and(|merge| !matches!(merge.as_str(), Some("squash" | "never")))
    {
        problems.push(format!("{name}: merge must be squash or never"));
    }

    if truthy(section.get("deploy_paths")) != truthy(section.get("deploy_otherwise")) {
        problems.push(format!(
            "{name}: deploy_paths and deploy_otherwise go together"
        ));
    }

    problems
}

/// Whether a repos.toml key is a lowercase `owner/name`
fn is_repo_key(key: &str) -> bool {
    let part = |part: &str| {
        !part.is_empty()
            && part.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-')
            })
    };

    key.split_once('/')
        .is_some_and(|(owner, name)| part(owner) && part(name))
}

/// Whether a value counts as set, the way Python's `bool` reads it, so an empty
/// list counts as unset
fn truthy(value: Option<&Toml>) -> bool {
    match value {
        None => false,
        Some(Toml::String(text)) => !text.is_empty(),
        Some(Toml::Integer(number)) => *number != 0,
        Some(Toml::Float(number)) => *number != 0.0,
        Some(Toml::Boolean(flag)) => *flag,
        Some(Toml::Array(items)) => !items.is_empty(),
        Some(Toml::Table(table)) => !table.is_empty(),
        Some(Toml::Datetime(_)) => true,
    }
}

/// A string quoted the way Python's `repr` quotes it, which `--check` has
/// always used to list unknown steps
fn python_repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };

    let mut out = String::from(quote);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }

    out.push(quote);
    out
}

/// A git checkout the plan reads remotes, changed paths, and its root from
#[derive(Debug, Clone)]
struct Checkout(PathBuf);

impl Checkout {
    /// git's stdout, or `None` when it fails
    fn git(&self, args: &[&str]) -> Option<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.0)
            .args(args)
            .output()
            .ok()?;

        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// A remote's GitHub `owner/name`, lowercased
    fn remote(&self, name: &str) -> Option<String> {
        parse_github_remote(&self.git(&["remote", "get-url", name])?)
    }

    /// Paths that differ from the remote default branch, committed or not
    fn changed_paths(&self) -> Option<Vec<String>> {
        let base = self.git(&["merge-base", "HEAD", "origin/HEAD"])?;

        // NUL-separated, since git quotes a path with unusual characters on its
        // own line, which would hide it from a deploy_paths prefix
        let tracked = self
            .git(&["diff", "--name-only", "-z", base.trim()])
            .unwrap_or_default();
        let untracked = self
            .git(&["ls-files", "--others", "--exclude-standard", "-z"])
            .unwrap_or_default();

        let paths = tracked
            .split('\0')
            .chain(untracked.split('\0'))
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();

        Some(paths.into_iter().collect())
    }

    /// The run's scratch directory under the checkout root, named by a path-safe slug
    fn task_dir(&self, task: &str) -> Result<TaskDir> {
        let root = self
            .git(&["rev-parse", "--show-toplevel"])
            .ok_or_else(|| PlanError::report("not inside a git checkout"))?;

        let slug = task_slug(task);
        if slug.is_empty() {
            return Err(PlanError::report(format!(
                "task '{task}' has no usable characters"
            )));
        }

        Ok(TaskDir(
            Path::new(root.trim()).join("_scratch/ship").join(slug),
        ))
    }
}

/// A task name as one path component: each run of other characters becomes `-`,
/// since branch names such as feature/x would otherwise nest directories
fn task_slug(task: &str) -> String {
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    let mut slug = String::with_capacity(task.len());
    let mut replacing = false;

    for c in task.chars() {
        if allowed(c) {
            slug.push(c);
            replacing = false;
        } else if !replacing {
            slug.push('-');
            replacing = true;
        }
    }

    slug.trim_matches(['-', '.']).to_owned()
}

/// A GitHub remote URL's `owner/name`, lowercased, or `None` for another host
fn parse_github_remote(url: &str) -> Option<String> {
    let url = url.trim();

    url.match_indices("github.com").find_map(|(at, host)| {
        let rest = &url[at + host.len()..];
        let path = rest.strip_prefix([':', '/'])?;
        let path = path.strip_suffix('/').unwrap_or(path);
        let path = path
            .strip_suffix(".git")
            .filter(|path| !path.ends_with('/'))
            .unwrap_or(path);

        let (owner, name) = path.split_once('/')?;
        let valid = !owner.is_empty() && !name.is_empty() && !name.contains('/');
        valid.then(|| path.to_lowercase())
    })
}

/// `_scratch/ship/<task>/` in a checkout, which holds the saved plan
#[derive(Debug, Clone)]
struct TaskDir(PathBuf);

impl TaskDir {
    fn plan_path(&self) -> PathBuf {
        self.0.join("plan.json")
    }

    /// Writes the plan with an empty `done` list; a new invocation starts a new plan
    fn save(&self, plan: &Plan) -> Result<PathBuf> {
        fs::create_dir_all(&self.0)
            .wrap_err_with(|| format!("could not create {}", self.0.display()))?;

        let path = self.plan_path();
        let saved = SavedPlan { plan, done: [] };
        write_json(&path, &saved)?;
        Ok(path)
    }

    /// Adds finished steps to the saved plan and returns its `done` list in
    /// plan order
    ///
    /// The plan is read as plain JSON and rewritten with its keys in place, so a
    /// plan saved by an older version keeps the fields it has
    fn mark_done(&self, steps: &[String]) -> Result<Vec<String>> {
        let path = self.plan_path();
        if !path.exists() {
            return Err(PlanError::report(format!(
                "no saved plan at {}; run with --task first",
                path.display()
            )));
        }

        let text = fs::read_to_string(&path)
            .wrap_err_with(|| format!("could not read {}", path.display()))?;
        let mut saved = serde_json::from_str::<Ordered<Json>>(&text)
            .wrap_err_with(|| format!("could not parse {}", path.display()))?;

        let planned = strings(saved.get("steps"))
            .ok_or_else(|| eyre!("{} has no steps list", path.display()))?;

        let unknown = steps
            .iter()
            .filter(|step| !planned.contains(step))
            .map(String::as_str)
            .collect::<Vec<_>>();

        if !unknown.is_empty() {
            return Err(PlanError::report(format!(
                "not in the plan: {}",
                unknown.join(", ")
            )));
        }

        let mut done = strings(saved.get("done")).unwrap_or_default();
        done.extend(steps.iter().cloned());

        let done = planned
            .into_iter()
            .filter(|step| done.contains(step))
            .collect::<Vec<_>>();

        saved.insert("done", Json::from(done.clone()));
        write_json(&path, &saved)?;
        Ok(done)
    }
}

/// A JSON list's strings, or `None` when the value is not a list
fn strings(value: Option<&Json>) -> Option<Vec<String>> {
    let items = value?.as_array()?;
    Some(
        items
            .iter()
            .filter_map(Json::as_str)
            .map(str::to_owned)
            .collect(),
    )
}

/// Writes `value` as two-space indented JSON with a trailing newline
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let text = serde_json::to_string_pretty(value)? + "\n";
    fs::write(path, text).wrap_err_with(|| format!("could not write {}", path.display()))
}

/// A table's entries in file order, since the `toml` and `serde_json` maps this
/// crate builds sort their keys
#[derive(Debug, Clone)]
struct Ordered<V>(Vec<(String, V)>);

impl<V> Default for Ordered<V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<V> Ordered<V> {
    fn get(&self, key: &str) -> Option<&V> {
        self.0
            .iter()
            .find_map(|(name, value)| (name == key).then_some(value))
    }

    /// Replaces the value under `key` in place, or appends it
    fn insert(&mut self, key: &str, value: V) {
        match self.0.iter_mut().find(|(name, _)| name == key) {
            Some((_, slot)) => *slot = value,
            None => self.0.push((key.to_owned(), value)),
        }
    }

    fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.0.iter().map(|(name, value)| (name.as_str(), value))
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'de, V: Deserialize<'de>> Deserialize<'de> for Ordered<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct OrderedVisitor<V>(PhantomData<V>);

        impl<'de, V: Deserialize<'de>> Visitor<'de> for OrderedVisitor<V> {
            type Value = Ordered<V>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a table")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    entries.push(entry);
                }

                Ok(Ordered(entries))
            }
        }

        deserializer.deserialize_map(OrderedVisitor(PhantomData))
    }
}

impl<V: Serialize> Serialize for Ordered<V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }

        map.end()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{parse_github_remote, task_slug, Config, Effort, Host, PlanError, Step};
    use crate::cmd::hook::claude::recommend;

    const CONFIG: &str = r#"
owners = ["me"]

[default]
steps = ["commit", "push", "clean"]

[external]
steps = ["sol", "commit", "push", "pr", "clean"]

[repos."me/app"]
steps = ["commit", "push", "deploy", "clean"]
deploy = ["just bump <part>", "just release"]

[repos."me/gated"]
steps = ["commit", "push", "pr", "greptile", "merge", "clean"]
greptile = true

[repos."me/tool"]
steps = ["commit", "push", "deploy", "clean"]
deploy = ["release tool"]
deploy_paths = ["tool/"]
deploy_otherwise = ["pull config"]

[repos."me/fork"]
steps = ["commit", "push", "pr", "clean"]
pr_repo = "them/upstream"
merge = "never"
"#;

    fn config() -> Config {
        Config::parse(CONFIG).unwrap()
    }

    fn steps(names: &str) -> Vec<Step> {
        names
            .split_whitespace()
            .map(|name| Step::from_name(name).unwrap())
            .collect()
    }

    fn plan_error(repo: &str, words: &[&str], changed: Option<&[String]>) -> String {
        let error = config()
            .resolve(repo, words, None, changed)
            .expect_err("expected a plan error");

        match error.downcast_ref::<PlanError>() {
            Some(plan_error) => plan_error.to_string(),
            None => panic!("{repo} {words:?}: not a plan error: {error:#}"),
        }
    }

    #[test]
    fn t3_claude_wait_uses_the_hook_timeout() {
        let mut plan = config().resolve("me/gated", &[], None, None).unwrap();
        plan.apply_host(Host::T3Claude);

        let wait = plan
            .host_actions()
            .into_iter()
            .find(|line| line.starts_with("wait:"))
            .expect("a gated plan waits on Greptile");

        let timeout = format!("timeout {} ", recommend::WAIT_TIMEOUT_MS);
        assert!(wait.contains(&timeout), "{wait}");
    }

    #[test]
    fn words_resolve_to_steps_in_canonical_order() {
        let cases: &[(&str, &[&str], &str)] = &[
            // typed order and dictation aliases never matter
            (
                "me/app",
                &["settle", "soul", "release", "xi"],
                "sol commit push deploy clean settle",
            ),
            ("me/app", &["+s"], "commit push deploy clean settle"),
            ("me/app", &["+set"], "commit push deploy clean settle"),
            ("me/app", &["s"], "commit push deploy clean settle"),
            ("me/app", &["set"], "commit push deploy clean settle"),
            // a merge on a direct-to-master repo opens the PR it merges
            ("me/app", &["merge"], "commit push pr merge deploy clean"),
            // a release after a PR waits for the merge
            ("me/app", &["pr"], "commit push pr merge deploy clean"),
            // only replaces the default but keeps prerequisites, the greptile
            // gate, and the external sol
            ("me/app", &["only", "merge"], "push pr merge"),
            ("me/gated", &["only", "merge"], "push pr greptile merge"),
            ("me/gated", &["merge", "only", "-greptile"], "push pr merge"),
            ("me/fork", &["only", "pr"], "sol push pr"),
            // a PR target outside owners always gets sol
            ("me/fork", &[], "sol commit push pr clean"),
            ("me/unlisted", &[], "commit push clean"),
            // an effort word is a sol request
            ("me/app", &["High"], "sol commit push deploy clean"),
        ];

        for &(repo, words, want) in cases {
            let plan = config()
                .resolve(repo, words, None, None)
                .unwrap_or_else(|error| panic!("{repo} {words:?}: {error:#}"));

            assert_eq!(plan.steps, steps(want), "{repo} {words:?}");
        }
    }

    #[test]
    fn unresolvable_words_and_steps_are_errors() {
        let cases: &[(&str, &[&str], &str)] = &[
            (
                "me/gated",
                &["-push"],
                "'pr' needs 'push', which was removed",
            ),
            ("me/app", &["pr", "-merge"], "'deploy' needs 'merge'"),
            ("me/fork", &["merge"], "never merges"),
            ("me/app", &["greptile"], "Greptile does not review me/app"),
            ("me/gated", &["deploy"], "me/gated has no deploy commands"),
            (
                "me/app",
                &["e2e"],
                "unknown word 'e2e'; steps are: sol, commit",
            ),
            ("me/app", &["E2E"], "unknown word 'E2E'"),
            ("me/app", &["-"], "unknown word '-'"),
            ("me/app", &["+high"], "unknown word '+high'"),
            ("me/gated", &["major"], "'major' has no effect"),
            (
                "me/app",
                &["only", "-deploy"],
                "'only' needs at least one step",
            ),
            ("me/app", &["high", "-sol"], "'high' sets the Sol effort"),
            ("me/tool", &[], "run from its checkout"),
        ];

        for &(repo, words, want) in cases {
            let message = plan_error(repo, words, None);
            assert!(message.contains(want), "{repo} {words:?}: {message}");
        }
    }

    #[test]
    fn plans_carry_the_settings_their_words_and_entry_set() {
        let config = config();

        let default = config.resolve("me/app", &[], None, None).unwrap();
        let confirmed = config.resolve("me/app", &["confirm"], None, None).unwrap();
        assert!(!default.confirm);
        assert!(confirmed.confirm);
        assert_eq!(confirmed.steps, default.steps);

        let aliased = config.resolve("me/app", &["xi"], None, None).unwrap();
        assert_eq!(aliased.sol_effort, Effort::Xhigh);

        let implied = config.resolve("me/app", &["merge"], None, None).unwrap();
        assert_eq!(implied.implied, [Step::Pr]);

        // repo names match regardless of case
        let mixed = config.resolve("Me/App", &[], None, None).unwrap();
        assert_eq!(mixed.source.to_string(), "repos.toml");
        assert!(!mixed.external);

        // an upstream remote does not redirect a listed repo
        let gated = config
            .resolve("me/gated", &[], Some("them/gated"), None)
            .unwrap();
        assert_eq!(gated.pr_repo, "me/gated");
        assert!(!gated.external);

        let listed_fork = config.resolve("me/fork", &[], None, None).unwrap();
        assert_eq!(listed_fork.pr_repo, "them/upstream");

        let unlisted_fork = config
            .resolve("me/unlisted", &[], Some("them/upstream"), None)
            .unwrap();
        assert_eq!(unlisted_fork.source.to_string(), "external fallback");
        assert_eq!(unlisted_fork.steps, steps("sol commit push pr clean"));
    }

    #[test]
    fn deploy_commands_follow_the_bump_and_touched_paths() {
        let config = config();
        let paths = |paths: &[&str]| {
            paths
                .iter()
                .map(|path| path.to_string())
                .collect::<Vec<_>>()
        };

        let minor = config.resolve("me/app", &["minor"], None, None).unwrap();
        assert_eq!(
            minor.commands[&Step::Deploy],
            ["just bump minor", "just release"]
        );
        assert!(minor.takes_bump);

        let touched = paths(&["tool/src/main.rs"]);
        let untouched = paths(&["docs/readme.md"]);
        let touched = config
            .resolve("me/tool", &[], None, Some(&touched))
            .unwrap();
        let untouched = config
            .resolve("me/tool", &[], None, Some(&untouched))
            .unwrap();
        assert_eq!(touched.commands[&Step::Deploy], ["release tool"]);
        assert_eq!(untouched.commands[&Step::Deploy], ["pull config"]);
        assert!(!untouched.takes_bump);
    }

    #[test]
    fn repos_toml_is_valid() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../agents/skills/ship/repos.toml");
        let config = Config::load(&path).unwrap();

        assert_eq!(config.check().unwrap(), Vec::<String>::new());
        assert!(config.repos.len() > 10);
    }

    #[test]
    fn check_reports_each_bad_value() {
        let config = Config::parse(
            r#"
owners = []

[default]
steps = ["commit", "bogus"]
extra = 1

[repos."Me/Mixed"]
steps = ["commit", "publish"]
merge = "rebase"
deploy_paths = ["x/"]

[repos."me/later"]
steps = "commit"

[repos."me/fork"]
steps = ["merge"]
merge = "never"
"#,
        )
        .unwrap();

        assert_eq!(
            config.check().unwrap(),
            [
                "owners must list at least one GitHub org or user",
                "[default]: unknown field extra",
                "[default]: unknown steps ['bogus']",
                "[external]: must be a table",
                "Me/Mixed: key must be a lowercase owner/name",
                "Me/Mixed: publish may not be a default step",
                "Me/Mixed: merge must be squash or never",
                "Me/Mixed: deploy_paths and deploy_otherwise go together",
                "me/later: steps must be a list of strings",
                "me/later: needs a steps list",
                "me/fork: me/fork never merges from here; someone else merges me/fork PRs",
            ]
        );
    }

    #[test]
    fn parses_github_remotes_and_task_slugs() {
        for (url, want) in [
            ("git@github.com:Me/App.git\n", Some("me/app")),
            ("https://github.com/me/app", Some("me/app")),
            ("https://github.com/me/app.git/", Some("me/app")),
            ("ssh://git@github.com/me/app", Some("me/app")),
            ("https://token@github.com/me/app.js", Some("me/app.js")),
            ("https://github.com/me/app/tree", None),
            ("https://gitlab.com/me/app", None),
        ] {
            assert_eq!(parse_github_remote(url).as_deref(), want, "{url}");
        }

        for (task, want) in [
            ("feature/x", "feature-x"),
            ("a//b  c", "a-b-c"),
            ("-.v1.2_x.-", "v1.2_x"),
            ("é/", ""),
        ] {
            assert_eq!(task_slug(task), want, "{task}");
        }
    }
}
