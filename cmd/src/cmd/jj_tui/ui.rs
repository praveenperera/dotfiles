use super::app::{App, Mode};
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

    render_tree(frame, app, chunks[0]);
    render_status_bar(frame, app, chunks[1]);

    if matches!(app.mode, Mode::Help) {
        render_help(frame);
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

    let lines: Vec<Line> = app
        .tree
        .visible_nodes()
        .enumerate()
        .skip(scroll_offset)
        .take(viewport_height)
        .map(|(visible_idx, (_, node))| render_tree_line(node, visible_idx == app.tree.cursor))
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_tree_line(node: &TreeNode, is_selected: bool) -> Line<'static> {
    let indent = "  ".repeat(node.depth);

    let connector = if node.depth > 0 { "├── " } else { "" };

    let at_marker = if node.is_working_copy { "@ " } else { "" };

    let (prefix, suffix) = node
        .change_id
        .split_at(node.unique_prefix_len.min(node.change_id.len()));

    let name_spans = if node.bookmarks.is_empty() {
        vec![
            Span::raw(format!("{indent}{connector}{at_marker}(")),
            Span::styled(prefix.to_string(), Style::default().fg(Color::Magenta)),
            Span::styled(suffix.to_string(), Style::default().fg(Color::DarkGray)),
            Span::raw(")"),
        ]
    } else {
        let bookmark_str = node.bookmarks.join(" ");
        vec![
            Span::raw(format!("{indent}{connector}{at_marker}")),
            Span::styled(bookmark_str, Style::default().fg(Color::Cyan)),
        ]
    };

    let desc = if node.description.is_empty() {
        if node.is_working_copy {
            "(working copy)".to_string()
        } else {
            "(no description)".to_string()
        }
    } else {
        node.description.clone()
    };

    let desc_span = Span::styled(format!("  {desc}"), Style::default().fg(Color::DarkGray));

    let rev_suffix = if !node.bookmarks.is_empty() {
        vec![
            Span::raw("  "),
            Span::styled(prefix.to_string(), Style::default().fg(Color::Magenta)),
            Span::styled(suffix.to_string(), Style::default().fg(Color::DarkGray)),
        ]
    } else {
        vec![]
    };

    let mut spans = name_spans;
    spans.push(desc_span);
    spans.extend(rev_suffix);

    let mut line = Line::from(spans);
    if is_selected {
        line = line.style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        );
    }

    line
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.mode {
        Mode::Normal => "NORMAL",
        Mode::Help => "HELP",
    };

    let full_indicator = if app.tree.full_mode { " [FULL]" } else { "" };

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
        Mode::Normal => "j/k:nav  g/G:top/bottom  @:working  f:full  ?:help  q:quit",
        Mode::Help => "q/Esc:close",
    };

    let left = format!(" {mode_indicator}{full_indicator}{current_info}");
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
    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = 14u16.min(area.height.saturating_sub(4));

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
        Line::from("  j/↓     Move cursor down"),
        Line::from("  k/↑     Move cursor up"),
        Line::from("  g       Jump to top"),
        Line::from("  G       Jump to bottom"),
        Line::from("  @       Jump to working copy"),
        Line::from(""),
        Line::from(Span::styled(
            "View",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  f       Toggle full mode"),
        Line::from(""),
        Line::from(Span::styled(
            "General",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?       Toggle help"),
        Line::from("  q/Esc   Quit (or close help)"),
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
