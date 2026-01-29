# JJ Tree TUI - Specification

## Overview

A ratatui-based TUI for jj that extends the existing `jj tree` command with interactive capabilities for managing commits. Launched via `cmd jj` (no arguments).

---

## Implementation Status

### Sprint 1: Foundation ✅ COMPLETE

- [x] Add ratatui/crossterm dependencies to Cargo.toml
- [x] Create basic TUI app structure with event loop (`app.rs`)
- [x] Port tree data loading from existing `tree()` function (`tree.rs`)
- [x] Render tree with cursor highlighting (`ui.rs`)
- [x] Navigation: j/k (up/down), g/G (top/bottom), @ (working copy)
- [x] Full mode toggle (f key)
- [x] Help dialog (? key)
- [x] Status bar with keybinding hints
- [x] Proper revset: `trunk() | descendants(roots(trunk()..@)) | @::`
- [x] Visual depth computation for non-full mode (collapsed view)

### Sprint 2: View Operations ✅ COMPLETE

- [x] **Diff viewing (D key)** - color-coded diff output
  - Added `syntect` dependency to Cargo.toml
  - Added `Mode::ViewingDiff` variant to `Mode` enum
  - Fetch diff via CLI: `cmd!(sh, "jj diff -r {rev}").read()?`
  - Color-coded diff lines: file headers (yellow), hunks (cyan), added (green), removed (red)
  - Scroll with j/k, page with d/u, top/bottom with g/G, close with Esc/q
- [x] **Commit details panel (Space key)** - inline expansion
  - Toggle expanded state via `TreeState.expanded_entry`
  - Shows change ID and description below selected commit
  - Render expanded details inline in tree view
- [x] **Scroll support (Ctrl+u/d)** - page navigation
  - Added Ctrl+u/Ctrl+d handlers for half-page scrolling
  - `page_up()` and `page_down()` methods on TreeState
  - Cursor stays visible via existing `update_scroll()` mechanism
- [x] **Multi-pane layout toggle (\\ key)** - tree + diff side-by-side
  - Added `split_view` field to App, toggled with `\` key
  - Horizontal split layout with 50/50 tree + diff pane
  - Right pane shows placeholder (press D for full diff view)

### Sprint 3: Edit Operations ✅ COMPLETE

- [x] Edit description inline (d key)
- [x] Edit working copy (e key) - `jj edit`
- [x] Multi-select mode (x to toggle, v for visual)
- [x] Abandon with preview (a key)
- [x] New commit (n key) - `jj new`
- [x] Commit changes (c key) - `jj commit`

### Sprint 4: Rebase Operations ✅ COMPLETE

- [x] Rebase mode entry (r for single, s for source+descendants)
- [x] Branch mode toggle (b key in rebase mode)
- [x] Quick rebase onto trunk (t/T keys)
- [x] Command preview in status bar + info popup
- [x] Conflict detection + undo support (u key)

### Sprint 5: Squash & Conflicts 🔲 NOT STARTED

- [ ] Squash into parent (q key)
- [ ] Squash into target (Q key)
- [ ] Conflict detection after operations
- [ ] Automatic new → resolve → squash flow
- [ ] Operation-based undo (u key)
- [ ] Operation log view (O key)

### Sprint 6: Polish 🔲 NOT STARTED

- [ ] Bookmark operations (m move, b create, B delete)
- [ ] Remote operations (F fetch, p push, P push all)
- [ ] Clipboard (y yank SHA, Y yank change id)
- [ ] Action menu popup (Enter key)
- [ ] Status indicators (conflicts ⚠, dirty ●, ahead/behind ↑↓)
- [ ] Configurable keybindings via keymap file

---

## Current Architecture

```
cmd/src/cmd/
├── jj.rs              # CLI commands, routes to TUI when no subcommand
├── jj_tui.rs          # Module entry point
└── jj_tui/
    ├── app.rs         # App struct, event loop, mode handling
    ├── tree.rs        # TreeNode, TreeState, VisibleEntry
    ├── ui.rs          # Ratatui rendering functions
    └── keybindings.rs # Stub for future keymap system (NOT wired up yet)
```

### Current Mode Enum (app.rs)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Help,
    ViewingDiff,  // Diff state stored in App.diff_state
    Editing,      // Description editing state in App.editing_state
    Confirming,   // Confirmation dialog state in App.confirm_state
    Selecting,    // Visual selection mode
    Rebasing,     // Rebase mode state in App.rebase_state
}
```

Mode state is stored separately in `App.*_state: Option<*State>` fields to keep Mode as Copy

### Keybindings Architecture

Keybindings are currently **hardcoded in app.rs** (see `handle_normal_key()`). The `keybindings.rs` file contains a `Key` struct stub but is not yet integrated. Future work will wire up configurable keymaps.

---

## Current Keybindings

### Normal Mode (Implemented)

