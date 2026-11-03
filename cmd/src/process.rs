//! Walks a process's ancestors through `ps`, one pid at a time
//!
//! A full `ps -A` snapshot costs tens of milliseconds on a busy Mac, while one
//! `ps -p` lookup costs about two, and the walks here stop after a few steps

use std::path::Path;
use std::process::Command;

/// One process in an ancestor walk
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub ppid: u32,
    /// The executable as `ps -o comm=` prints it, a full path on macOS
    pub comm: String,
}

impl Process {
    /// Looks up one process, or `None` when it is gone or `ps` fails
    pub fn lookup(pid: u32) -> Option<Self> {
        // comm goes last so `ps` prints it whole instead of cutting it to 16 bytes
        let output = Command::new("ps")
            .args(["-o", "ppid=,comm=", "-p", &pid.to_string()])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        Self::parse(pid, &String::from_utf8_lossy(&output.stdout))
    }

    /// Parses one `ps -o ppid=,comm=` line for `pid`
    pub fn parse(pid: u32, line: &str) -> Option<Self> {
        let (ppid, comm) = line.trim().split_once(char::is_whitespace)?;

        Some(Self {
            pid,
            ppid: ppid.parse().ok()?,
            comm: comm.trim().to_owned(),
        })
    }

    /// The file name of the executable, such as `claude` for `/usr/local/bin/claude`
    pub fn name(&self) -> &str {
        Path::new(&self.comm)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.comm)
    }
}

/// Iterates over `pid` and then each of its ancestors, stopping before pid 1 or
/// at the first process `ps` cannot read
pub fn ancestors(pid: u32) -> impl Iterator<Item = Process> {
    ancestors_with(pid, Process::lookup)
}

/// [`ancestors`] over any lookup, so a walk can run against a fixed table
pub fn ancestors_with(
    pid: u32,
    mut lookup: impl FnMut(u32) -> Option<Process>,
) -> impl Iterator<Item = Process> {
    let mut next = Some(pid);

    std::iter::from_fn(move || {
        let pid = next.take().filter(|&pid| pid > 1)?;
        let process = lookup(pid)?;
        next = Some(process.ppid);
        Some(process)
    })
}

