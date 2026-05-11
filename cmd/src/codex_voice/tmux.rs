use std::path::PathBuf;

use color_eyre::eyre::{eyre, Result, WrapErr};
use xshell::{cmd, Shell};

const FIELD_SEP: char = '\t';

#[derive(Debug, Clone)]
pub struct PaneMetadata {
    pub id: String,
    pub cwd: PathBuf,
    pub current_command: String,
    pub title: String,
}

pub struct Tmux<'a> {
    sh: &'a Shell,
}

impl<'a> Tmux<'a> {
    pub fn new(sh: &'a Shell) -> Self {
        Self { sh }
    }

    pub fn pane_metadata(&self, pane: Option<&str>) -> Result<PaneMetadata> {
        let format = "#{pane_id}\t#{pane_current_path}\t#{pane_current_command}\t#{pane_title}";
        let output = if let Some(pane) = pane {
            cmd!(self.sh, "tmux display-message -p -t {pane} {format}")
                .quiet()
                .read()
        } else {
            cmd!(self.sh, "tmux display-message -p {format}")
                .quiet()
                .read()
        }
        .wrap_err("Failed to read focused tmux pane metadata")?;

        parse_pane_metadata(output.trim_end())
    }
}

pub fn parse_pane_metadata(line: &str) -> Result<PaneMetadata> {
    let mut parts = line.splitn(4, FIELD_SEP);
    let id = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| eyre!("tmux did not return a pane id"))?
        .to_owned();
    let cwd = parts
        .next()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| eyre!("tmux did not return a pane cwd"))?;
    let current_command = parts.next().unwrap_or_default().to_owned();
    let title = parts.next().unwrap_or_default().to_owned();

    Ok(PaneMetadata {
        id,
        cwd,
        current_command,
        title,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_pane_metadata;

    #[test]
    fn parses_tmux_metadata() {
        let pane = parse_pane_metadata("%1\t/tmp/repo\tcodex\tcodex: 019e").unwrap();
        assert_eq!(pane.id, "%1");
        assert_eq!(pane.cwd.to_str(), Some("/tmp/repo"));
        assert_eq!(pane.current_command, "codex");
        assert_eq!(pane.title, "codex: 019e");
    }
}
