use std::io::Write;
use std::process::{Command, Stdio};
use std::time::SystemTime;

use eyre::{eyre, Context, Result};

use super::transcript::{Provider, Session};

/// A session row the picker can show
pub struct Entry {
    pub modified: SystemTime,
    pub session: Session,
    pub in_t3: bool,
}

/// Prompts double as the filter state, so tab can read which one is showing
const ALL_PROMPT: &str = "all> ";
const CLAUDE_PROMPT: &str = "claude> ";
const CODEX_PROMPT: &str = "codex> ";

/// Let the user choose a session with fzf; None when they cancel
pub fn pick(entries: Vec<Entry>, start: Option<Provider>) -> Result<Option<Session>> {
    let mut list = tempfile::NamedTempFile::new()?;
    for entry in &entries {
        writeln!(list, "{}", line(entry))?;
    }

    list.flush()?;

    let path = list.path().display().to_string();
    // fzf runs bindings through a shell, and a temp path never contains quotes
    let cycle = format!(
        "case $FZF_PROMPT in \
           '{ALL_PROMPT}') echo \"change-prompt({CLAUDE_PROMPT})+reload(grep '^claude' '{path}' || true)\" ;; \
           '{CLAUDE_PROMPT}') echo \"change-prompt({CODEX_PROMPT})+reload(grep '^codex' '{path}' || true)\" ;; \
           *) echo \"change-prompt({ALL_PROMPT})+reload(cat '{path}')\" ;; \
         esac"
    );

    let (prompt, start_command) = match start {
        None => (ALL_PROMPT, format!("cat '{path}'")),
        Some(provider) => (
            match provider {
                Provider::Claude => CLAUDE_PROMPT,
                Provider::Codex => CODEX_PROMPT,
            },
            format!("grep '^{}' '{path}' || true", provider.label()),
        ),
    };

    let output = Command::new("fzf")
        .args([
            "--prompt",
            prompt,
            "--height=100%",
            "--no-sort",
            "--layout=reverse",
        ])
        .args(["--delimiter", "\t", "--with-nth", "1,3,4"])
        .args(["--header", "tab: all → claude → codex"])
        .arg(format!("--bind=start:reload({start_command})"))
        .arg(format!("--bind=tab:transform:{cycle}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .wrap_err("failed to start fzf")?
        .wait_with_output()?;

    // fzf exits 130 on escape and 1 on no match
    if !output.status.success() {
        return Ok(None);
    }

    let selected = String::from_utf8_lossy(&output.stdout);
    let id = selected
        .split('\t')
        .nth(1)
        .ok_or_else(|| eyre!("unexpected fzf output: {selected}"))?;

    Ok(entries
        .into_iter()
        .map(|entry| entry.session)
        .find(|session| session.id == id))
}

fn line(entry: &Entry) -> String {
    let session = &entry.session;
    let title = session.title.as_deref().unwrap_or("(untitled)");
    let marker = if entry.in_t3 { "  [in T3]" } else { "" };

    format!(
        "{:<6}\t{}\t{:>4}\t{title}{marker}",
        session.provider.label(),
        session.id,
        age(entry.modified),
    )
}

fn age(modified: SystemTime) -> String {
    let seconds = SystemTime::now()
        .duration_since(modified)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);

    match seconds {
        0..60 => format!("{seconds}s"),
        60..3600 => format!("{}m", seconds / 60),
        3600..86400 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86400),
    }
}
