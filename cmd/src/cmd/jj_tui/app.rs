use super::tree::TreeState;
use super::ui;
use crate::jj_lib_helpers::JjRepo;
use eyre::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::style::Color;
use ratatui::DefaultTerminal;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use xshell::{cmd, Shell};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    FileHeader,
    Hunk,
    Added,
    Removed,
    Context,
}

#[derive(Clone)]
pub struct StyledSpan {
    pub text: String,
    pub fg: Color,
}

pub struct DiffLine {
    pub spans: Vec<StyledSpan>,
    pub kind: DiffLineKind,
}

pub struct DiffState {
    pub lines: Vec<DiffLine>,
    pub scroll_offset: usize,
    pub rev: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Help,
    ViewingDiff,
    Editing,
    Confirming,
    Selecting,
    Rebasing,
    MovingBookmark,
    BookmarkInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebaseType {
    Single,          // -r: just this revision
    WithDescendants, // -s: revision + all descendants
}

impl std::fmt::Display for RebaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RebaseType::Single => write!(f, "-r"),
            RebaseType::WithDescendants => write!(f, "-s"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    #[allow(dead_code)]
    Info,
    Success,
    Warning,
    Error,
}

pub struct StatusMessage {
    pub text: String,
    pub kind: MessageKind,
    pub expires: Instant,
}

pub struct EditingState {
    pub text: String,
    pub cursor: usize,
    pub target_rev: String,
    pub original_desc: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    Abandon,
}

pub struct ConfirmState {
    pub action: ConfirmAction,
    pub message: String,
    pub revs: Vec<String>,
}

#[derive(Clone)]
pub struct RebaseState {
    pub source_rev: String,
    pub rebase_type: RebaseType,
    pub dest_cursor: usize,
    pub allow_branches: bool,
    pub op_before: String,
}

pub struct DiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Clone)]
pub struct MovingBookmarkState {
    pub bookmark_name: String,
    pub dest_cursor: usize,
    pub op_before: String,
}

pub struct BookmarkInputState {
    pub name: String,
    pub cursor: usize,
    pub target_rev: String,
    pub deleting: bool,
}

pub struct App {
    pub tree: TreeState,
    pub mode: Mode,
    pub should_quit: bool,
    pub split_view: bool,
    pub diff_state: Option<DiffState>,
    pub diff_stats_cache: std::collections::HashMap<String, DiffStats>,
    pub status_message: Option<StatusMessage>,
    pub editing_state: Option<EditingState>,
    pub confirm_state: Option<ConfirmState>,
    pub rebase_state: Option<RebaseState>,
    pub moving_bookmark_state: Option<MovingBookmarkState>,
    pub bookmark_input_state: Option<BookmarkInputState>,
    pub last_op: Option<String>,
    sh: Shell,
}

