use super::app::{App, ConfirmState, DiffLineKind, DiffStats, EditingState, MessageKind, Mode};
use super::tree::TreeNode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    match app.mode {
        Mode::ViewingDiff => {
            if let Some(ref state) = app.diff_state {
                render_diff(frame, state, chunks[0]);
            }
        }
        Mode::Normal | Mode::Help | Mode::Selecting | Mode::Editing | Mode::Confirming => {
            if app.split_view {
                let split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(chunks[0]);
                render_tree(frame, app, split[0]);
                render_diff_pane(frame, app, split[1]);
            } else {
                render_tree(frame, app, chunks[0]);
            }
        }
    }

    render_status_bar(frame, app, chunks[1]);

    // render overlays
    if matches!(app.mode, Mode::Help) {
        render_help(frame);
    }

    if let Some(ref state) = app.editing_state {
        if matches!(app.mode, Mode::Editing) {
            render_editing(frame, state);
        }
    }

    if let Some(ref state) = app.confirm_state {
        if matches!(app.mode, Mode::Confirming) {
            render_confirmation(frame, state);
        }
    }
}

fn render_tree(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" jj tree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.tree.visible_count() == 0 {
        let empty = Paragraph::new("No commits found").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, inner);
        return;
    }

    let viewport_height = inner.height as usize;
    let scroll_offset = app.tree.scroll_offset;

    let mut lines: Vec<Line> = Vec::new();
    let mut line_count = 0;

    for (visible_idx, entry) in app.tree.visible_nodes().enumerate().skip(scroll_offset) {
        if line_count >= viewport_height {
            break;
        }

        let node = app.tree.get_node(entry);
        let is_cursor = visible_idx == app.tree.cursor;
        let is_multi_selected = app.tree.selected.contains(&visible_idx);
        lines.push(render_tree_line(node, entry.visual_depth, is_cursor, is_multi_selected));
        line_count += 1;

        // render expanded details if this entry is expanded
        if app.tree.is_expanded(visible_idx) && line_count < viewport_height {
            let stats = app.diff_stats_cache.get(&node.change_id);
            let detail_lines = render_commit_details(node, entry.visual_depth, stats);
            for detail in detail_lines {
                if line_count >= viewport_height {
                    break;
                }
                lines.push(detail);
                line_count += 1;
            }
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_tree_line(node: &TreeNode, visual_depth: usize, is_cursor: bool, is_multi_selected: bool) -> Line<'static> {
    let indent = "  ".repeat(visual_depth);
    let connector = if visual_depth > 0 { "├── " } else { "" };
    let at_marker = if node.is_working_copy { "@ " } else { "" };

    // selection marker
    let selection_marker = if is_multi_selected { "[x] " } else { "" };

    let (prefix, suffix) = node
        .change_id
        .split_at(node.unique_prefix_len.min(node.change_id.len()));

    // always show revision first
    let mut spans = vec![
        Span::raw(format!("{indent}{connector}{selection_marker}{at_marker}(")),
        Span::styled(prefix.to_string(), Style::default().fg(Color::Magenta)),
        Span::styled(suffix.to_string(), Style::default().fg(Color::DarkGray)),
        Span::raw(")"),
    ];

    // then bookmark if present
    if !node.bookmarks.is_empty() {
        let bookmark_str = node.bookmarks.join(" ");
        spans.push(Span::raw(" "));
        spans.push(Span::styled(bookmark_str, Style::default().fg(Color::Cyan)));
    }

    // then description
    let desc = if node.description.is_empty() {
        if node.is_working_copy {
            "(working copy)".to_string()
        } else {
            "(no description)".to_string()
        }
    } else {
        node.description.clone()
    };
    spans.push(Span::styled(format!("  {desc}"), Style::default().fg(Color::DarkGray)));

    let mut line = Line::from(spans);

    // apply styling: multi-selection has different background than just cursor
    if is_cursor && is_multi_selected {
        line = line.style(
            Style::default()
                .bg(Color::Rgb(60, 40, 60))
                .add_modifier(Modifier::BOLD),
        );
    } else if is_cursor {
        line = line.style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        );
    } else if is_multi_selected {
        line = line.style(
            Style::default()
                .bg(Color::Rgb(40, 50, 40)),
        );
    }

    line
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    // if there's a status message, show it instead of normal status bar
    if let Some(ref msg) = app.status_message {
        if std::time::Instant::now() < msg.expires {
            let color = match msg.kind {
                MessageKind::Info => Color::Blue,
                MessageKind::Success => Color::Green,
                MessageKind::Warning => Color::Yellow,
                MessageKind::Error => Color::Red,
            };
            let bar = Paragraph::new(format!(" {}", msg.text))
                .style(Style::default().bg(Color::Rgb(30, 30, 50)).fg(color));
            frame.render_widget(bar, area);
            return;
        }
    }

    let mode_indicator = match app.mode {
        Mode::Normal => "NORMAL",
        Mode::Help => "HELP",
        Mode::ViewingDiff => "DIFF",
        Mode::Editing => "EDIT",
        Mode::Confirming => "CONFIRM",
        Mode::Selecting => "SELECT",
    };

    let full_indicator = if app.tree.full_mode { " [FULL]" } else { "" };
    let split_indicator = if app.split_view { " [SPLIT]" } else { "" };

    // show selection count when there are selected items
    let selection_indicator = if !app.tree.selected.is_empty() {
        format!(" [{}sel]", app.tree.selected.len())
    } else {
        String::new()
    };

    let current_info = app
        .tree
        .current_node()
        .map(|n| {
            let name = if n.bookmarks.is_empty() {
                n.change_id.clone()
            } else {
                n.bookmarks.join(" ")
            };
            format!(" | {name}")
        })
        .unwrap_or_default();

    let hints = match app.mode {
        Mode::Normal => {
            if !app.tree.selected.is_empty() {
                "a:abandon  x:toggle  Esc:clear"
            } else {
                "d:desc e:edit n:new c:commit a:abandon x/v:select ?:help q:quit"
            }
        }
        Mode::Help => "q/Esc:close",
        Mode::ViewingDiff => "j/k:scroll  d/u:page  g/G:top/bottom  q/Esc:close",
        Mode::Editing => "Ctrl+Enter:save  Esc:cancel",
        Mode::Confirming => "y/Enter:yes  n/Esc:no",
        Mode::Selecting => "j/k:extend  a:abandon  Esc:exit",
    };

    let left = format!(" {mode_indicator}{full_indicator}{split_indicator}{selection_indicator}{current_info}");
    let right = format!("{hints} ");

    let available = area.width as usize;
    let left_len = left.len();
    let right_len = right.len();

    let text = if left_len + right_len < available {
        let padding = available - left_len - right_len;
        format!("{left}{:padding$}{right}", "")
    } else {
        format!("{left}  {hints}")
    };

    let bar =
        Paragraph::new(text).style(Style::default().bg(Color::Rgb(30, 30, 50)).fg(Color::White));

    frame.render_widget(bar, area);
}