/// The full command line of `pid`, as `ps -o command=` prints it
pub fn command_line(pid: u32) -> Option<String> {
    let output = Command::new("ps")
        .args(["-o", "command=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The agent CLI that runs a T3 Code thread
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum T3Agent {
    Claude,
    Codex,
}

impl T3Agent {
    /// The agent an executable name runs, such as `codex` for Codex
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }
}

/// Whether the Claude session running this process belongs to a T3 Code thread
///
/// Environment variables are not enough: `T3CODE_HOME` and the sdk-ts entrypoint
/// leak into a `claude -p` that an agent starts from a T3 thread, and that nested
/// session has no t3-code tools. T3 starts its sessions directly under `t3 serve`,
/// so this checks the parent of the nearest `claude` process instead
pub fn in_t3_session() -> bool {
    in_t3_session_with(std::process::id(), Process::lookup, command_line)
}

/// [`in_t3_session`] over any process lookup, starting from `pid`
pub fn in_t3_session_with(
    pid: u32,
    lookup: impl FnMut(u32) -> Option<Process>,
    command_line: impl FnOnce(u32) -> Option<String>,
) -> bool {
    t3_child(pid, lookup, command_line, |process| {
        process.name() == "claude"
    })
    .is_some()
}

/// The agent of the T3 Code thread that runs this process, or `None` outside one
///
/// Unlike [`in_t3_session`], the nearest Claude or Codex session decides, so a
/// command a T3 Codex thread runs reports Codex, and one run by a `claude -p` that
/// such a thread started reports nothing
pub fn t3_agent() -> Option<T3Agent> {
    t3_agent_with(std::process::id(), Process::lookup, command_line)
}

/// [`t3_agent`] over any process lookup, starting from `pid`
pub fn t3_agent_with(
    pid: u32,
    lookup: impl FnMut(u32) -> Option<Process>,
    command_line: impl FnOnce(u32) -> Option<String>,
) -> Option<T3Agent> {
    let agent = |process: &Process| T3Agent::from_name(process.name());

    t3_child(pid, lookup, command_line, |process| {
        agent(process).is_some()
    })
    .as_ref()
    .and_then(agent)
}

/// The nearest ancestor that `is_agent` accepts, when `t3 serve` started it
fn t3_child(
    pid: u32,
    lookup: impl FnMut(u32) -> Option<Process>,
    command_line: impl FnOnce(u32) -> Option<String>,
    is_agent: impl Fn(&Process) -> bool,
) -> Option<Process> {
    let agent = ancestors_with(pid, lookup).find(|process| is_agent(process))?;
    let parent = command_line(agent.ppid)?;

    is_t3_serve(&parent).then_some(agent)
}

/// Whether a command line runs `t3 serve`, by bare name or by path
fn is_t3_serve(command_line: &str) -> bool {
    command_line.match_indices("t3 serve").any(|(at, found)| {
        let before = command_line[..at].chars().next_back();
        let after = command_line[at + found.len()..].chars().next();

        matches!(before, None | Some('/')) && matches!(after, None | Some(' '))
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{ancestors_with, in_t3_session_with, is_t3_serve, t3_agent_with, Process, T3Agent};

    fn table(rows: &[(u32, u32, &str)]) -> HashMap<u32, Process> {
        rows.iter()
            .map(|&(pid, ppid, comm)| {
                let process = Process {
                    pid,
                    ppid,
                    comm: comm.to_owned(),
                };
                (pid, process)
            })
            .collect()
    }

    #[test]
    fn parses_ps_lines() {
        let process = Process::parse(20959, "  17747 /Users/me/.local/bin/claude\n").unwrap();
        assert_eq!(process.ppid, 17747);
        assert_eq!(process.comm, "/Users/me/.local/bin/claude");
        assert_eq!(process.name(), "claude");

        let spaced = Process::parse(5, "1 /Applications/Google Chrome.app/Chrome").unwrap();
        assert_eq!(spaced.comm, "/Applications/Google Chrome.app/Chrome");

        assert_eq!(Process::parse(5, ""), None);
        assert_eq!(Process::parse(5, "abc claude"), None);
    }

    #[test]
    fn walks_ancestors_until_pid_one() {
        let processes = table(&[(30, 20, "cmd"), (20, 10, "bash"), (10, 1, "launchd")]);
        let pids = ancestors_with(30, |pid| processes.get(&pid).cloned())
            .map(|process| process.pid)
            .collect::<Vec<_>>();

        assert_eq!(pids, [30, 20, 10]);

        let broken = table(&[(30, 20, "cmd")]);
        let pids = ancestors_with(30, |pid| broken.get(&pid).cloned())
            .map(|process| process.pid)
            .collect::<Vec<_>>();

        assert_eq!(pids, [30]);
    }

    #[test]
    fn t3_session_needs_t3_serve_as_claude_parent() {
        let parents = HashMap::from([
            (
                100,
                "/Users/me/.t3/runtime/versions/0.1.6/t3 serve".to_owned(),
            ),
            (200, "zsh -l".to_owned()),
            (300, "t3 serve --port 1".to_owned()),
        ]);

        let cases = [
            (
                "direct t3 child",
                vec![(9, 8, "cmd"), (8, 100, "claude")],
                true,
            ),
            (
                "bare t3 name",
                vec![(9, 8, "cmd"), (8, 300, "claude")],
                true,
            ),
            (
                "hook under a shell",
                vec![(9, 7, "cmd"), (7, 8, "sh"), (8, 100, "/opt/bin/claude")],
                true,
            ),
            (
                "claude under a shell",
                vec![(9, 8, "cmd"), (8, 200, "claude")],
                false,
            ),
            // a `claude -p` started from a T3 thread runs under that thread's shell
            (
                "nested claude",
                vec![
                    (9, 8, "cmd"),
                    (8, 7, "claude"),
                    (7, 6, "bash"),
                    (6, 100, "claude"),
                ],
                false,
            ),
            ("no claude", vec![(9, 8, "cmd"), (8, 100, "node")], false),
        ];

        for (name, rows, want) in cases {
            let processes = table(&rows);
            let got = in_t3_session_with(
                9,
                |pid| processes.get(&pid).cloned(),
                |pid| parents.get(&pid).cloned(),
            );

            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn t3_agent_is_the_nearest_claude_or_codex_under_t3_serve() {
        let parents = HashMap::from([
            (100, "/Users/me/.t3/runtime/t3 serve".to_owned()),
            (200, "zsh -l".to_owned()),
        ]);

        let cases = [
            (
                "claude thread",
                vec![(9, 8, "cmd"), (8, 100, "claude")],
                Some(T3Agent::Claude),
            ),
            (
                "codex thread through a shell",
                vec![(9, 7, "cmd"), (7, 8, "zsh"), (8, 100, "/opt/bin/codex")],
                Some(T3Agent::Codex),
            ),
            // the nearest agent decides, so a claude a codex thread started is not T3
            (
                "claude under a codex thread",
                vec![
                    (9, 8, "cmd"),
                    (8, 7, "claude"),
                    (7, 6, "zsh"),
                    (6, 100, "codex"),
                ],
                None,
            ),
            (
                "codex under a shell",
                vec![(9, 8, "cmd"), (8, 200, "codex")],
                None,
            ),
            ("no agent", vec![(9, 8, "cmd"), (8, 100, "node")], None),
        ];

        for (name, rows, want) in cases {
            let processes = table(&rows);
            let got = t3_agent_with(
                9,
                |pid| processes.get(&pid).cloned(),
                |pid| parents.get(&pid).cloned(),
            );

            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn matches_t3_serve_command_lines() {
        assert!(is_t3_serve("t3 serve"));
        assert!(is_t3_serve("/a/b/t3 serve --x"));
        assert!(!is_t3_serve("/a/b/t3 server"));
        assert!(!is_t3_serve("xt3 serve"));
        assert!(!is_t3_serve("/a/t3 __service-launcher"));
    }
}