impl App {
    pub fn new(sh: &Shell) -> Result<Self> {
        let jj_repo = JjRepo::load(None)?;
        let tree = TreeState::load(&jj_repo)?;

        Ok(Self {
            tree,
            mode: Mode::Normal,
            should_quit: false,
            split_view: false,
            diff_state: None,
            diff_stats_cache: std::collections::HashMap::new(),
            status_message: None,
            editing_state: None,
            confirm_state: None,
            rebase_state: None,
            moving_bookmark_state: None,
            bookmark_input_state: None,
            last_op: None,
            sh: sh.clone(),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let mut terminal = ratatui::init();
        let result = self.run_loop(&mut terminal);
        ratatui::restore();
        result
    }

    fn run_loop(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            let viewport_height = terminal.size()?.height.saturating_sub(3) as usize;
            self.tree.update_scroll(viewport_height);

            // fetch diff stats for expanded entry if needed
            self.ensure_expanded_stats();

            terminal.draw(|frame| ui::render(frame, self))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key, viewport_height);
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: event::KeyEvent, viewport_height: usize) {
        // clear expired status messages
        if let Some(ref msg) = self.status_message {
            if Instant::now() > msg.expires {
                self.status_message = None;
            }
        }

        match self.mode {
            Mode::Normal => self.handle_normal_key(key, viewport_height),
            Mode::Help => self.handle_help_key(key.code),
            Mode::ViewingDiff => self.handle_diff_key(key.code),
            Mode::Editing => self.handle_editing_key(key),
            Mode::Confirming => self.handle_confirm_key(key.code),
            Mode::Selecting => self.handle_selecting_key(key, viewport_height),
            Mode::Rebasing => self.handle_rebasing_key(key.code),
            Mode::MovingBookmark => self.handle_moving_bookmark_key(key.code),
            Mode::BookmarkInput => self.handle_bookmark_input_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: event::KeyEvent, viewport_height: usize) {
        let code = key.code;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => {
                if !self.tree.selected.is_empty() {
                    self.tree.clear_selection();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('?') => self.mode = Mode::Help,

            KeyCode::Char('j') | KeyCode::Down => self.tree.move_cursor_down(),
            KeyCode::Char('k') | KeyCode::Up => self.tree.move_cursor_up(),
            KeyCode::Char('g') => self.tree.move_cursor_top(),
            KeyCode::Char('G') => self.tree.move_cursor_bottom(),
            KeyCode::Char('@') => self.tree.jump_to_working_copy(),

            KeyCode::Char('f') => self.tree.toggle_full_mode(),

            // diff viewing
            KeyCode::Char('D') => {
                let _ = self.enter_diff_view();
            }

            // details toggle
            KeyCode::Char(' ') => self.tree.toggle_expanded(),

            // page scrolling
            KeyCode::Char('u') if ctrl => self.tree.page_up(viewport_height / 2),
            KeyCode::Char('d') if ctrl => self.tree.page_down(viewport_height / 2),

            // split view toggle
            KeyCode::Char('\\') => self.split_view = !self.split_view,

            // edit operations
            KeyCode::Char('d') => {
                let _ = self.enter_edit_description();
            }
            KeyCode::Char('e') => {
                let _ = self.edit_working_copy();
            }
            KeyCode::Char('n') => {
                let _ = self.create_new_commit();
            }
            KeyCode::Char('c') => {
                let _ = self.commit_working_copy();
            }

            // selection
            KeyCode::Char('x') => self.toggle_selection(),
            KeyCode::Char('v') => self.enter_visual_selection(),
            KeyCode::Char('a') => self.request_abandon(),

            // rebase operations
            KeyCode::Char('r') => {
                let _ = self.enter_rebase_mode(RebaseType::Single);
            }
            KeyCode::Char('s') => {
                let _ = self.enter_rebase_mode(RebaseType::WithDescendants);
            }
            KeyCode::Char('t') => {
                let _ = self.quick_rebase_onto_trunk(RebaseType::Single);
            }
            KeyCode::Char('T') => {
                let _ = self.quick_rebase_onto_trunk(RebaseType::WithDescendants);
            }

            // undo
            KeyCode::Char('u') => {
                let _ = self.undo_last_operation();
            }

            // bookmark operations
            KeyCode::Char('m') => {
                let _ = self.enter_move_bookmark_mode();
            }
            KeyCode::Char('b') => {
                let _ = self.enter_create_bookmark();
            }
            KeyCode::Char('B') => {
                let _ = self.delete_bookmark();
            }

            _ => {}
        }
    }

    fn handle_help_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }

    fn handle_diff_key(&mut self, code: KeyCode) {
        if let Some(ref mut state) = self.diff_state {
            match code {
                KeyCode::Char('j') | KeyCode::Down => {
                    state.scroll_offset = state.scroll_offset.saturating_add(1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    state.scroll_offset = state.scroll_offset.saturating_sub(1);
                }
                KeyCode::Char('d') => {
                    state.scroll_offset = state.scroll_offset.saturating_add(20);
                }
                KeyCode::Char('u') => {
                    state.scroll_offset = state.scroll_offset.saturating_sub(20);
                }
                KeyCode::Char('g') => {
                    state.scroll_offset = 0;
                }
                KeyCode::Char('G') => {
                    state.scroll_offset = state.lines.len().saturating_sub(1);
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.mode = Mode::Normal;
                }
                _ => {}
            }
        } else {
            // no diff state, return to normal
            self.mode = Mode::Normal;
        }
    }

    fn enter_diff_view(&mut self) -> Result<()> {
        let rev = self.current_rev();
        let diff_output = cmd!(self.sh, "jj diff --git -r {rev}").quiet().ignore_stderr().read()?;
        let lines = parse_diff(&diff_output);
        self.diff_state = Some(DiffState {
            lines,
            scroll_offset: 0,
            rev: rev.to_string(),
        });
        self.mode = Mode::ViewingDiff;
        Ok(())
    }

    fn current_rev(&self) -> String {
        self.tree
            .current_node()
            .map(|n| n.change_id.clone())
            .unwrap_or_default()
    }

    pub fn get_diff_stats(&mut self, change_id: &str) -> Option<&DiffStats> {
        if !self.diff_stats_cache.contains_key(change_id) {
            if let Ok(stats) = self.fetch_diff_stats(change_id) {
                self.diff_stats_cache.insert(change_id.to_string(), stats);
            }
        }
        self.diff_stats_cache.get(change_id)
    }

    fn fetch_diff_stats(&self, change_id: &str) -> Result<DiffStats> {
        let output = cmd!(self.sh, "jj diff --stat -r {change_id}").quiet().ignore_stderr().read()?;

        // parse output like: "3 files changed, 45 insertions(+), 12 deletions(-)"
        // or individual file lines and final summary
        let mut files_changed = 0;
        let mut insertions = 0;
        let mut deletions = 0;

        for line in output.lines() {
            // look for the summary line
            if line.contains("file") && line.contains("changed") {
                // parse: "N file(s) changed, M insertion(s)(+), K deletion(s)(-)"
                for part in line.split(',') {
                    let part = part.trim();
                    if part.contains("file") {
                        if let Some(num) = part.split_whitespace().next() {
                            files_changed = num.parse().unwrap_or(0);
                        }
                    } else if part.contains("insertion") {
                        if let Some(num) = part.split_whitespace().next() {
                            insertions = num.parse().unwrap_or(0);
                        }
                    } else if part.contains("deletion") {
                        if let Some(num) = part.split_whitespace().next() {
                            deletions = num.parse().unwrap_or(0);
                        }
                    }
                }
            }
        }

        Ok(DiffStats {
            files_changed,
            insertions,
            deletions,
        })
    }

    pub fn ensure_expanded_stats(&mut self) {
        if let Some(entry) = self.tree.current_entry() {
            if self.tree.is_expanded(self.tree.cursor) {
                let node = &self.tree.nodes[entry.node_index];
                let change_id = node.change_id.clone();
                let _ = self.get_diff_stats(&change_id);
            }
        }
    }

    fn set_status(&mut self, text: &str, kind: MessageKind) {
        self.status_message = Some(StatusMessage {
            text: text.to_string(),
            kind,
            expires: Instant::now() + Duration::from_secs(3),
        });
    }

    fn refresh_tree(&mut self) -> Result<()> {
        let jj_repo = JjRepo::load(None)?;
        self.tree = TreeState::load(&jj_repo)?;
        self.tree.clear_selection();
        self.diff_stats_cache.clear();
        Ok(())
    }

    // Edit operations

    fn edit_working_copy(&mut self) -> Result<()> {
        let rev = self.current_rev();
        if let Some(node) = self.tree.current_node() {
            if node.is_working_copy {
                self.set_status("Already editing this revision", MessageKind::Warning);
                return Ok(());
            }
        }
        match cmd!(self.sh, "jj edit {rev}").quiet().ignore_stdout().ignore_stderr().run() {
            Ok(_) => {
                self.set_status(&format!("Now editing {rev}"), MessageKind::Success);
                self.refresh_tree()?;
            }
            Err(e) => self.set_status(&format!("Edit failed: {e}"), MessageKind::Error),
        }
        Ok(())
    }

    fn create_new_commit(&mut self) -> Result<()> {
        let rev = self.current_rev();
        match cmd!(self.sh, "jj new {rev}").quiet().ignore_stdout().ignore_stderr().run() {
            Ok(_) => {
                self.set_status("Created new commit", MessageKind::Success);
                self.refresh_tree()?;
                self.tree.jump_to_working_copy();
            }
            Err(e) => self.set_status(&format!("Failed: {e}"), MessageKind::Error),
        }
        Ok(())
    }

    fn commit_working_copy(&mut self) -> Result<()> {
        if let Some(node) = self.tree.current_node() {
            if !node.is_working_copy {
                self.set_status("Can only commit from working copy (@)", MessageKind::Warning);
                return Ok(());
            }
        }
        // use -m with current description to avoid opening $EDITOR
        let desc = self.tree.current_node()
            .map(|n| n.description.clone())
            .unwrap_or_default();
        let desc = if desc.is_empty() { "(no description)".to_string() } else { desc };
        match cmd!(self.sh, "jj commit -m {desc}").quiet().ignore_stdout().ignore_stderr().run() {
            Ok(_) => {
                self.set_status("Changes committed", MessageKind::Success);
                self.refresh_tree()?;
            }
            Err(e) => self.set_status(&format!("Commit failed: {e}"), MessageKind::Error),
        }
        Ok(())
    }

    // Selection operations

    fn toggle_selection(&mut self) {
        self.tree.toggle_selected(self.tree.cursor);
    }

    fn enter_visual_selection(&mut self) {
        self.tree.selection_anchor = Some(self.tree.cursor);
        self.tree.selected.insert(self.tree.cursor);
        self.mode = Mode::Selecting;
    }

    fn handle_selecting_key(&mut self, key: event::KeyEvent, _viewport_height: usize) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.tree.move_cursor_down();
                self.extend_selection_to_cursor();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.tree.move_cursor_up();
                self.extend_selection_to_cursor();
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.tree.selection_anchor = None;
            }
            KeyCode::Char('a') => self.request_abandon(),
            _ => {}
        }
    }

