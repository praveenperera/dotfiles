use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::{Parser, Subcommand};
use eyre::{eyre, Result, WrapErr};
use xshell::{cmd, Shell};

#[derive(Debug, Clone, Parser)]
pub struct Skill {
    #[command(subcommand)]
    pub subcommand: SkillCmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SkillCmd {
    /// Link project-local skills into the current Git repo
    Add {
        /// Skill names to link. Opens a searchable multi-select picker when omitted
        skills: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SkillEntry {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LinkPlan {
    Link {
        name: String,
        source: PathBuf,
        target: PathBuf,
    },
    SkipExisting {
        name: String,
        target: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInstallSummary {
    pub linked: Vec<String>,
    pub skipped: Vec<String>,
}

pub fn run(sh: &Shell, args: &[OsString]) -> Result<()> {
    let flags = Skill::parse_from(args);
    run_with_flags(sh, flags)
}

pub fn run_with_flags(sh: &Shell, flags: Skill) -> Result<()> {
    match flags.subcommand {
        SkillCmd::Add { skills } => add_skills(sh, &skills),
    }
}

fn add_skills(sh: &Shell, requested_skills: &[String]) -> Result<()> {
    let project_skills_dir = crate::dotfiles_dir()?.join("agents/project-skills");
    let available_skills = list_project_skills(&project_skills_dir)?;
    let selected_skills = if requested_skills.is_empty() {
        select_skills(sh, &available_skills)?
    } else {
        requested_skills.to_vec()
    };

    if selected_skills.is_empty() {
        println!("No skills selected");
        return Ok(());
    }

    let summary = add_skills_by_name(sh, &selected_skills)?;
    print_skill_summary(&summary);

    Ok(())
}

pub fn add_skills_by_name(sh: &Shell, selected_skills: &[String]) -> Result<SkillInstallSummary> {
    if selected_skills.is_empty() {
        return Ok(SkillInstallSummary {
            linked: Vec::new(),
            skipped: Vec::new(),
        });
    }

    let project_skills_dir = crate::dotfiles_dir()?.join("agents/project-skills");
    let available_skills = list_project_skills(&project_skills_dir)?;
    let git_root = git_root(sh)?;
    let target_skills_dir = git_root.join(".agents/skills");
    let plan = plan_skill_links(&available_skills, selected_skills, &target_skills_dir)?;

    fs::create_dir_all(&target_skills_dir).wrap_err_with(|| {
        format!(
            "failed to create project skills directory: {}",
            target_skills_dir.display()
        )
    })?;

    let mut summary = SkillInstallSummary {
        linked: Vec::new(),
        skipped: Vec::new(),
    };
    for item in plan {
        match item {
            LinkPlan::Link {
                name,
                source,
                target,
            } => {
                create_symlink(&source, &target).wrap_err_with(|| {
                    format!(
                        "failed to link skill {name} from {} to {}",
                        source.display(),
                        target.display()
                    )
                })?;
                summary.linked.push(name);
            }
            LinkPlan::SkipExisting { name, .. } => summary.skipped.push(name),
        }
    }

    Ok(summary)
}

pub fn project_skill_names() -> Result<BTreeSet<String>> {
    let project_skills_dir = crate::dotfiles_dir()?.join("agents/project-skills");
    Ok(list_project_skills(&project_skills_dir)?
        .into_iter()
        .map(|skill| skill.name)
        .collect())
}

fn print_skill_summary(summary: &SkillInstallSummary) {
    if !summary.linked.is_empty() {
        println!("Linked skills: {}", summary.linked.join(", "));
    }
    if !summary.skipped.is_empty() {
        println!("Skipped existing skills: {}", summary.skipped.join(", "));
    }
}

fn git_root(sh: &Shell) -> Result<PathBuf> {
    let output = cmd!(sh, "git rev-parse --show-toplevel").read()?;
    Ok(PathBuf::from(output.trim()))
}

fn list_project_skills(project_skills_dir: &Path) -> Result<Vec<SkillEntry>> {
    let mut skills = Vec::new();

    let entries = match fs::read_dir(project_skills_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(skills),
        Err(err) => {
            return Err(err).wrap_err_with(|| {
                format!(
                    "failed to read project skills directory: {}",
                    project_skills_dir.display()
                )
            })
        }
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.join("SKILL.md").is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        skills.push(SkillEntry {
            name: name.to_string(),
            path,
        });
    }

    skills.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(skills)
}

fn select_skills(sh: &Shell, available_skills: &[SkillEntry]) -> Result<Vec<String>> {
    if available_skills.is_empty() {
        return Err(eyre!(
            "no project skills found in {}",
            crate::dotfiles_dir()?
                .join("agents/project-skills")
                .display()
        ));
    }

    if !crate::util::has_tool(sh, "fzf") {
        return Err(eyre!(
            "fzf is required for interactive skill selection; install fzf or pass skill names directly"
        ));
    }

    let input = available_skills
        .iter()
        .map(|skill| skill.name.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let mut child = Command::new("fzf")
        .args([
            "--multi",
            "--prompt",
            "skill> ",
            "--height=100%",
            "--no-sort",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .wrap_err("failed to start fzf")?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write as _;
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn plan_skill_links(
    available_skills: &[SkillEntry],
    selected_skills: &[String],
    target_skills_dir: &Path,
) -> Result<Vec<LinkPlan>> {
    let mut plan = Vec::new();

    for name in selected_skills {
        let source = available_skills
            .iter()
            .find(|skill| skill.name == *name)
            .ok_or_else(|| eyre!("project skill not found: {name}"))?
            .path
            .clone();
        let target = target_skills_dir.join(name);

        if path_exists(&target)? {
            plan.push(LinkPlan::SkipExisting {
                name: name.clone(),
                target,
            });
        } else {
            plan.push(LinkPlan::Link {
                name: name.clone(),
                source,
                target,
            });
        }
    }

    Ok(plan)
}

fn path_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err.into()),
    }
}

#[cfg(unix)]
fn create_symlink(source: &Path, target: &Path) -> Result<()> {
    std::os::unix::fs::symlink(source, target)?;
    Ok(())
}

#[cfg(windows)]
fn create_symlink(source: &Path, target: &Path) -> Result<()> {
    std::os::windows::fs::symlink_dir(source, target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{list_project_skills, plan_skill_links, LinkPlan};

    #[test]
    fn lists_project_skills_with_skill_files() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("project-skills");
        std::fs::create_dir_all(skills_dir.join("beta")).unwrap();
        std::fs::create_dir_all(skills_dir.join("alpha")).unwrap();
        std::fs::create_dir_all(skills_dir.join("notes")).unwrap();
        std::fs::write(skills_dir.join("beta/SKILL.md"), "beta").unwrap();
        std::fs::write(skills_dir.join("alpha/SKILL.md"), "alpha").unwrap();
        std::fs::write(skills_dir.join("notes/README.md"), "notes").unwrap();

        let skills = list_project_skills(&skills_dir).unwrap();
        let names = skills
            .into_iter()
            .map(|skill| skill.name)
            .collect::<Vec<_>>();

        assert_eq!(names, ["alpha", "beta"]);
    }

    #[test]
    fn plans_links_for_selected_skills() {
        let dir = tempfile::tempdir().unwrap();
        let source_dir = dir.path().join("source");
        let target_dir = dir.path().join("target");
        std::fs::create_dir_all(source_dir.join("alpha")).unwrap();

        let available = vec![super::SkillEntry {
            name: "alpha".to_string(),
            path: source_dir.join("alpha"),
        }];
        let selected = vec!["alpha".to_string()];

        let plan = plan_skill_links(&available, &selected, &target_dir).unwrap();

        assert_eq!(
            plan,
            [LinkPlan::Link {
                name: "alpha".to_string(),
                source: source_dir.join("alpha"),
                target: target_dir.join("alpha"),
            }]
        );
    }

    #[test]
    fn skips_existing_targets() {
        let dir = tempfile::tempdir().unwrap();
        let source_dir = dir.path().join("source");
        let target_dir = dir.path().join("target");
        std::fs::create_dir_all(source_dir.join("alpha")).unwrap();
        std::fs::create_dir_all(target_dir.join("alpha")).unwrap();

        let available = vec![super::SkillEntry {
            name: "alpha".to_string(),
            path: source_dir.join("alpha"),
        }];
        let selected = vec!["alpha".to_string()];

        let plan = plan_skill_links(&available, &selected, &target_dir).unwrap();

        assert_eq!(
            plan,
            [LinkPlan::SkipExisting {
                name: "alpha".to_string(),
                target: target_dir.join("alpha"),
            }]
        );
    }

    #[test]
    fn errors_for_unknown_skill() {
        let dir = tempfile::tempdir().unwrap();
        let err = plan_skill_links(&[], &["missing".to_string()], dir.path()).unwrap_err();

        assert!(err.to_string().contains("project skill not found: missing"));
    }
}
