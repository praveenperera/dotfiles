//! Reads the message a `git commit` call gives on its command line or in a
//! heredoc, so a check can catch a message git would misread before the
//! commit lands

use super::shell::Command;

/// Where a `git commit` call takes its message from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source<'a> {
    /// The first `-m`, which git makes the first paragraph; any later `-m` is
    /// a paragraph of the body
    Flag(&'a str),
    /// `-F -`, read from the heredoc that feeds the command
    Stdin,
    /// `-F <file>`, an editor, or a reused message, none of which the
    /// command's text holds
    Elsewhere,
}

/// The message of a `git commit` call, or `None` for any other command and for
/// a message read from somewhere the command's text does not hold
pub fn message<'a>(command: &Command<'a>) -> Option<&'a str> {
    if command.program() != "git" {
        return None;
    }

    let mut args = command.args();
    if subcommand(&mut args)? != "commit" {
        return None;
    }

    match source(args) {
        Source::Flag(message) => Some(message),
        Source::Stdin => command.heredoc(),
        Source::Elsewhere => None,
    }
}

/// Whether git would show the whole first paragraph as the subject, which
/// happens when the line after the subject is not blank
pub fn joins_subject(message: &str) -> bool {
    // git's default cleanup for `-m` and `-F` drops leading blank lines and
    // trailing whitespace, so a line of only spaces counts as blank
    let mut lines = message.lines().skip_while(|line| line.trim().is_empty());
    lines.next();

    lines.next().is_some_and(|line| !line.trim().is_empty())
}

/// Moves past git's global options, such as `-C <dir>`, and returns the
/// subcommand
fn subcommand<'a>(args: &mut impl Iterator<Item = &'a str>) -> Option<&'a str> {
    while let Some(arg) = args.next() {
        match arg {
            "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace" | "--config-env" => {
                args.next();
            }
            arg if arg.starts_with('-') => {}
            subcommand => return Some(subcommand),
        }
    }

    None
}

/// Reads `git commit`'s options for where the message comes from
fn source<'a>(mut args: impl Iterator<Item = &'a str>) -> Source<'a> {
    let mut flag = None;
    let mut stdin = false;

    while let Some(arg) = args.next() {
        if arg == "--" {
            break;
        }

        let Some((option, attached)) = split_option(arg) else {
            continue;
        };

        let value = match attached {
            Some(value) => Some(value),
            None if takes_value(option) => args.next(),
            None => None,
        };

        match (option, value) {
            ("m" | "message", Some(message)) => {
                flag.get_or_insert(message);
            }
            ("F" | "file", Some(file)) => stdin = file == "-",
            _ => {}
        }
    }

    match (flag, stdin) {
        (Some(message), _) => Source::Flag(message),
        (None, true) => Source::Stdin,
        (None, false) => Source::Elsewhere,
    }
}

/// Short options of `git commit` that take a value, attached or in the next
/// word
const SHORT_WITH_VALUE: &[char] = &['m', 'F', 'C', 'c', 't'];

/// Short options whose value can only be attached, such as `-uno` and
/// `-S<keyid>`
const SHORT_WITH_ATTACHED: &[char] = &['u', 'S'];

/// Splits `--name=value`, `--name`, or the option of a short cluster such as
/// `-qm` or `-mSubject` that takes a value, into the option and any attached
/// value; `None` for a word that is not an option or holds no option with a
/// value
fn split_option(arg: &str) -> Option<(&str, Option<&str>)> {
    if let Some(long) = arg.strip_prefix("--") {
        return Some(match long.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (long, None),
        });
    }

    let cluster = arg
        .strip_prefix('-')
        .filter(|cluster| !cluster.is_empty())?;
    for (at, option) in cluster.char_indices() {
        if SHORT_WITH_ATTACHED.contains(&option) {
            return None;
        }

        if SHORT_WITH_VALUE.contains(&option) {
            let name = &cluster[at..at + 1];
            let rest = &cluster[at + 1..];
            return Some((name, (!rest.is_empty()).then_some(rest)));
        }
    }

    None
}

/// Whether an option with no attached value reads the next word as its value
fn takes_value(option: &str) -> bool {
    matches!(
        option,
        "m" | "F"
            | "C"
            | "c"
            | "t"
            | "message"
            | "file"
            | "reuse-message"
            | "reedit-message"
            | "template"
            | "author"
            | "date"
            | "cleanup"
            | "trailer"
    )
}

#[cfg(test)]
mod tests {
    use super::super::shell::Script;
    use super::{joins_subject, message};

    fn messages(source: &str) -> Vec<String> {
        Script::parse(source)
            .invocations()
            .filter_map(|command| message(&command))
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn reads_commit_messages() {
        let cases: &[(&str, &[&str])] = &[
            ("git commit -m 'Subject'", &["Subject"]),
            ("git commit -qm 'Subject'", &["Subject"]),
            ("git commit -am Subject", &["Subject"]),
            ("git commit -mSubject", &["Subject"]),
            ("git commit --message=Subject", &["Subject"]),
            ("git commit --message Subject", &["Subject"]),
            ("git commit -m First -m Second", &["First"]),
            ("git -C repo -c a=b commit -m x", &["x"]),
            ("git --no-pager commit -m x", &["x"]),
            (
                "git commit -q -F - <<'EOF'\nSubject\n\nBody\nEOF",
                &["Subject\n\nBody\n"],
            ),
            ("git commit -F- <<EOF\nx\nEOF", &["x\n"]),
            ("git commit --file=- <<EOF\nx\nEOF", &["x\n"]),
            (
                "cd r && git commit -F - <<EOF && git push\nx\nEOF",
                &["x\n"],
            ),
            ("bash -c \"git commit -m 'x'\"", &["x"]),
            // nothing in the command's text to read
            ("git commit -F msg.txt", &[]),
            ("cat <<EOF | git commit -F -\nx\nEOF", &[]),
            ("git commit --amend --no-edit", &[]),
            ("git commit -C HEAD", &[]),
            ("git commit -Smykey", &[]),
            ("git commit -- -m", &[]),
            ("git log -m", &[]),
            ("echo git commit -m x", &[]),
            ("git commit-tree -m x HEAD^{tree}", &[]),
        ];

        for (source, want) in cases {
            assert_eq!(messages(source), *want, "{source:?}");
        }
    }

    #[test]
    fn finds_a_subject_run_into_its_body() {
        let cases = [
            ("Subject", false),
            ("Subject\n", false),
            ("Subject\n\nBody\nmore", false),
            ("Subject\n   \nBody", false),
            ("\n\nSubject\n\nBody", false),
            ("Subject\nBody", true),
            ("Subject\nBody\n\nMore", true),
            ("\nSubject\nBody", true),
            ("Subject\r\nBody", true),
        ];

        for (message, want) in cases {
            assert_eq!(joins_subject(message), want, "{message:?}");
        }
    }
}