    fn extend_selection_to_cursor(&mut self) {
        if let Some(anchor) = self.tree.selection_anchor {
            self.tree.selected.clear();
            self.tree.select_range(anchor, self.tree.cursor);
        }
    }

    // Confirmation dialog

    fn request_abandon(&mut self) {
        let revs: Vec<String> = if self.tree.selected.is_empty() {
            vec![self.current_rev()]
        } else {
            self.tree
                .selected
                .iter()
                .filter_map(|&idx| {
                    self.tree
                        .visible_entries
                        .get(idx)
                        .map(|e| self.tree.nodes[e.node_index].change_id.clone())
                })
                .collect()
        };

        // check for working copy in selection
        for rev in &revs {
            if self
                .tree
                .nodes
                .iter()
                .any(|n| n.change_id == *rev && n.is_working_copy)
            {
                self.set_status("Cannot abandon working copy", MessageKind::Error);
                return;
            }
        }

        let count = revs.len();
        let message = if count == 1 {
            format!("Abandon revision {}?", revs[0])
        } else {
            format!("Abandon {} revisions?", count)
        };

        self.confirm_state = Some(ConfirmState {
            action: ConfirmAction::Abandon,
            message,
            revs,
        });
        self.mode = Mode::Confirming;
    }

    fn handle_confirm_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('y') | KeyCode::Enter => self.execute_confirmed_action(),
            KeyCode::Char('n') | KeyCode::Esc => self.cancel_confirmation(),
            _ => {}
        }
    }

    fn execute_confirmed_action(&mut self) {
        if let Some(state) = self.confirm_state.take() {
            match state.action {
                ConfirmAction::Abandon => {
                    let revset = state.revs.join(" | ");
                    match cmd!(self.sh, "jj abandon {revset}").quiet().ignore_stdout().ignore_stderr().run() {
                        Ok(_) => {
                            let count = state.revs.len();
                            let msg = if count == 1 {
                                "Revision abandoned".to_string()
                            } else {
                                format!("{} revisions abandoned", count)
                            };
                            self.set_status(&msg, MessageKind::Success);
                            let _ = self.refresh_tree();
                        }
                        Err(e) => {
                            self.set_status(&format!("Abandon failed: {e}"), MessageKind::Error)
                        }
                    }
                }
            }
            self.tree.clear_selection();
            self.mode = Mode::Normal;
        }
    }

    fn cancel_confirmation(&mut self) {
        self.confirm_state = None;
        self.mode = Mode::Normal;
    }

    // Description editing

    fn enter_edit_description(&mut self) -> Result<()> {
        let rev = self.current_rev();
        let desc = cmd!(self.sh, "jj log -r {rev} -T description --no-graph").quiet().ignore_stderr().read()?;

        self.editing_state = Some(EditingState {
            text: desc.clone(),
            cursor: desc.len(),
            target_rev: rev,
            original_desc: desc,
        });
        self.mode = Mode::Editing;
        Ok(())
    }

    fn handle_editing_key(&mut self, key: event::KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        if let Some(ref mut state) = self.editing_state {
            match key.code {
                KeyCode::Enter if ctrl => self.save_description(),
                KeyCode::Esc => self.cancel_editing(),
                KeyCode::Char(c) => {
                    state.text.insert(state.cursor, c);
                    state.cursor += c.len_utf8();
                }
                KeyCode::Backspace => {
                    if state.cursor > 0 {
                        let prev_char_boundary = state.text[..state.cursor]
                            .char_indices()
                            .last()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        state.text.remove(prev_char_boundary);
                        state.cursor = prev_char_boundary;
                    }
                }
                KeyCode::Delete => {
                    if state.cursor < state.text.len() {
                        state.text.remove(state.cursor);
                    }
                }
                KeyCode::Left => {
                    if state.cursor > 0 {
                        state.cursor = state.text[..state.cursor]
                            .char_indices()
                            .last()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                    }
                }
                KeyCode::Right => {
                    if state.cursor < state.text.len() {
                        state.cursor = state.text[state.cursor..]
                            .char_indices()
                            .nth(1)
                            .map(|(i, _)| state.cursor + i)
                            .unwrap_or(state.text.len());
                    }
                }
                KeyCode::Home => {
                    // move to start of current line
                    state.cursor = state.text[..state.cursor]
                        .rfind('\n')
                        .map(|i| i + 1)
                        .unwrap_or(0);
                }
                KeyCode::End => {
                    // move to end of current line
                    state.cursor = state.text[state.cursor..]
                        .find('\n')
                        .map(|i| state.cursor + i)
                        .unwrap_or(state.text.len());
                }
                KeyCode::Up => {
                    // move to previous line, same column
                    let text_before = &state.text[..state.cursor];
                    let current_line_start = text_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
                    let col = state.cursor - current_line_start;

                    if current_line_start > 0 {
                        let prev_line_end = current_line_start - 1;
                        let prev_line_start = state.text[..prev_line_end].rfind('\n').map(|i| i + 1).unwrap_or(0);
                        let prev_line_len = prev_line_end - prev_line_start;
                        state.cursor = prev_line_start + col.min(prev_line_len);
                    }
                }
                KeyCode::Down => {
                    // move to next line, same column
                    let text_before = &state.text[..state.cursor];
                    let current_line_start = text_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
                    let col = state.cursor - current_line_start;

                    if let Some(next_newline) = state.text[state.cursor..].find('\n') {
                        let next_line_start = state.cursor + next_newline + 1;
                        let next_line_end = state.text[next_line_start..].find('\n')
                            .map(|i| next_line_start + i)
                            .unwrap_or(state.text.len());
                        let next_line_len = next_line_end - next_line_start;
                        state.cursor = next_line_start + col.min(next_line_len);
                    }
                }
                KeyCode::Enter => {
                    state.text.insert(state.cursor, '\n');
                    state.cursor += 1;
                }
                _ => {}
            }
        }
    }

    fn save_description(&mut self) {
        if let Some(state) = self.editing_state.take() {
            let new_desc = &state.text;
            if *new_desc != state.original_desc {
                let rev = &state.target_rev;
                match cmd!(self.sh, "jj desc -r {rev} -m {new_desc}").quiet().ignore_stdout().ignore_stderr().run() {
                    Ok(_) => {
                        self.set_status("Description updated", MessageKind::Success);
                        let _ = self.refresh_tree();
                    }
                    Err(e) => self.set_status(&format!("Failed: {e}"), MessageKind::Error),
                }
            }
            self.mode = Mode::Normal;
        }
    }

    fn cancel_editing(&mut self) {
        self.editing_state = None;
        self.mode = Mode::Normal;
    }

    // Rebase operations

    fn get_current_operation_id(&self) -> Result<String> {
        let output = cmd!(self.sh, "jj op log --limit 1 -T id --no-graph").quiet().ignore_stderr().read()?;
        Ok(output.trim().to_string())
    }

    fn enter_rebase_mode(&mut self, rebase_type: RebaseType) -> Result<()> {
        let source_rev = self.current_rev();
        if source_rev.is_empty() {
            self.set_status("No revision selected", MessageKind::Error);
            return Ok(());
        }

        // capture current operation ID for potential undo
        let op_before = self.get_current_operation_id().unwrap_or_default();

        // temporarily create rebase state to compute moving indices
        self.rebase_state = Some(RebaseState {
            source_rev: source_rev.clone(),
            rebase_type,
            dest_cursor: self.tree.cursor,
            allow_branches: false,
            op_before,
        });

        // find source's parent so initial preview shows source at its original position
        let moving = self.compute_moving_indices();
        let max = self.tree.visible_count();
        let current = self.tree.cursor;

        // get source's structural depth
        let source_struct_depth = self.tree.visible_entries
            .get(current)
            .map(|e| self.tree.nodes[e.node_index].depth)
            .unwrap_or(0);

        // find source's parent: closest entry above with smaller structural depth
        let mut initial_cursor = current.saturating_sub(1);
        while initial_cursor > 0 {
            let entry = &self.tree.visible_entries[initial_cursor];
            let node = &self.tree.nodes[entry.node_index];
            if node.depth < source_struct_depth && !moving.contains(&initial_cursor) {
                break;
            }
            initial_cursor -= 1;
        }

        // verify we found a valid non-moving entry
        if moving.contains(&initial_cursor) || initial_cursor >= max {
            // fallback: search forward for any non-moving entry
            initial_cursor = 0;
            while initial_cursor < max && moving.contains(&initial_cursor) {
                initial_cursor += 1;
            }
        }

        if let Some(ref mut state) = self.rebase_state {
            state.dest_cursor = initial_cursor;
        }

        self.mode = Mode::Rebasing;
        Ok(())
    }

    fn handle_rebasing_key(&mut self, code: KeyCode) {
        // clone rebase_state to avoid borrow issues
        let state = match self.rebase_state.as_ref() {
            Some(s) => s.clone(),
            None => {
                self.mode = Mode::Normal;
                return;
            }
        };

        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_rebase_dest_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_rebase_dest_up();
            }
            KeyCode::Char('b') => {
                if let Some(ref mut s) = self.rebase_state {
                    s.allow_branches = !s.allow_branches;
                }
            }
            KeyCode::Enter => {
                let _ = self.execute_rebase(&state);
            }
            KeyCode::Esc => {
                self.cancel_rebase();
            }
            _ => {}
        }
    }

    fn move_rebase_dest_up(&mut self) {
        let moving = self.compute_moving_indices();
        if let Some(ref mut state) = self.rebase_state {
            let mut next = state.dest_cursor.saturating_sub(1);
            // skip over moving entries
            while next > 0 && moving.contains(&next) {
                next -= 1;
            }
            // only move if we found a valid non-moving position
            if !moving.contains(&next) {
                state.dest_cursor = next;
            }
        }
    }

    fn move_rebase_dest_down(&mut self) {
        let moving = self.compute_moving_indices();
        let max = self.tree.visible_count();
        if let Some(ref mut state) = self.rebase_state {
            let mut next = state.dest_cursor + 1;
            // skip over moving entries
            while next < max && moving.contains(&next) {
                next += 1;
            }
            // only move if we found a valid position
            if next < max {
                state.dest_cursor = next;
            }
        }
    }

    fn get_rev_at_cursor(&self, cursor: usize) -> Option<String> {
        self.tree
            .visible_entries
            .get(cursor)
            .map(|e| self.tree.nodes[e.node_index].change_id.clone())
    }

    fn get_first_child(&self, rev: &str) -> Result<Option<String>> {
        let output = cmd!(
            self.sh,
            "jj log -r children({rev}) -T change_id --no-graph --limit 1"
        )
        .read()?;
        let trimmed = output.trim();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed.to_string()))
        }
    }

    fn execute_rebase(&mut self, state: &RebaseState) -> Result<()> {
        let source = &state.source_rev;
        let dest = match self.get_rev_at_cursor(state.dest_cursor) {
            Some(d) => d,
            None => {
                self.set_status("Invalid destination", MessageKind::Error);
                return Ok(());
            }
        };

        // don't allow rebasing onto self
        if *source == dest {
            self.set_status("Cannot rebase onto self", MessageKind::Error);
            return Ok(());
        }

        let mode_flag = match state.rebase_type {
            RebaseType::Single => "-r",
            RebaseType::WithDescendants => "-s",
        };

        let result = if state.allow_branches {
            // simple -A only (creates branch point)
            cmd!(self.sh, "jj rebase {mode_flag} {source} -A {dest}")
                .quiet()
                .ignore_stdout()
                .ignore_stderr()
                .run()
        } else {
            // clean inline: try to insert between dest and its first child
            match self.get_first_child(&dest) {
                Ok(Some(next)) => cmd!(
                    self.sh,
                    "jj rebase {mode_flag} {source} -A {dest} -B {next}"
                )
                .quiet()
                .ignore_stdout()
                .ignore_stderr()
                .run(),
                _ => cmd!(self.sh, "jj rebase {mode_flag} {source} -A {dest}")
                    .quiet()
                    .ignore_stdout()
                    .ignore_stderr()
                    .run(),
            }
        };

        match result {
            Ok(_) => {
                // store operation for undo
                self.last_op = Some(state.op_before.clone());

                // check for conflicts
                let has_conflicts = self.check_conflicts();

                self.rebase_state = None;
                self.mode = Mode::Normal;
                let _ = self.refresh_tree();

                if has_conflicts {
                    self.set_status(
                        "Rebase created conflicts. Press u to undo",
                        MessageKind::Warning,
                    );
                } else {
                    self.set_status("Rebase complete", MessageKind::Success);
                }
            }
            Err(e) => {
                self.set_status(&format!("Rebase failed: {e}"), MessageKind::Error);
            }
        }
        Ok(())
    }

    fn check_conflicts(&self) -> bool {
        cmd!(self.sh, "jj log -r @ -T 'if(conflict, \"conflict\")'")
            .quiet()
            .ignore_stderr()
            .read()
            .map(|s| s.contains("conflict"))
            .unwrap_or(false)
    }

    fn cancel_rebase(&mut self) {
        self.rebase_state = None;
        self.mode = Mode::Normal;
    }

    fn quick_rebase_onto_trunk(&mut self, rebase_type: RebaseType) -> Result<()> {
        let source = self.current_rev();
        if source.is_empty() {
            self.set_status("No revision selected", MessageKind::Error);
            return Ok(());
        }

        // capture operation ID for undo
        let op_before = self.get_current_operation_id().unwrap_or_default();

        let mode_flag = match rebase_type {
            RebaseType::Single => "-r",
            RebaseType::WithDescendants => "-s",
        };

        match cmd!(self.sh, "jj rebase {mode_flag} {source} -d trunk()")
            .quiet()
            .ignore_stdout()
            .ignore_stderr()
            .run()
        {
            Ok(_) => {
                self.last_op = Some(op_before);
                let has_conflicts = self.check_conflicts();
                let _ = self.refresh_tree();

                if has_conflicts {
                    self.set_status(
                        "Rebased onto trunk but conflicts exist. Press u to undo",
                        MessageKind::Warning,
                    );
                } else {
                    let msg = match rebase_type {
                        RebaseType::Single => "Rebased revision onto trunk",
                        RebaseType::WithDescendants => "Rebased tree onto trunk",
                    };
                    self.set_status(msg, MessageKind::Success);
                }
            }
            Err(e) => {
                self.set_status(&format!("Rebase failed: {e}"), MessageKind::Error);
            }
        }
        Ok(())
    }

    fn undo_last_operation(&mut self) -> Result<()> {
        if let Some(ref op_id) = self.last_op.take() {
            match cmd!(self.sh, "jj op restore {op_id}").quiet().ignore_stdout().ignore_stderr().run() {
                Ok(_) => {
                    self.set_status("Operation undone", MessageKind::Success);
                    let _ = self.refresh_tree();
                }
                Err(e) => {
                    self.set_status(&format!("Undo failed: {e}"), MessageKind::Error);
                }
            }
        } else {
            self.set_status("Nothing to undo", MessageKind::Warning);
        }
        Ok(())
    }

    // Bookmark operations

    fn enter_move_bookmark_mode(&mut self) -> Result<()> {
        let node = match self.tree.current_node() {
            Some(n) => n,
            None => {
                self.set_status("No revision selected", MessageKind::Error);
                return Ok(());
            }
        };

        if node.bookmarks.is_empty() {
            self.set_status("No bookmarks on this revision", MessageKind::Warning);
            return Ok(());
        }

        // if multiple bookmarks, use the first one
        let bookmark_name = node.bookmarks[0].clone();
        let op_before = self.get_current_operation_id().unwrap_or_default();

        self.moving_bookmark_state = Some(MovingBookmarkState {
            bookmark_name,
            dest_cursor: self.tree.cursor,
            op_before,
        });
        self.mode = Mode::MovingBookmark;
        Ok(())
    }

    fn handle_moving_bookmark_key(&mut self, code: KeyCode) {
        let state = match self.moving_bookmark_state.as_ref() {
            Some(s) => s.clone(),
            None => {
                self.mode = Mode::Normal;
                return;
            }
        };

        match code {
            KeyCode::Char('j') | KeyCode::Down => self.move_bookmark_dest_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_bookmark_dest_up(),
            KeyCode::Enter => {
                let _ = self.execute_bookmark_move(&state);
            }
            KeyCode::Esc => self.cancel_bookmark_move(),
            _ => {}
        }
    }

    fn move_bookmark_dest_up(&mut self) {
        if let Some(ref mut state) = self.moving_bookmark_state {
            if state.dest_cursor > 0 {
                state.dest_cursor -= 1;
            }
        }
    }

    fn move_bookmark_dest_down(&mut self) {
        if let Some(ref mut state) = self.moving_bookmark_state {
            let max = self.tree.visible_count().saturating_sub(1);
            if state.dest_cursor < max {
                state.dest_cursor += 1;
            }
        }
    }

    fn execute_bookmark_move(&mut self, state: &MovingBookmarkState) -> Result<()> {
        let dest = match self.get_rev_at_cursor(state.dest_cursor) {
            Some(d) => d,
            None => {
                self.set_status("Invalid destination", MessageKind::Error);
                return Ok(());
            }
        };

        let name = &state.bookmark_name;
        match cmd!(self.sh, "jj bookmark set {name} -r {dest}").quiet().ignore_stdout().ignore_stderr().run() {
            Ok(_) => {
                self.last_op = Some(state.op_before.clone());
                let _ = self.refresh_tree();
                self.set_status(&format!("Moved bookmark '{name}' to {}", &dest[..8.min(dest.len())]), MessageKind::Success);
            }
            Err(e) => {
                self.set_status(&format!("Move bookmark failed: {e}"), MessageKind::Error);
            }
        }

        self.moving_bookmark_state = None;
        self.mode = Mode::Normal;
        Ok(())
    }

    fn cancel_bookmark_move(&mut self) {
        self.moving_bookmark_state = None;
        self.mode = Mode::Normal;
    }

    fn enter_create_bookmark(&mut self) -> Result<()> {
        let rev = self.current_rev();
        if rev.is_empty() {
            self.set_status("No revision selected", MessageKind::Error);
            return Ok(());
        }

        self.bookmark_input_state = Some(BookmarkInputState {
            name: String::new(),
            cursor: 0,
            target_rev: rev,
            deleting: false,
        });
        self.mode = Mode::BookmarkInput;
        Ok(())
    }

    fn delete_bookmark(&mut self) -> Result<()> {
        // extract data we need before taking any mutable borrows
        let (bookmarks, change_id) = match self.tree.current_node() {
            Some(n) => (n.bookmarks.clone(), n.change_id.clone()),
            None => {
                self.set_status("No revision selected", MessageKind::Error);
                return Ok(());
            }
        };

        if bookmarks.is_empty() {
            self.set_status("No bookmarks on this revision", MessageKind::Warning);
            return Ok(());
        }

        // if multiple bookmarks, enter selection mode; otherwise delete directly
        if bookmarks.len() == 1 {
            let name = &bookmarks[0];
            let op_before = self.get_current_operation_id().unwrap_or_default();

            match cmd!(self.sh, "jj bookmark delete {name}").quiet().ignore_stdout().ignore_stderr().run() {
                Ok(_) => {
                    self.last_op = Some(op_before);
                    let _ = self.refresh_tree();
                    self.set_status(&format!("Deleted bookmark '{name}'"), MessageKind::Success);
                }
                Err(e) => {
                    self.set_status(&format!("Delete bookmark failed: {e}"), MessageKind::Error);
                }
            }
        } else {
            // multiple bookmarks - show input with first bookmark name for selection
            self.bookmark_input_state = Some(BookmarkInputState {
                name: bookmarks[0].clone(),
                cursor: bookmarks[0].len(),
                target_rev: change_id,
                deleting: true,
            });
            self.mode = Mode::BookmarkInput;
        }
        Ok(())
    }

    fn handle_bookmark_input_key(&mut self, key: event::KeyEvent) {
        if let Some(ref mut state) = self.bookmark_input_state {
            match key.code {
                KeyCode::Enter => {
                    let name = state.name.clone();
                    let target = state.target_rev.clone();
                    let deleting = state.deleting;
                    self.execute_bookmark_input(&name, &target, deleting);
                }
                KeyCode::Esc => {
                    self.bookmark_input_state = None;
                    self.mode = Mode::Normal;
                }
                KeyCode::Char(c) => {
                    state.name.insert(state.cursor, c);
                    state.cursor += c.len_utf8();
                }
                KeyCode::Backspace => {
                    if state.cursor > 0 {
                        let prev = state.name[..state.cursor]
                            .char_indices()
                            .last()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        state.name.remove(prev);
                        state.cursor = prev;
                    }
                }
                KeyCode::Delete => {
                    if state.cursor < state.name.len() {
                        state.name.remove(state.cursor);
                    }
                }
                KeyCode::Left => {
                    if state.cursor > 0 {
                        state.cursor = state.name[..state.cursor]
                            .char_indices()
                            .last()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                    }
                }
                KeyCode::Right => {
                    if state.cursor < state.name.len() {
                        state.cursor = state.name[state.cursor..]
                            .char_indices()
                            .nth(1)
                            .map(|(i, _)| state.cursor + i)
                            .unwrap_or(state.name.len());
                    }
                }
                _ => {}
            }
        }
    }

    fn execute_bookmark_input(&mut self, name: &str, target: &str, deleting: bool) {
        if name.is_empty() {
            self.set_status("Bookmark name cannot be empty", MessageKind::Error);
            self.bookmark_input_state = None;
            self.mode = Mode::Normal;
            return;
        }

        let op_before = self.get_current_operation_id().unwrap_or_default();

        let result = if deleting {
            cmd!(self.sh, "jj bookmark delete {name}").quiet().ignore_stdout().ignore_stderr().run()
        } else {
            cmd!(self.sh, "jj bookmark create {name} -r {target}").quiet().ignore_stdout().ignore_stderr().run()
        };

        match result {
            Ok(_) => {
                self.last_op = Some(op_before);
                let _ = self.refresh_tree();
                let action = if deleting { "Deleted" } else { "Created" };
                self.set_status(&format!("{action} bookmark '{name}'"), MessageKind::Success);
            }
            Err(e) => {
                let action = if deleting { "Delete" } else { "Create" };
                self.set_status(&format!("{action} bookmark failed: {e}"), MessageKind::Error);
            }
        }

        self.bookmark_input_state = None;
        self.mode = Mode::Normal;
    }

    pub fn current_has_bookmark(&self) -> bool {
        self.tree
            .current_node()
            .map(|n| !n.bookmarks.is_empty())
            .unwrap_or(false)
    }

    /// Compute indices of entries that will move during rebase
    /// For 's' mode: source + all descendants
    /// For 'r' mode: only source
    pub fn compute_moving_indices(&self) -> HashSet<usize> {
        let Some(ref state) = self.rebase_state else {
            return HashSet::new();
        };

        let mut indices = HashSet::new();
        let mut in_source_tree = false;
        let mut source_struct_depth = 0usize;

        for (idx, entry) in self.tree.visible_entries.iter().enumerate() {
            let node = &self.tree.nodes[entry.node_index];

            if node.change_id == state.source_rev {
                indices.insert(idx);
                if state.rebase_type == RebaseType::WithDescendants {
                    in_source_tree = true;
                    source_struct_depth = node.depth;
                }
            } else if in_source_tree {
                if node.depth > source_struct_depth {
                    indices.insert(idx);
                } else {
                    break;
                }
            }
        }

        indices
    }
}

