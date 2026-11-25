//! `cmd git commit-paths` runs in checkouts where other threads keep
//! uncommitted work, so these pin that it commits and pushes only the named
//! paths and never rewrites anyone else's files

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::{tempdir, TempDir};

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned()
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

fn commit_paths(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cmd"))
        .args(["git", "commit-paths"])
        .args(args)
        .env("NO_COLOR", "1")
        .current_dir(cwd)
        .output()
        .unwrap()
}

/// A bare origin, the shared checkout under test, and another checkout that
/// pushes upstream changes, all starting from one pushed commit
struct Fixture {
    _root: TempDir,
    origin: PathBuf,
    shared: PathBuf,
    other: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempdir().unwrap();
        let origin = root.path().join("origin.git");
        let shared = root.path().join("shared");
        let other = root.path().join("other");

        git(
            root.path(),
            &["init", "-q", "--bare", "-b", "master", "origin.git"],
        );
        git(root.path(), &["clone", "-q", "origin.git", "shared"]);

        write(&shared.join(".gitignore"), "_scratch/\n");
        write(&shared.join("a.md"), "a\n");
        write(&shared.join("shared.md"), "base\n");
        configure(&shared);
        git(&shared, &["add", "."]);
        git(&shared, &["commit", "-q", "-m", "init"]);
        git(&shared, &["push", "-q", "origin", "master"]);

        git(root.path(), &["clone", "-q", "origin.git", "other"]);
        configure(&other);

        Self {
            _root: root,
            origin,
            shared,
            other,
        }
    }

    /// Commits and pushes files from the other checkout
    fn push_upstream(&self, files: &[(&str, &str)], subject: &str) {
        for (name, contents) in files {
            write(&self.other.join(name), contents);
        }

        git(&self.other, &["add", "."]);
        git(&self.other, &["commit", "-q", "-m", subject]);
        git(&self.other, &["push", "-q", "origin", "master"]);
    }

    fn origin_subjects(&self) -> Vec<String> {
        git(&self.origin, &["log", "--format=%s", "master"])
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

fn configure(checkout: &Path) {
    git(checkout, &["config", "user.name", "t"]);
    git(checkout, &["config", "user.email", "t@t"]);
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn pushes_only_the_named_paths_and_keeps_other_work() {
    let fx = Fixture::new();
    fx.push_upstream(
        &[("new.md", "same\n"), ("shared.md", "theirs\n")],
        "Add new",
    );

    // other threads' work in the shared tree: a tracked edit, an untracked
    // file, and an untracked copy of what upstream just added
    write(&fx.shared.join("a.md"), "someone's edit\n");
    write(&fx.shared.join("wip.md"), "draft\n");
    write(&fx.shared.join("new.md"), "same\n");
    write(&fx.shared.join("notes/mine.md"), "mine\n");

    let output = commit_paths(&fx.shared, &["notes/mine.md", "-m", "Add my note"]);
    assert!(output.status.success(), "{}", stderr(&output));

    assert_eq!(fx.origin_subjects(), ["Add my note", "Add new", "init"]);
    assert_eq!(
        git(&fx.shared, &["rev-parse", "HEAD"]),
        git(&fx.origin, &["rev-parse", "master"])
    );
    assert_eq!(
        git(&fx.shared, &["show", "--name-only", "--format=", "HEAD"]),
        "notes/mine.md"
    );
    assert_eq!(
        git(&fx.shared, &["log", "-1", "--format=%an"]),
        "Praveen Perera"
    );

    assert_eq!(read(&fx.shared.join("a.md")), "someone's edit\n");
    assert_eq!(read(&fx.shared.join("wip.md")), "draft\n");
    assert_eq!(read(&fx.shared.join("shared.md")), "theirs\n");
    assert_eq!(read(&fx.shared.join("new.md")), "same\n");
    assert_eq!(
        git(&fx.shared, &["status", "--porcelain"]),
        " M a.md\n?? wip.md"
    );

    let moved = fs::read_dir(fx.shared.join("_scratch/commit-paths-moved"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(read(&moved.join("new.md")), "same\n");
}

#[test]
fn refuses_before_committing_when_upstream_changes_a_dirty_file() {
    let fx = Fixture::new();
    fx.push_upstream(&[("a.md", "upstream\n")], "Edit a");

    write(&fx.shared.join("a.md"), "local\n");
    write(&fx.shared.join("note.md"), "note\n");
    let head = git(&fx.shared, &["rev-parse", "HEAD"]);

    let output = commit_paths(&fx.shared, &["note.md", "-m", "Add note"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("a.md"), "{}", stderr(&output));

    assert_eq!(git(&fx.shared, &["rev-parse", "HEAD"]), head);
    assert_eq!(read(&fx.shared.join("a.md")), "local\n");
    assert_eq!(
        git(&fx.shared, &["status", "--porcelain"]),
        " M a.md\n?? note.md"
    );
}

#[test]
fn rebase_conflict_stays_out_of_the_shared_checkout() {
    let fx = Fixture::new();
    fx.push_upstream(&[("shared.md", "theirs\n")], "Edit shared upstream");

    write(&fx.shared.join("shared.md"), "ours\n");
    write(&fx.shared.join("wip.md"), "draft\n");

    let output = commit_paths(&fx.shared, &["shared.md", "-m", "Edit shared here"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("NOT pushed"),
        "{}",
        stderr(&output)
    );

    // committed locally, no rebase left in progress, temp worktree gone
    assert_eq!(
        git(&fx.shared, &["log", "-1", "--format=%s"]),
        "Edit shared here"
    );
    assert_eq!(git(&fx.shared, &["status", "--porcelain"]), "?? wip.md");
    assert_eq!(git(&fx.shared, &["worktree", "list"]).lines().count(), 1);
    assert_eq!(fx.origin_subjects(), ["Edit shared upstream", "init"]);
}

// INDEX.md is generated from the tree, so both sides touching it is routine
// and must not need a person
#[test]
fn regenerates_a_conflicting_index() {
    let fx = Fixture::new();
    let script = "import subprocess\n\
        out = subprocess.run(['git', 'ls-files'], capture_output=True, text=True).stdout\n\
        names = sorted({n for n in out.split() if n.endswith('.md') and n != 'INDEX.md'})\n\
        open('INDEX.md', 'w').write(''.join(n + '\\n' for n in names))\n";

    fx.push_upstream(
        &[
            (".github/scripts/index.py", script),
            ("INDEX.md", "a.md\nshared.md\n"),
        ],
        "Add the index",
    );
    git(&fx.shared, &["pull", "-q"]);

    fx.push_upstream(
        &[
            ("theirs.md", "t\n"),
            ("INDEX.md", "a.md\nshared.md\ntheirs.md\n"),
        ],
        "Add theirs",
    );
    write(&fx.shared.join("mine.md"), "m\n");
    write(&fx.shared.join("INDEX.md"), "a.md\nmine.md\nshared.md\n");

    let output = commit_paths(&fx.shared, &["mine.md", "INDEX.md", "-m", "Add mine"]);
    assert!(output.status.success(), "{}", stderr(&output));

    assert_eq!(
        fx.origin_subjects(),
        ["Add mine", "Add theirs", "Add the index", "init"]
    );
    assert_eq!(
        read(&fx.shared.join("INDEX.md")),
        "a.md\nmine.md\nshared.md\ntheirs.md\n"
    );
    assert_eq!(git(&fx.shared, &["status", "--porcelain"]), "");
}
