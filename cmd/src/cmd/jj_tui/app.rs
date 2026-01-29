use super::tree::TreeState;
use super::ui;
use crate::jj_lib_helpers::JjRepo;
use eyre::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::style::Color;
use ratatui::DefaultTerminal;
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

pub struct DiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
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
        let diff_output = cmd!(self.sh, "jj diff --git -r {rev}").read()?;
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
        let output = cmd!(self.sh, "jj diff --stat -r {change_id}").read()?;

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
        match cmd!(self.sh, "jj edit {rev}").quiet().run() {
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
        match cmd!(self.sh, "jj new {rev}").run() {
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
        match cmd!(self.sh, "jj commit -m {desc}").quiet().run() {
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
                    match cmd!(self.sh, "jj abandon {revset}").run() {
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
        let desc = cmd!(self.sh, "jj log -r {rev} -T description --no-graph").read()?;

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
                match cmd!(self.sh, "jj desc -r {rev} -m {new_desc}").run() {
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