| Key | Action | Status |
|-----|--------|--------|
| `j` / `↓` | Move cursor down | ✅ |
| `k` / `↑` | Move cursor up | ✅ |
| `Ctrl+d` | Page down | ✅ |
| `Ctrl+u` | Page up | ✅ |
| `g` | Jump to top | ✅ |
| `G` | Jump to bottom | ✅ |
| `@` | Jump to working copy | ✅ |
| `D` | View diff | ✅ |
| `Space` | Toggle commit details | ✅ |
| `\` | Toggle split view | ✅ |
| `f` | Toggle full mode | ✅ |
| `?` | Toggle help | ✅ |
| `q` / `Esc` | Quit | ✅ |
| `d` | Edit description | ✅ |
| `e` | Edit working copy | ✅ |
| `n` | New commit | ✅ |
| `c` | Commit changes | ✅ |
| `x` | Toggle selection | ✅ |
| `v` | Visual select mode | ✅ |
| `a` | Abandon selected | ✅ |
| `r` | Rebase single (-r) | ✅ |
| `s` | Rebase + descendants (-s) | ✅ |
| `t` | Quick rebase onto trunk | ✅ |
| `T` | Quick rebase tree onto trunk | ✅ |
| `u` | Undo last operation | ✅ |

### Rebase Mode (Implemented)

| Key | Action | Status |
|-----|--------|--------|
| `j` / `↓` | Move destination cursor down | ✅ |
| `k` / `↑` | Move destination cursor up | ✅ |
| `b` | Toggle branch mode | ✅ |
| `Enter` | Execute rebase | ✅ |
| `Esc` | Cancel rebase | ✅ |

### Diff View Mode (Implemented)

| Key | Action | Status |
|-----|--------|--------|
| `j` / `↓` | Scroll down | ✅ |
| `k` / `↑` | Scroll up | ✅ |
| `d` | Page down | ✅ |
| `u` | Page up | ✅ |
| `g` | Jump to top | ✅ |
| `G` | Jump to bottom | ✅ |
| `q` / `Esc` | Close diff view | ✅ |

### Planned Keybindings

See PLAN.md for full keybinding reference

---

## Dependencies

```toml
# TUI (currently in Cargo.toml)
ratatui = { version = "0.30", features = ["crossterm"] }
tui-tree-widget = "0.22"
unicode-width = "0.2"
smallvec = { version = "1.13", features = ["serde"] }

# Sprint 2 additions (added)
syntect = { version = "5.3", default-features = false, features = ["default-fancy"] }

# Future sprint additions
tui-textarea = "0.7"        # Multi-line text editing (Sprint 3)
arboard = "3.5"             # Clipboard (Sprint 6)
```

**Note:** `syntect` has been added to Cargo.toml (Sprint 2)

---

## Key Design Decisions

1. **Default to full mode**: TUI starts showing all commits like `cmd jj tree --full`
2. **Visual vs structural depth**: Non-full mode computes visual depths to collapse hidden commits
3. **ratatui 0.30**: Uses built-in `init()`/`restore()` with panic hooks
4. **crossterm via ratatui**: No need to add crossterm directly, use `ratatui::crossterm`
5. **CLI for mutations**: Use `jj` CLI for mutations, `jj-lib` for reads
6. **Simulated previews**: Calculate preview trees locally without running jj commands

---

## Syntax Highlighting

### Current Implementation

Uses **syntect** (regex-based, Sublime Text syntax definitions):
- ~40 languages supported (Rust, Python, JS/TS, Go, C/C++, etc.)
- Falls back to `bat` CLI for unsupported file extensions
- Falls back to plain text if bat unavailable

### Future: Migrate to Lumis

Consider switching to [lumis](https://github.com/leandrocp/lumis) for better language coverage:

| | syntect (current) | lumis (future) |
|---|---|---|
| Engine | Regex | Tree-sitter |
| Languages | ~40 | 70+ |
| Themes | ~20 | 120+ (Neovim) |
| Accuracy | Good | Better (AST-based) |

**Binary size impact:** Lumis bundles tree-sitter grammars, expect ~5-10MB increase.

**Ratatui integration work required:**
1. Lumis `TerminalBuilder` outputs ANSI escape codes as a `String`
2. Need to parse ANSI codes back into ratatui `Span`s with `Style`
3. Options:
   - Use `ansi-to-tui` crate to convert ANSI string → `Text`
   - Or request lumis add a "raw styles" output mode (returns `Vec<(Style, &str)>` like syntect)
4. Replace `parse_diff()` in `app.rs` to use lumis instead of syntect

**Migration checklist:**
- [ ] Add `lumis = "x.x"` to Cargo.toml
- [ ] Add `ansi-to-tui = "x.x"` for ANSI parsing (or implement custom parser)
- [ ] Update `parse_diff()` to use `TerminalBuilder` with language detection
- [ ] Remove syntect dependency
- [ ] Remove bat fallback (no longer needed)
- [ ] Test binary size delta
