use super::app::{App, BookmarkInputState, BookmarkSelectAction, BookmarkSelectState, ConfirmState, DiffLineKind, DiffStats, MessageKind, Mode, RebaseType, StatusMessage};
use super::tree::{BookmarkInfo, TreeNode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tui_popup::Popup;

struct PreviewEntry {
    original_index: usize,
    visual_depth: usize,
    is_source: bool,
    is_moving: bool,
    is_dest: bool,
}

/// Format bookmarks to fit within max_width, showing "+N" for overflow
/// Diverged bookmarks are marked with * suffix
fn format_bookmarks_truncated(bookmarks: &[BookmarkInfo], max_width: usize) -> String {
    if bookmarks.is_empty() {
        return String::new();
    }

    let format_bookmark = |b: &BookmarkInfo| {
        if b.is_diverged {
            format!("{}*", b.name)
        } else {
            b.name.clone()
        }
    };

    if bookmarks.len() == 1 {
        return format_bookmark(&bookmarks[0]);
    }

    let mut result = String::new();

    for (i, bm) in bookmarks.iter().enumerate() {
        let bm_display = format_bookmark(bm);
        let remaining = bookmarks.len() - i - 1;
        let suffix = if remaining > 0 { format!(" +{}", remaining) } else { String::new() };
        let candidate = if result.is_empty() {
            format!("{}{}", bm_display, suffix)
        } else {
            format!("{} {}{}", result, bm_display, suffix)
        };

        if candidate.len() <= max_width {
            if remaining == 0 {
                result = candidate;
            } else {
                // add this bookmark, continue to next
                if result.is_empty() {
                    result = bm_display;
                } else {
                    result = format!("{} {}", result, bm_display);
                }
            }
        } else {
            // doesn't fit, stop here and add +N
            let overflow = bookmarks.len() - i;
            if result.is_empty() {
                return format!("{} +{}", format_bookmark(&bookmarks[0]), overflow - 1);
            }
            return format!("{} +{}", result, overflow);
        }
    }
    result
}

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
        Mode::Normal | Mode::Help | Mode::Selecting | Mode::Confirming
        | Mode::Rebasing | Mode::MovingBookmark | Mode::BookmarkInput | Mode::BookmarkSelect | Mode::Squashing => {
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

    if let Some(ref state) = app.confirm_state {
        if matches!(app.mode, Mode::Confirming) {
            render_confirmation(frame, state);
        }
    }

    if let Some(ref state) = app.bookmark_input_state {
        if matches!(app.mode, Mode::BookmarkInput) {
            render_bookmark_input(frame, state);
        }
    }

    if let Some(ref state) = app.bookmark_select_state {
        if matches!(app.mode, Mode::BookmarkSelect) {
            render_bookmark_select(frame, state);
        }
    }

    // render toast notification last (on top of everything)
    if let Some(ref msg) = app.status_message {
        if std::time::Instant::now() < msg.expires {
            render_toast(frame, msg);
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

    // check if we're in rebase mode - use preview rendering
    if let (Mode::Rebasing, Some(ref state)) = (&app.mode, &app.rebase_state) {
        let preview = build_rebase_preview(app, state.dest_cursor, &state.source_rev, state.rebase_type);
        render_tree_with_preview(frame, app, inner, viewport_height, scroll_offset, &preview);
    } else {
        // normal rendering (including MovingBookmark and Squashing modes)
        render_tree_normal(frame, app, inner, viewport_height, scroll_offset);
    }
}

/// Normal tree rendering (non-rebase mode)
fn render_tree_normal(frame: &mut Frame, app: &App, area: Rect, viewport_height: usize, scroll_offset: usize) {
    let mut lines: Vec<Line> = Vec::new();
    let mut line_count = 0;

    // get bookmark move info if in that mode
    let bm_move_info = if let (Mode::MovingBookmark, Some(ref state)) = (&app.mode, &app.moving_bookmark_state) {
        Some((state.bookmark_name.clone(), state.dest_cursor))
    } else {
        None
    };

    // get squash info if in that mode
    let squash_info = if let (Mode::Squashing, Some(ref state)) = (&app.mode, &app.squash_state) {
        Some((state.source_rev.clone(), state.dest_cursor))
    } else {
        None
    };

    for (visible_idx, entry) in app.tree.visible_nodes().enumerate().skip(scroll_offset) {
        if line_count >= viewport_height {
            break;
        }

        let node = app.tree.get_node(entry);

        // determine cursor and markers based on mode
        let (is_cursor, is_source, is_dest) = if let Some((ref bm_name, dest_cursor)) = bm_move_info {
            let is_source = node.has_bookmark(bm_name);
            let is_dest = visible_idx == dest_cursor && !is_source;
            (visible_idx == dest_cursor, is_source, is_dest)
        } else if let Some((ref source_rev, dest_cursor)) = squash_info {
            let is_source = node.change_id == *source_rev;
            let is_dest = visible_idx == dest_cursor && !is_source;
            (visible_idx == dest_cursor, is_source, is_dest)
        } else {
            (visible_idx == app.tree.cursor, false, false)
        };
        let is_multi_selected = app.tree.selected.contains(&visible_idx);

        lines.push(render_tree_line_with_markers(
            node,
            entry.visual_depth,
            is_cursor,
            is_multi_selected,
            is_source,
            is_dest,
            squash_info.is_some(),
        ));
        line_count += 1;

        // render expanded details
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
    frame.render_widget(paragraph, area);
}

/// Render a tree line with source/dest markers (for bookmark move and squash modes)
fn render_tree_line_with_markers(
    node: &TreeNode,
    visual_depth: usize,
    is_cursor: bool,
    is_multi_selected: bool,
    is_source: bool,
    is_dest: bool,
    is_squash_mode: bool,
) -> Line<'static> {
    let indent = "  ".repeat(visual_depth);
    let connector = if visual_depth > 0 { "├── " } else { "" };
    let at_marker = if node.is_working_copy { "@ " } else { "" };
    let selection_marker = if is_multi_selected { "[x] " } else { "" };

    let (prefix, suffix) = node
        .change_id
        .split_at(node.unique_prefix_len.min(node.change_id.len()));

    let mut spans = Vec::new();

    // change_id color: yellow for source, normal magenta otherwise
    let prefix_color = if is_source { Color::Yellow } else { Color::Magenta };

    spans.extend([
        Span::raw(format!("{indent}{connector}{selection_marker}{at_marker}(")),
        Span::styled(prefix.to_string(), Style::default().fg(prefix_color)),
        Span::styled(suffix.to_string(), Style::default().fg(Color::DarkGray)),
        Span::raw(")"),
    ]);

    if !node.bookmarks.is_empty() {
        let bookmark_str = format_bookmarks_truncated(&node.bookmarks, 30);
        spans.push(Span::raw(" "));
        let bm_color = if is_source { Color::Yellow } else { Color::Cyan };
        spans.push(Span::styled(bookmark_str, Style::default().fg(bm_color)));
    }

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

    // add source/dest markers on the right
    if is_source {
        let marker = if is_squash_mode { "  ← src" } else { "  ← bm" };
        spans.push(Span::styled(marker, Style::default().fg(Color::Yellow)));
    } else if is_dest {
        spans.push(Span::styled("  ← dest", Style::default().fg(Color::Green)));
    }

    let mut line = Line::from(spans);

    // apply styling based on state
    if is_cursor {
        line = line.style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        );
    } else if is_source {
        // highlight source entry
        line = line.style(Style::default().bg(Color::Rgb(50, 50, 30)));
    } else if is_multi_selected {
        line = line.style(Style::default().bg(Color::Rgb(40, 50, 40)));
    }

    line
}

/// Build the preview list showing how the tree will look after rebase
fn build_rebase_preview(app: &App, dest_cursor: usize, source_rev: &str, rebase_type: RebaseType) -> Vec<PreviewEntry> {
    let mut preview = Vec::new();

    // find source index
    let mut source_idx = None;
    let mut source_struct_depth = 0usize;
    let mut source_visual_depth = 0usize;

    for (idx, entry) in app.tree.visible_entries.iter().enumerate() {
        let node = &app.tree.nodes[entry.node_index];
        if node.change_id == source_rev {
            source_idx = Some(idx);
            source_struct_depth = node.depth;
            source_visual_depth = entry.visual_depth;
            break;
        }
    }

    // get destination visual depth
    let dest_visual_depth = app.tree.visible_entries
        .get(dest_cursor)
        .map(|e| e.visual_depth)
        .unwrap_or(0);

    // for 'r' mode: source moves to after dest, entries between them shift down
    if rebase_type == RebaseType::Single {
        let source_index = source_idx.unwrap_or(0);

        for (idx, entry) in app.tree.visible_entries.iter().enumerate() {
            // skip source at its original position - it will be inserted after dest
            if idx == source_index {
                continue;
            }

            let is_dest = idx == dest_cursor;

            // entries between dest (exclusive) and source (exclusive) shift down by 1 depth
            let depth = if idx > dest_cursor && idx < source_index {
                entry.visual_depth + 1
            } else {
                entry.visual_depth
            };

            preview.push(PreviewEntry {
                original_index: idx,
                visual_depth: depth,
                is_source: false,
                is_moving: false,
                is_dest,
            });

            // insert source right after dest
            if is_dest {
                preview.push(PreviewEntry {
                    original_index: source_index,
                    visual_depth: dest_visual_depth + 1,
                    is_source: true,
                    is_moving: true,
                    is_dest: false,
                });
            }
        }
        return preview;
    }

    // for 's' mode: source + descendants move together after dest
    let mut moving_indices = std::collections::HashSet::new();
    let mut in_source_tree = false;

    for (idx, entry) in app.tree.visible_entries.iter().enumerate() {
        let node = &app.tree.nodes[entry.node_index];
        if node.change_id == source_rev {
            moving_indices.insert(idx);
            in_source_tree = true;
        } else if in_source_tree {
            if node.depth > source_struct_depth {
                moving_indices.insert(idx);
            } else {
                break;
            }
        }
    }

    let moving_entries: Vec<(usize, usize)> = app.tree.visible_entries
        .iter()
        .enumerate()
        .filter(|(idx, _)| moving_indices.contains(idx))
        .map(|(idx, entry)| (idx, entry.visual_depth))
        .collect();

    // find the first moving index (source) for determining entries that need shifting
    let first_moving_idx = moving_indices.iter().min().copied().unwrap_or(0);
    let num_moving = moving_indices.len();

    for (idx, entry) in app.tree.visible_entries.iter().enumerate() {
        if moving_indices.contains(&idx) {
            continue;
        }

        let is_dest = idx == dest_cursor;

        // entries between dest and source shift down by the size of the moving stack
        let depth = if idx > dest_cursor && idx < first_moving_idx {
            entry.visual_depth + num_moving
        } else {
            entry.visual_depth
        };

        preview.push(PreviewEntry {
            original_index: idx,
            visual_depth: depth,
            is_source: false,
            is_moving: false,
            is_dest,
        });

        // after destination, insert moving entries with adjusted visual depths
        if is_dest {
            for (mov_idx, mov_visual_depth) in &moving_entries {
                let is_source_entry = source_idx == Some(*mov_idx);
                // source becomes child of dest, descendants keep relative depth
                let new_depth = dest_visual_depth + 1 + mov_visual_depth.saturating_sub(source_visual_depth);

                preview.push(PreviewEntry {
                    original_index: *mov_idx,
                    visual_depth: new_depth,
                    is_source: is_source_entry,
                    is_moving: true,
                    is_dest: false,
                });
            }
        }
    }

    preview
}

/// Render tree with rebase preview
fn render_tree_with_preview(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    viewport_height: usize,
    scroll_offset: usize,
    preview: &[PreviewEntry],
) {
    let mut lines: Vec<Line> = Vec::new();

    // find which preview index contains the destination cursor
    let cursor_preview_idx = preview.iter().position(|p| p.is_dest);

    for (line_count, (preview_idx, entry)) in preview.iter().enumerate().skip(scroll_offset).enumerate() {
        if line_count >= viewport_height {
            break;
        }

        let orig_entry = &app.tree.visible_entries[entry.original_index];
        let node = &app.tree.nodes[orig_entry.node_index];

        // cursor is on the destination entry
        let is_cursor = cursor_preview_idx == Some(preview_idx);

        lines.push(render_tree_line_rebase(
            node,
            entry.visual_depth,
            is_cursor,
            false, // is_multi_selected - not relevant in rebase mode
            entry.is_source,
            entry.is_moving,
            entry.is_dest,
        ));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

fn render_tree_line_rebase(
    node: &TreeNode,
    visual_depth: usize,
    is_cursor: bool,
    is_multi_selected: bool,
    is_source: bool,
    is_moving: bool,
    is_dest: bool,
) -> Line<'static> {
    let indent = "  ".repeat(visual_depth);
    let connector = if visual_depth > 0 { "├── " } else { "" };
    let at_marker = if node.is_working_copy { "@ " } else { "" };
    let selection_marker = if is_multi_selected { "[x] " } else { "" };

    let (prefix, suffix) = node
        .change_id
        .split_at(node.unique_prefix_len.min(node.change_id.len()));

    let mut spans = Vec::new();

    // change_id color: yellow for moving entries, magenta for normal
    let prefix_color = if is_moving { Color::Yellow } else { Color::Magenta };

    spans.extend([
        Span::raw(format!("{indent}{connector}{selection_marker}{at_marker}(")),
        Span::styled(prefix.to_string(), Style::default().fg(prefix_color)),
        Span::styled(suffix.to_string(), Style::default().fg(Color::DarkGray)),
        Span::raw(")"),
    ]);

    if !node.bookmarks.is_empty() {
        let bookmark_str = format_bookmarks_truncated(&node.bookmarks, 30);
        spans.push(Span::raw(" "));
        spans.push(Span::styled(bookmark_str, Style::default().fg(Color::Cyan)));
    }

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

    // add rebase markers on the right
    if is_source {
        spans.push(Span::styled("  ← src", Style::default().fg(Color::Yellow)));
    } else if is_dest && !is_moving {
        spans.push(Span::styled("  ← dest", Style::default().fg(Color::Cyan)));
    } else if is_moving && !is_source {
        spans.push(Span::styled("  ↳", Style::default().fg(Color::Yellow)));
    }

    let mut line = Line::from(spans);

    // apply styling based on state
    if is_cursor {
        line = line.style(
            Style::default()
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD),
        );
    } else if is_moving {
        // highlight moving entries (source and descendants)
        line = line.style(Style::default().bg(Color::Rgb(50, 50, 30)));
    } else if is_multi_selected {
        line = line.style(Style::default().bg(Color::Rgb(40, 50, 40)));
    }

    line
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.mode {
        Mode::Normal => "NORMAL",
        Mode::Help => "HELP",
        Mode::ViewingDiff => "DIFF",
        Mode::Confirming => "CONFIRM",
        Mode::Selecting => "SELECT",
        Mode::Rebasing => {
            if let Some(ref state) = app.rebase_state {
                if state.rebase_type == RebaseType::Single {
                    "REBASE -r"
                } else {
                    "REBASE -s"
                }
            } else {
                "REBASE"
            }
        }
        Mode::MovingBookmark => "MOVE BOOKMARK",
        Mode::BookmarkInput => "BOOKMARK",
        Mode::BookmarkSelect => "SELECT BM",
        Mode::Squashing => "SQUASH",
    };

    let full_indicator = if app.tree.full_mode { " [FULL]" } else { "" };
    let split_indicator = if app.split_view { " [SPLIT]" } else { "" };

    // show pending key when waiting for second key in sequence
    let pending_indicator = match app.pending_key {
        Some('g') => " g-",
        Some('z') => " z-",
        _ => "",
    };

    // show selection count when there are selected items
    let selection_indicator = if !app.tree.selected.is_empty() {
        format!(" [{}sel]", app.tree.selected.len())
    } else {
        String::new()
    };

    // in rebase mode, show source→dest instead of current node
    let current_info = if let (Mode::Rebasing, Some(ref state)) = (&app.mode, &app.rebase_state) {
        let dest_name = app
            .tree
            .visible_entries
            .get(state.dest_cursor)
            .map(|e| {
                let node = &app.tree.nodes[e.node_index];
                if node.bookmarks.is_empty() {
                    node.change_id.chars().take(8).collect::<String>()
                } else {
                    node.bookmark_names().join(" ")
                }
            })
            .unwrap_or_else(|| "?".to_string());
        let src_short: String = state.source_rev.chars().take(8).collect();
        format!(" | {src_short}→{dest_name}")
    } else if let (Mode::MovingBookmark, Some(ref state)) = (&app.mode, &app.moving_bookmark_state) {
        let dest_name = app
            .tree
            .visible_entries
            .get(state.dest_cursor)
            .map(|e| {
                let node = &app.tree.nodes[e.node_index];
                node.change_id.chars().take(8).collect::<String>()
            })
            .unwrap_or_else(|| "?".to_string());
        let bm_name: String = state.bookmark_name.chars().take(12).collect();
        format!(" | {bm_name}→{dest_name}")
    } else if let (Mode::Squashing, Some(ref state)) = (&app.mode, &app.squash_state) {
        let dest_name = app
            .tree
            .visible_entries
            .get(state.dest_cursor)
            .map(|e| {
                let node = &app.tree.nodes[e.node_index];
                if node.bookmarks.is_empty() {
                    node.change_id.chars().take(8).collect::<String>()
                } else {
                    node.bookmark_names().join(" ")
                }
            })
            .unwrap_or_else(|| "?".to_string());
        let src_short: String = state.source_rev.chars().take(8).collect();
        format!(" | {src_short}→{dest_name}")
    } else {
        app.tree
            .current_node()
            .map(|n| {
                let name = if n.bookmarks.is_empty() {
                    n.change_id.clone()
                } else {
                    n.bookmark_names().join(" ")
                };
                format!(" | {name}")
            })
            .unwrap_or_default()
    };

    let hints = match app.mode {
        Mode::Normal => {
            if !app.tree.selected.is_empty() {
                "a:abandon  x:toggle  Esc:clear"
            } else if app.current_has_bookmark() {
                "p:push m:move-bm B:del-bm r:rebase ?:help q:quit"
            } else {
                "b:new-bm r/s:rebase t:trunk d:desc gi:import ge:export ?:help q:quit"
            }
        }
        Mode::Help => "q/Esc:close",
        Mode::ViewingDiff => "j/k:scroll  d/u:page  zt/zb:top/bottom  q/Esc:close",
        Mode::Confirming => "y/Enter:yes  n/Esc:no",
        Mode::Selecting => "j/k:extend  a:abandon  Esc:exit",
        Mode::Rebasing => {
            if let Some(ref state) = app.rebase_state {
                if state.allow_branches {
                    "j/k:dest  b:inline  Enter:run  Esc:cancel"
                } else {
                    "j/k:dest  b:branch  Enter:run  Esc:cancel"
                }
            } else {
                "j/k:dest  b:toggle  Enter:run  Esc:cancel"
            }
        }
        Mode::MovingBookmark => "j/k:dest  Enter:run  Esc:cancel",
        Mode::BookmarkInput => "Enter:confirm  Esc:cancel",
        Mode::BookmarkSelect => "j/k:navigate  Enter:select  Esc:cancel",
        Mode::Squashing => "j/k:dest  Enter:run  Esc:cancel",
    };

    let left = format!(" {mode_indicator}{full_indicator}{split_indicator}{pending_indicator}{selection_indicator}{current_info}");
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
    let popup_height = 50u16.min(area.height.saturating_sub(4));

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
        Line::from("  z t       Jump to top"),
        Line::from("  z b       Jump to bottom"),
        Line::from("  z z       Center current line"),
        Line::from("  @         Jump to working copy"),
        Line::from(""),
        Line::from(Span::styled(
            "View",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  D         View diff"),
        Line::from("  Tab       Toggle commit details"),
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
            "Rebase",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  r         Rebase single (-r)"),
        Line::from("  s         Rebase + descendants (-s)"),
        Line::from("  t         Quick rebase onto trunk"),
        Line::from("  T         Quick rebase tree onto trunk"),
        Line::from("  Q         Squash into target"),
        Line::from("  u         Undo last operation"),
        Line::from(""),
        Line::from(Span::styled(
            "Bookmarks & Git",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  p         Push current bookmark"),
        Line::from("  m         Move bookmark"),
        Line::from("  b         Create bookmark"),
        Line::from("  B         Delete bookmark"),
        Line::from("  g i       Git import"),
        Line::from("  g e       Git export"),
        Line::from(""),
        Line::from(Span::styled(
            "General",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?         Toggle help"),
        Line::from("  q         Quit"),
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

fn render_bookmark_input(frame: &mut Frame, state: &BookmarkInputState) {
    let area = frame.area();
    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = 7u16;

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let title = if state.deleting {
        " Delete Bookmark "
    } else {
        " Create Bookmark "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if state.deleting { Color::Red } else { Color::Cyan }));

    let inner = block.inner(popup_area);
    let bg_color = if state.deleting { Color::Rgb(30, 20, 20) } else { Color::Rgb(20, 20, 30) };
    frame.render_widget(block.style(Style::default().bg(bg_color)), popup_area);

    // render text with cursor
    let before = &state.name[..state.cursor];
    let cursor_char = state.name.get(state.cursor..).and_then(|s| s.chars().next());
    let after = if let Some(c) = cursor_char {
        &state.name[state.cursor + c.len_utf8()..]
    } else {
        ""
    };
    let cursor_display = cursor_char.unwrap_or(' ');

    let input_line = Line::from(vec![
        Span::styled("Name: ", Style::default().fg(Color::Yellow)),
        Span::raw(before.to_string()),
        Span::styled(
            cursor_display.to_string(),
            Style::default().bg(Color::White).fg(Color::Black),
        ),
        Span::raw(after.to_string()),
    ]);

    let target_short: String = state.target_rev.chars().take(8).collect();
    let target_line = Line::from(vec![
        Span::styled("At: ", Style::default().fg(Color::Yellow)),
        Span::styled(target_short, Style::default().fg(Color::DarkGray)),
    ]);

    let help_text = if state.deleting {
        "Enter: delete  |  Esc: cancel"
    } else {
        "Enter: create  |  Esc: cancel"
    };

    let lines = vec![
        input_line,
        Line::from(""),
        target_line,
        Line::from(""),
        Line::from(Span::styled(help_text, Style::default().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_bookmark_select(frame: &mut Frame, state: &BookmarkSelectState) {
    let area = frame.area();
    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = (6 + state.bookmarks.len().min(10)) as u16;

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let (title, border_color, bg_color) = match state.action {
        BookmarkSelectAction::Move => (
            " Select Bookmark to Move ",
            Color::Cyan,
            Color::Rgb(20, 20, 30),
        ),
        BookmarkSelectAction::Delete => (
            " Select Bookmark to Delete ",
            Color::Red,
            Color::Rgb(30, 20, 20),
        ),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(popup_area);
    frame.render_widget(block.style(Style::default().bg(bg_color)), popup_area);

    let mut lines: Vec<Line> = Vec::new();

    // show revision context
    let rev_short: String = state.target_rev.chars().take(8).collect();
    lines.push(Line::from(vec![
        Span::styled("At: ", Style::default().fg(Color::Yellow)),
        Span::styled(rev_short, Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(""));

    for (i, bookmark) in state.bookmarks.iter().enumerate() {
        let marker = if i == state.selected_index { "> " } else { "  " };
        let style = if i == state.selected_index {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(format!("{marker}{bookmark}"), style)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "j/k: navigate | Enter: select | Esc: cancel",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn render_toast(frame: &mut Frame, msg: &StatusMessage) {
    let color = match msg.kind {
        MessageKind::Info => Color::Blue,
        MessageKind::Success => Color::Green,
        MessageKind::Warning => Color::Yellow,
        MessageKind::Error => Color::Red,
    };

    let popup = Popup::new(msg.text.clone())
        .style(Style::default().fg(color).bg(Color::Rgb(30, 30, 40)));

    frame.render_widget(popup, frame.area());
}
