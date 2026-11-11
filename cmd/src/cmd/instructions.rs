//! Renders the global agent instruction files from the shared Markdown fragments in
//! `agents/instructions`, and checks the checked-in files for drift
//!
//! The fragments are read at runtime, never embedded, so an old compiled binary
//! cannot write stale policy text back over newer fragments

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use colored::Colorize;
use eyre::{bail, eyre, Result, WrapErr};

/// Directory, relative to the dotfiles root, that holds the fragments
pub const FRAGMENTS_DIR: &str = "agents/instructions";

const COMMON_FRAGMENT: &str = "common.md";

const GENERATED_NOTE: &str = "<!-- Generated from agents/instructions by `cmd instructions render`; edit those fragments, not this file -->";

/// One generated instruction file and the client fragment layered over `common.md`
struct Target {
    fragment: &'static str,
    output: &'static str,
}

const TARGETS: &[Target] = &[
    Target {
        fragment: "claude.md",
        output: "claude/CLAUDE.md",
    },
    Target {
        fragment: "codex.md",
        output: "agents/AGENTS.md",
    },
];

#[derive(Debug, Clone, Subcommand)]
pub enum InstructionsCmd {
    /// Write claude/CLAUDE.md and agents/AGENTS.md from the fragments
    Render(RootArgs),

    /// Fail when a generated file differs from a fresh render
    Check(RootArgs),
}

#[derive(Debug, Clone, Args)]
pub struct RootArgs {
    /// Dotfiles checkout to use; defaults to the nearest ancestor of the current
    /// directory that has agents/instructions, then ~/code/dotfiles
    #[arg(long, value_name = "DIR")]
    pub root: Option<PathBuf>,
}

impl RootArgs {
    fn instructions(&self) -> Result<Instructions> {
        if let Some(root) = &self.root {
            return Ok(Instructions::new(root.clone()));
        }

        let cwd = std::env::current_dir()?;
        // match on the directory, not a fragment inside it, so a checkout with a
        // missing fragment reports it instead of falling back to another checkout
        let found = cwd.ancestors().find(|dir| dir.join(FRAGMENTS_DIR).is_dir());

        match found {
            Some(root) => Ok(Instructions::new(root.to_path_buf())),
            None => Ok(Instructions::new(crate::dotfiles_dir()?)),
        }
    }
}

pub fn run(cmd: InstructionsCmd) -> Result<()> {
    match cmd {
        InstructionsCmd::Render(args) => args.instructions()?.render(),
        InstructionsCmd::Check(args) => args.instructions()?.check(),
    }
}

/// A dotfiles checkout whose generated instruction files come from its fragments
pub struct Instructions(PathBuf);

impl Instructions {
    pub fn new(root: PathBuf) -> Self {
        Self(root)
    }

    /// Writes every generated file whose content differs from a fresh render
    pub fn render(&self) -> Result<()> {
        for (target, rendered) in self.rendered()? {
            let path = self.0.join(target.output);
            if read_optional(&path)?.as_deref() == Some(rendered.as_str()) {
                println!("{} {}", "unchanged".dimmed(), target.output);
                continue;
            }

            fs::write(&path, rendered)
                .wrap_err_with(|| format!("failed to write {}", path.display()))?;
            println!("{} {}", "wrote".green(), target.output);
        }

        Ok(())
    }

    /// Fails, naming each drifted file, when a generated file differs from a fresh render
    pub fn check(&self) -> Result<()> {
        let mut drifted = Vec::new();
        for (target, rendered) in self.rendered()? {
            let path = self.0.join(target.output);
            if read_optional(&path)?.as_deref() != Some(rendered.as_str()) {
                drifted.push(target.output);
            }
        }

        if !drifted.is_empty() {
            bail!(
                "{} out of date with {FRAGMENTS_DIR} in {}; edit the fragments, then run `cmd instructions render`",
                drifted.join(" and "),
                self.0.display(),
            );
        }

        println!("{}", "agent instructions match their fragments".green());
        Ok(())
    }

    fn rendered(&self) -> Result<Vec<(&'static Target, String)>> {
        let common = self.read_fragment(COMMON_FRAGMENT)?;
        let common = Sections::parse(&common)
            .wrap_err_with(|| format!("invalid {FRAGMENTS_DIR}/{COMMON_FRAGMENT}"))?;

        TARGETS
            .iter()
            .map(|target| {
                let client = self.read_fragment(target.fragment)?;
                let client = Sections::parse(&client)
                    .wrap_err_with(|| format!("invalid {FRAGMENTS_DIR}/{}", target.fragment))?;

                Ok((target, render(&common, &client)))
            })
            .collect()
    }

    fn read_fragment(&self, name: &str) -> Result<String> {
        let path = self.0.join(FRAGMENTS_DIR).join(name);
        fs::read_to_string(&path).wrap_err_with(|| format!("failed to read {}", path.display()))
    }
}

/// Lays the client fragment over the common one: a client section with the same
/// heading extends that common section's list, and other client sections follow
/// the common ones in fragment order
fn render(common: &Sections, client: &Sections) -> String {
    let mut sections = common.0.clone();
    for (heading, body) in &client.0 {
        match sections
            .iter_mut()
            .find(|(existing, _)| existing == heading)
        {
            Some((_, existing)) => {
                existing.push('\n');
                existing.push_str(body);
            }
            None => sections.push((heading.clone(), body.clone())),
        }
    }

    let body = sections
        .iter()
        .map(|(heading, body)| format!("{heading}\n\n{body}\n"))
        .collect::<Vec<_>>()
        .join("\n");

    format!("{GENERATED_NOTE}\n\n{body}")
}

/// Top-level `# ` sections of a fragment, as (heading line, trimmed body) in order
struct Sections(Vec<(String, String)>);

impl Sections {
    fn parse(text: &str) -> Result<Self> {
        let mut sections: Vec<(String, Vec<&str>)> = Vec::new();
        for line in text.lines() {
            if line.starts_with("# ") {
                if sections.iter().any(|(heading, _)| heading == line) {
                    bail!("heading {line:?} appears twice");
                }

                sections.push((line.to_string(), Vec::new()));
                continue;
            }

            match sections.last_mut() {
                Some((_, body)) => body.push(line),
                // text before the first heading would have no section to merge into
                None if line.trim().is_empty() => {}
                None => bail!("text before the first `# ` heading: {line:?}"),
            }
        }

        sections
            .into_iter()
            .map(|(heading, body)| {
                let body = body.join("\n").trim().to_string();
                if body.is_empty() {
                    return Err(eyre!("section {heading:?} is empty"));
                }

                Ok((heading, body))
            })
            .collect::<Result<_>>()
            .map(Self)
    }
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).wrap_err_with(|| format!("failed to read {}", path.display())),
    }
}
