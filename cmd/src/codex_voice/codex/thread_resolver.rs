use color_eyre::eyre::{eyre, Result};
use xshell::Shell;

use super::{
    app_server::ThreadStore,
    types::{CodexContext, ResolutionSource},
};
use crate::codex_voice::tmux::Tmux;

pub struct ThreadResolver<'a> {
    sh: &'a Shell,
    store: ThreadStore,
}

impl<'a> ThreadResolver<'a> {
    pub fn new(sh: &'a Shell) -> Result<Self> {
        let store = ThreadStore::from_default_home()?;
        Ok(Self { sh, store })
    }

    pub fn resolve(
        &self,
        pane: Option<&str>,
        explicit_thread: Option<&str>,
    ) -> Result<CodexContext> {
        if let Some(thread_id) = explicit_thread {
            let thread = self
                .store
                .find_by_id(thread_id, ResolutionSource::ExplicitThread)?;
            return Ok(CodexContext { pane: None, thread });
        }

        let pane = Tmux::new(self.sh).pane_metadata(pane)?;
        if let Some(thread_id) = parse_thread_id_from_title(&pane.title) {
            let thread = self
                .store
                .find_by_id(&thread_id, ResolutionSource::TerminalTitle)?;
            return Ok(CodexContext {
                pane: Some(pane),
                thread,
            });
        }

        let Some(thread) = self.store.latest_for_cwd(&pane.cwd)?.into_iter().next() else {
            return Err(eyre!(
                "No interactive Codex threads found for cwd {}",
                pane.cwd.display()
            ));
        };

        Ok(CodexContext {
            pane: Some(pane),
            thread,
        })
    }
}

pub fn parse_thread_id_from_title(title: &str) -> Option<String> {
    title
        .split(|ch: char| !(ch.is_ascii_hexdigit() || ch == '-'))
        .find(|part| looks_like_thread_id(part))
        .map(str::to_owned)
}

fn looks_like_thread_id(value: &str) -> bool {
    let mut groups = value.split('-');
    matches!(
        (
            groups.next(),
            groups.next(),
            groups.next(),
            groups.next(),
            groups.next(),
            groups.next()
        ),
        (Some(a), Some(b), Some(c), Some(d), Some(e), None)
            if a.len() == 8 && b.len() == 4 && c.len() == 4 && d.len() == 4 && e.len() == 12
    )
}

#[cfg(test)]
mod tests {
    use super::parse_thread_id_from_title;

    #[test]
    fn parses_thread_id_from_terminal_title() {
        let title = "codex 019e0804-c5b5-7561-a59d-b732efdf0001";
        assert_eq!(
            parse_thread_id_from_title(title).as_deref(),
            Some("019e0804-c5b5-7561-a59d-b732efdf0001")
        );
    }
}