fn syntect_to_ratatui_color(style: SyntectStyle) -> Color {
    Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b)
}

/// Parse ANSI escape codes into styled spans (for bat fallback)
fn parse_ansi_line(line: &str) -> Vec<StyledSpan> {
    let mut spans = Vec::new();
    let mut current_color = Color::White;
    let mut current_text = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            if !current_text.is_empty() {
                spans.push(StyledSpan {
                    text: std::mem::take(&mut current_text),
                    fg: current_color,
                });
            }

            chars.next(); // consume '['

            let mut seq = String::new();
            while let Some(&sc) = chars.peek() {
                if sc.is_ascii_alphabetic() {
                    chars.next();
                    break;
                }
                seq.push(chars.next().unwrap());
            }

            // parse color codes
            for code in seq.split(';') {
                match code {
                    "0" => current_color = Color::White,
                    "30" => current_color = Color::Black,
                    "31" => current_color = Color::Red,
                    "32" => current_color = Color::Green,
                    "33" => current_color = Color::Yellow,
                    "34" => current_color = Color::Blue,
                    "35" => current_color = Color::Magenta,
                    "36" => current_color = Color::Cyan,
                    "37" => current_color = Color::White,
                    "90" => current_color = Color::DarkGray,
                    "91" => current_color = Color::LightRed,
                    "92" => current_color = Color::LightGreen,
                    "93" => current_color = Color::LightYellow,
                    "94" => current_color = Color::LightBlue,
                    "95" => current_color = Color::LightMagenta,
                    "96" => current_color = Color::LightCyan,
                    "97" => current_color = Color::White,
                    s if s.starts_with("38;5;") => {
                        if let Ok(n) = s[5..].parse::<u8>() {
                            current_color = Color::Indexed(n);
                        }
                    }
                    s if s.starts_with("38;2;") => {
                        let parts: Vec<&str> = s[5..].split(';').collect();
                        if parts.len() >= 3 {
                            if let (Ok(r), Ok(g), Ok(b)) = (
                                parts[0].parse::<u8>(),
                                parts[1].parse::<u8>(),
                                parts[2].parse::<u8>(),
                            ) {
                                current_color = Color::Rgb(r, g, b);
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else {
            current_text.push(c);
        }
    }

    if !current_text.is_empty() {
        spans.push(StyledSpan {
            text: current_text,
            fg: current_color,
        });
    }

    if spans.is_empty() {
        spans.push(StyledSpan {
            text: String::new(),
            fg: Color::White,
        });
    }

    spans
}

/// Try to highlight code using bat (fallback for unsupported syntect languages)
fn highlight_with_bat(code: &str, extension: &str) -> Option<Vec<StyledSpan>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("bat")
        .args([
            "--color=always",
            "--style=plain",
            "--paging=never",
            &format!("--language={extension}"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    child.stdin.take()?.write_all(code.as_bytes()).ok()?;
    let output = child.wait_with_output().ok()?;

    if !output.status.success() {
        return None;
    }

    let highlighted = String::from_utf8_lossy(&output.stdout);
    let line = highlighted.trim_end_matches('\n');
    Some(parse_ansi_line(line))
}

fn parse_diff(output: &str) -> Vec<DiffLine> {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes["base16-eighties.dark"];
    let plain_text = ss.find_syntax_plain_text();

    let mut current_file: Option<String> = None;
    let mut current_ext: Option<String> = None;
    let mut lines = Vec::new();

    for line in output.lines() {
        let (kind, code_content) = if line.starts_with("diff --git") {
            // extract filename from "diff --git a/path/file.rs b/path/file.rs"
            if let Some(b_path) = line.split(" b/").nth(1) {
                current_file = Some(b_path.to_string());
                current_ext = std::path::Path::new(b_path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_string());
            }
            (DiffLineKind::FileHeader, None)
        } else if line.starts_with("+++") || line.starts_with("---") {
            (DiffLineKind::FileHeader, None)
        } else if line.starts_with("@@") {
            (DiffLineKind::Hunk, None)
        } else if let Some(rest) = line.strip_prefix('+') {
            (DiffLineKind::Added, Some(rest))
        } else if let Some(rest) = line.strip_prefix('-') {
            (DiffLineKind::Removed, Some(rest))
        } else if let Some(rest) = line.strip_prefix(' ') {
            (DiffLineKind::Context, Some(rest))
        } else {
            (DiffLineKind::Context, Some(line))
        };

        let spans = if let Some(code) = code_content {
            let prefix = match kind {
                DiffLineKind::Added => "+",
                DiffLineKind::Removed => "-",
                DiffLineKind::Context => " ",
                _ => "",
            };

            let prefix_color = match kind {
                DiffLineKind::Added => Color::Green,
                DiffLineKind::Removed => Color::Red,
                _ => Color::DarkGray,
            };

            // try syntect first
            let syntax = current_file.as_ref().and_then(|f| {
                std::path::Path::new(f)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .and_then(|ext| ss.find_syntax_by_extension(ext))
            });

            let code_spans = if let Some(syn) = syntax {
                // syntect supports this language
                let mut highlighter = syntect::easy::HighlightLines::new(syn, theme);
                highlighter.highlight_line(code, &ss).ok().map(|ranges| {
                    ranges
                        .into_iter()
                        .map(|(style, text)| StyledSpan {
                            text: text.to_string(),
                            fg: syntect_to_ratatui_color(style),
                        })
                        .collect::<Vec<_>>()
                })
            } else if let Some(ref ext) = current_ext {
                // try bat for unsupported extensions
                highlight_with_bat(code, ext)
            } else {
                None
            };

            // fall back to plain text coloring
            let code_spans = code_spans.unwrap_or_else(|| {
                let mut highlighter = syntect::easy::HighlightLines::new(plain_text, theme);
                highlighter
                    .highlight_line(code, &ss)
                    .map(|ranges| {
                        ranges
                            .into_iter()
                            .map(|(style, text)| StyledSpan {
                                text: text.to_string(),
                                fg: syntect_to_ratatui_color(style),
                            })
                            .collect()
                    })
                    .unwrap_or_else(|_| {
                        vec![StyledSpan {
                            text: code.to_string(),
                            fg: Color::White,
                        }]
                    })
            });

            let mut result = vec![StyledSpan {
                text: prefix.to_string(),
                fg: prefix_color,
            }];
            result.extend(code_spans);
            result
        } else {
            // non-code lines (headers, hunks)
            let color = match kind {
                DiffLineKind::FileHeader => Color::Yellow,
                DiffLineKind::Hunk => Color::Cyan,
                _ => Color::White,
            };
            vec![StyledSpan {
                text: line.to_string(),
                fg: color,
            }]
        };

        lines.push(DiffLine { spans, kind });
    }

    lines
}