fn render_help(frame: &mut Frame) {
    let area = frame.area();
    let popup_width = 56u16.min(area.width.saturating_sub(4));
    let popup_height = 32u16.min(area.height.saturating_sub(4));

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled(
            "Navigation",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  j/↓       Move cursor down"),
        Line::from("  k/↑       Move cursor up"),
        Line::from("  Ctrl+d    Page down"),
        Line::from("  Ctrl+u    Page up"),
        Line::from("  g         Jump to top"),
        Line::from("  G         Jump to bottom"),
        Line::from("  @         Jump to working copy"),
        Line::from(""),
        Line::from(Span::styled(
            "View",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  D         View diff"),
        Line::from("  Space     Toggle commit details"),
        Line::from("  \\         Toggle split view"),
        Line::from("  f         Toggle full mode"),
        Line::from(""),
        Line::from(Span::styled(
            "Edit Operations",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  d         Edit description"),
        Line::from("  e         Edit working copy (jj edit)"),
        Line::from("  n         New commit (jj new)"),
        Line::from("  c         Commit changes (jj commit)"),
        Line::from(""),
        Line::from(Span::styled(
            "Selection",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  x         Toggle selection"),
        Line::from("  v         Visual select mode"),
        Line::from("  a         Abandon selected"),
        Line::from("  Esc       Clear selection"),
        Line::from(""),
        Line::from(Span::styled(
            "General",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?         Toggle help"),
        Line::from("  q/Esc     Quit (or close)"),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    frame.render_widget(help, popup_area);
}

fn render_editing(frame: &mut Frame, state: &EditingState) {
    let area = frame.area();
    let popup_width = 70u16.min(area.width.saturating_sub(4));
    let popup_height = 14u16.min(area.height.saturating_sub(4));

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" Edit: {} ", state.target_rev))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup_area);
    frame.render_widget(block.style(Style::default().bg(Color::Rgb(20, 20, 30))), popup_area);

    // split text into lines and find which line has the cursor
    let text_before_cursor = &state.text[..state.cursor];
    let cursor_line_idx = text_before_cursor.matches('\n').count();
    let cursor_col = text_before_cursor.rfind('\n').map(|i| state.cursor - i - 1).unwrap_or(state.cursor);

    let text_lines: Vec<&str> = state.text.split('\n').collect();

    let mut lines: Vec<Line> = Vec::new();

    for (line_idx, line_text) in text_lines.iter().enumerate() {
        if line_idx == cursor_line_idx {
            // this line has the cursor
            let before = &line_text[..cursor_col.min(line_text.len())];
            let cursor_char = line_text.get(cursor_col..).and_then(|s| s.chars().next());
            let after = if let Some(c) = cursor_char {
                &line_text[cursor_col + c.len_utf8()..]
            } else {
                ""
            };
            let cursor_display = cursor_char.unwrap_or(' ');

            lines.push(Line::from(vec![
                Span::raw(before.to_string()),
                Span::styled(
                    cursor_display.to_string(),
                    Style::default().bg(Color::White).fg(Color::Black),
                ),
                Span::raw(after.to_string()),
            ]));
        } else {
            lines.push(Line::from(line_text.to_string()));
        }
    }

    // add help text at bottom
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Ctrl+Enter: save  |  Esc: cancel  |  Enter: newline",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_confirmation(frame: &mut Frame, state: &ConfirmState) {
    let area = frame.area();
    let popup_width = 50u16.min(area.width.saturating_sub(4));

    // calculate height based on content
    let rev_count = state.revs.len();
    let popup_height = (7 + rev_count.min(5)) as u16; // message + revs (max 5) + padding + buttons

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Confirm ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(popup_area);
    frame.render_widget(block.style(Style::default().bg(Color::Rgb(30, 20, 20))), popup_area);

    let mut lines = vec![
        Line::from(Span::styled(
            state.message.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    // show affected revisions (up to 5)
    for (i, rev) in state.revs.iter().take(5).enumerate() {
        lines.push(Line::from(format!("  {rev}")));
        if i == 4 && state.revs.len() > 5 {
            lines.push(Line::from(format!("  ... and {} more", state.revs.len() - 5)));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  Press "),
        Span::styled("y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" to confirm or "),
        Span::styled("n", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw(" to cancel"),
    ]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_diff(frame: &mut Frame, state: &super::app::DiffState, area: Rect) {
    let block = Block::default()
        .title(format!(" Diff: {} ", state.rev))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let viewport_height = inner.height as usize;
    let lines: Vec<Line> = state
        .lines
        .iter()
        .skip(state.scroll_offset)
        .take(viewport_height)
        .map(|dl| {
            // apply background tint for added/removed lines
            let bg = match dl.kind {
                DiffLineKind::Added => Some(Color::Rgb(0, 40, 0)),
                DiffLineKind::Removed => Some(Color::Rgb(40, 0, 0)),
                _ => None,
            };

            let spans: Vec<Span> = dl
                .spans
                .iter()
                .map(|s| {
                    let mut style = Style::default().fg(s.fg);
                    if let Some(bg_color) = bg {
                        style = style.bg(bg_color);
                    }
                    Span::styled(s.text.clone(), style)
                })
                .collect();

            Line::from(spans)
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_diff_pane(frame: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default()
        .title(" Diff ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let hint = Paragraph::new("Press D to view full diff")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, inner);
}

fn render_commit_details(
    node: &TreeNode,
    visual_depth: usize,
    stats: Option<&DiffStats>,
) -> Vec<Line<'static>> {
    let indent = "  ".repeat(visual_depth + 1);
    let dim = Style::default().fg(Color::DarkGray);
    let label_style = Style::default().fg(Color::Yellow);

    let author = if node.author_email.is_empty() {
        node.author_name.clone()
    } else {
        format!("{} <{}>", node.author_name, node.author_email)
    };

    let desc = if node.description.is_empty() {
        "(empty)".to_string()
    } else {
        node.description.clone()
    };

    let stats_str = match stats {
        Some(s) => format!(
            "{} file{}, +{} -{}",
            s.files_changed,
            if s.files_changed == 1 { "" } else { "s" },
            s.insertions,
            s.deletions
        ),
        None => "loading...".to_string(),
    };

    vec![
        Line::from(vec![
            Span::styled(format!("{indent}Change ID: "), label_style),
            Span::styled(node.change_id.clone(), dim),
        ]),
        Line::from(vec![
            Span::styled(format!("{indent}Author: "), label_style),
            Span::styled(author, dim),
        ]),
        Line::from(vec![
            Span::styled(format!("{indent}Date: "), label_style),
            Span::styled(node.timestamp.clone(), dim),
        ]),
        Line::from(vec![
            Span::styled(format!("{indent}Changes: "), label_style),
            Span::styled(format!("+{}", stats.map(|s| s.insertions).unwrap_or(0)), Style::default().fg(Color::Green)),
            Span::raw(" "),
            Span::styled(format!("-{}", stats.map(|s| s.deletions).unwrap_or(0)), Style::default().fg(Color::Red)),
            Span::styled(format!(" ({stats_str})"), dim),
        ]),
        Line::from(vec![
            Span::styled(format!("{indent}Description: "), label_style),
            Span::styled(desc, dim),
        ]),
    ]
}
