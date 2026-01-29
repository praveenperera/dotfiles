# JJ Tree TUI - Comprehensive Implementation Plan

## Overview

Building a ratatui-based TUI for jj that extends the existing `jj tree` command with interactive capabilities for managing commits.

---

## Keybindings Reference

All keybindings in one place. Edit this section to change bindings.

> **Architecture Note:** Keybindings should be defined in a single data structure
> (`Keymap`) that can be serialized/deserialized. This enables future config file
> customization (e.g., `~/.config/jj-tui/keymap.toml`). The implementation should:
> 1. Define all defaults in one place (see `keybindings.rs` below)
> 2. Use a `Keymap` struct that maps `(Mode, KeyEvent) -> Action`
> 3. Load user overrides from config file at startup
> 4. Never hardcode key checks scattered throughout the codebase
>
> ```rust
> // keybindings.rs - Single source of truth
> use crossterm::event::{KeyCode, KeyModifiers, KeyEvent};
>
> /// Wrapper for KeyEvent that's easy to construct and serialize
> #[derive(Hash, Eq, PartialEq, Clone, Serialize, Deserialize)]
> pub struct Key {
>     pub code: KeyCode,
>     pub modifiers: KeyModifiers,
> }
>
> impl Key {
>     pub fn new(code: KeyCode) -> Self {
>         Self { code, modifiers: KeyModifiers::NONE }
>     }
>     pub fn ctrl(code: KeyCode) -> Self {
>         Self { code, modifiers: KeyModifiers::CONTROL }
>     }
>     pub fn shift(code: KeyCode) -> Self {
>         Self { code, modifiers: KeyModifiers::SHIFT }
>     }
>     pub fn alt(code: KeyCode) -> Self {
>         Self { code, modifiers: KeyModifiers::ALT }
>     }
> }
>
> // Convenience macros
> macro_rules! key {
>     (Ctrl+$c:literal) => { Key::ctrl(KeyCode::Char($c)) };
>     (Ctrl+$k:ident) => { Key::ctrl(KeyCode::$k) };
>     (Shift+$c:literal) => { Key::shift(KeyCode::Char($c)) };
>     (Alt+$c:literal) => { Key::alt(KeyCode::Char($c)) };
>     ($c:literal) => { Key::new(KeyCode::Char($c)) };
>     ($k:ident) => { Key::new(KeyCode::$k) };
> }
>
> #[derive(Serialize, Deserialize)]
> pub struct Keymap {
>     pub global: HashMap<Key, Action>,
>     pub normal: HashMap<Key, Action>,
>     pub rebase: HashMap<Key, Action>,
>     pub diff_view: HashMap<Key, Action>,
>     // ... other modes
> }
>
> impl Default for Keymap {
>     fn default() -> Self {
>         Self {
>             global: hashmap! {
>                 key!(Ctrl+'c') => Action::Quit,
>                 key!('?') => Action::ToggleHelp,
>                 key!(Esc) => Action::Cancel,
>             },
>             normal: hashmap! {
>                 key!('j') => Action::MoveDown,
>                 key!('k') => Action::MoveUp,
>                 key!(Enter) => Action::OpenActionMenu,
>                 key!('D') => Action::ViewDiff,  // Shift+d
>                 key!(Ctrl+'r') => Action::Refresh,
>                 // ... defined from tables below
>             },
>             // ...
>         }
>     }
> }
>
> // Config file format (keymap.toml):
> // [normal]
> // "j" = "MoveDown"
> // "ctrl+c" = "Quit"
> // "ctrl+r" = "Refresh"
> // "g g" = "GoToTop"        # multi-key sequence
> // "g b" = "GoToBookmark"   # another sequence starting with g
> ```
>
> **Multi-key sequences** (like vim's `g g`, `d d`, or `<leader>x`):
>
> ```rust
> use smallvec::SmallVec;
>
> /// A key sequence - typically 1-2 keys, but can be longer
> /// SmallVec avoids heap allocation for common single/double key bindings
> #[derive(Hash, Eq, PartialEq, Clone, Serialize, Deserialize)]
> pub struct KeySeq(pub SmallVec<[Key; 2]>);
>
> impl KeySeq {
>     pub fn single(k: Key) -> Self { Self(smallvec![k]) }
>     pub fn multi(keys: impl IntoIterator<Item = Key>) -> Self {
>         Self(keys.into_iter().collect())
>     }
> }
>
> // Convenience macro for defining sequences
> macro_rules! seq {
>     ($key:tt) => { KeySeq::single(key!($key)) };
>     ($($key:tt),+) => {
>         KeySeq(smallvec![$(key!($key)),+])
>     };
> }
>
> // Usage:
> // seq!('j')           -> single key (no allocation)
> // seq!('g', 'g')      -> "g g" (no allocation, fits in SmallVec)
> // seq!('g', 'b')      -> "g b"
> // seq!(Ctrl+'x', 's') -> "ctrl+x s"
>
> pub struct Keymap {
>     pub normal: HashMap<KeySeq, Action>,
>     // ...
> }
>
> /// Track pending keys for multi-key sequences
> pub struct KeyState {
>     pub pending: Vec<Key>,      // Keys pressed so far
>     pub timeout: Option<Instant>, // Clear after 1s of no input
> }
>
> impl KeyState {
>     pub fn handle_key(&mut self, key: Key, keymap: &Keymap, mode: &Mode) -> KeyResult {
>         self.pending.push(key);
>
>         // Check for exact match
>         if let Some(action) = keymap.lookup(&self.pending, mode) {
>             self.pending.clear();
>             return KeyResult::Action(action);
>         }
>
>         // Check if any sequence starts with these keys (prefix match)
>         if keymap.has_prefix(&self.pending, mode) {
>             self.timeout = Some(Instant::now() + Duration::from_secs(1));
>             return KeyResult::Pending; // Show "g-" in status bar
>         }
>
>         // No match and no prefix - invalid sequence
>         self.pending.clear();
>         KeyResult::None
>     }
> }
>
> // Example sequences:
> // "g g" -> GoToTop
> // "g G" -> GoToBottom
> // "g @" -> GoToWorkingCopy
> // "g b" -> GoToBookmark (prompt for name)
> // "d d" -> DeleteLine (abandon current)
> // "z z" -> CenterCursor
> ```
>
> When a prefix is pending, show it in the status bar: `g-` waiting for next key.

### Global (All Modes)

| Key | Action |
|-----|--------|
| `Ctrl+c` | Quit (press twice) |
| `?` | Toggle help pane |
| `Esc` | Cancel / close / back to Normal |

### Normal Mode

**Navigation:**
| Key | Action | Command |
|-----|--------|---------|
| `j` / `↓` | Move down | - |
| `k` / `↑` | Move up | - |
| `h` | Jump to left sibling stack | - |
| `l` | Jump to right sibling stack | - |
| `g` | Go to top (trunk) | - |
| `G` | Go to bottom | - |
| `@` | Go to working copy | - |
| `f` | Toggle full mode (show/hide non-bookmarked) | - |

**View:**
| Key | Action | Command |
|-----|--------|---------|
| `Space` | Show/hide commit details | - |
| `Enter` | Open action menu | - |
| `D` | View diff (syntax highlighted) | `jj diff -r {rev}` |

**Create:**
| Key | Action | Command |
|-----|--------|---------|
| `n` | New commit | `jj new` |
| `c` | Commit changes | `jj commit` |

**Edit:**
| Key | Action | Command |
|-----|--------|---------|
| `e` | Edit working copy | `jj edit {rev}` |
| `d` | Edit description (inline) | `jj desc -r {rev}` |

**Rebase (enter mode):**
| Key | Action | Command |
|-----|--------|---------|
| `r` | Rebase single revision | `jj rebase -r` |
| `s` | Rebase with descendants | `jj rebase -s` |
| `t` | Quick rebase onto trunk | `jj rebase -r @ -o trunk()` |
| `T` | Quick rebase tree onto trunk | `jj rebase -s @ -o trunk()` |

**Actions:**
| Key | Action | Command |
|-----|--------|---------|
| `q` | Squash into parent | `jj squash` |
| `x` | Mark/unmark for action | - |
| `a` | Abandon marked (or current) | `jj abandon {rev}` |
| `u` | Undo last operation | `jj undo` |
| `O` | Open operation log | `jj op log` |

**Bookmarks:**
| Key | Action | Command |
|-----|--------|---------|
| `m` | Move bookmark (if present) | `jj bookmark set` |
| `b` | Create new bookmark | `jj bookmark create` |
| `B` | Delete bookmark | `jj bookmark delete` |

**Remote:**
| Key | Action | Command |
|-----|--------|---------|
| `F` | Fetch from remote | `jj git fetch` |
| `p` | Push bookmark | `jj git push -b` |
| `P` | Push all bookmarks | `jj git push --all` |

**Clipboard:**
| Key | Action |
|-----|--------|
| `y` | Yank git SHA |
| `Y` | Yank change id |

**Layout:**
| Key | Action |
|-----|--------|
| `\` | Toggle multi-pane layout |
| `Tab` | Switch pane focus |

### Rebase Mode (after `r` or `s`)

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate to destination |
| `h` / `l` | Move between stacks |
| `b` | Toggle branch mode (default: OFF for clean inline) |
| `Enter` | Confirm rebase |
| `Esc` | Cancel |

### Bookmark Move Mode (after `m`)

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate to destination |
| `h` / `l` | Move between stacks |
| `Enter` | Drop bookmark here |
| `Esc` | Cancel |

### Action Menu Mode (after `Enter`)

| Key | Action |
|-----|--------|
| `j` / `k` | Select action |
| `Enter` | Execute selected action |
| Any key | Execute action directly |
| `Esc` | Close menu |

### Diff View Mode (after `D`)

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll down/up |
| `d` / `u` | Page down/up |
| `g` / `G` | Top/bottom |
| `Esc` | Close |

### Operation Log Mode (after `O`)

| Key | Action |
|-----|--------|
| `j` / `k` | Select operation |
| `Enter` | View operation details |
| `u` | Undo to selected operation |
| `Esc` | Close |

### Editing Description Mode (after `d`)

| Key | Action |
|-----|--------|
| `Enter` | Save description |
| `Esc` | Cancel |
| (text input) | Edit text |

### Verify Mismatch Mode (after rebase if result ≠ preview)

| Key | Action |
|-----|--------|
| `u` | Undo to before the move |
| `k` | Keep the result anyway |

---

## Phase 1: Foundation & Core Architecture

### 1.1 Project Structure

Using `module_name.rs` pattern (no `mod.rs` files):

```
cmd/src/
├── cmd/
│   ├── jj.rs              # Existing (keep CLI commands)
│   ├── jj_tui.rs          # Module entry point (declares submodules)
│   └── jj_tui/            # Submodules directory
│       ├── app.rs         # Application state & event loop
│       ├── ui.rs          # Ratatui rendering
│       ├── tree.rs        # Tree data model
│       ├── actions.rs     # JJ operations (rebase, squash, etc.)
│       ├── preview.rs     # Preview state management
│       ├── conflict.rs    # Conflict resolution flow
│       ├── keybindings.rs # Key handling
│       ├── widgets.rs     # Widgets module entry point
│       └── widgets/       # Widget submodules
│           ├── tree_view.rs
│           ├── diff_view.rs
│           ├── preview_panel.rs
│           └── help_dialog.rs
└── jj_lib_helpers.rs      # Extend with new operations
```

**Module declaration example (`jj_tui.rs`):**
```rust
mod app;
mod ui;
mod tree;
mod actions;
mod preview;
mod conflict;
mod keybindings;
mod widgets;

pub use app::App;
// re-export public API
```

### 1.2 Dependencies to Add

```toml
# Cargo.toml additions

# Core TUI - ratatui 0.30.0 (Dec 2025) includes automatic panic hooks
ratatui = { version = "0.30", features = ["crossterm"] }

# Widgets - all actively maintained (Jan 2026)
tui-textarea = "0.7"        # Multi-line text editing
tui-tree-widget = "0.22"    # Tree rendering with collapse/expand
tui-widgets = "0.5"         # Scrollview, scrollbar (by ratatui maintainer)

# Clipboard - maintained by 1Password, enable Wayland support
arboard = { version = "3.5", features = ["wayland-data-control"] }

# Syntax highlighting - pure Rust regex engine
syntect = { version = "5.3", default-features = false, features = ["default-fancy"] }

# Utilities
unicode-width = "0.2"       # Text width calculations for CJK support
smallvec = { version = "1.13", features = ["serde"] }  # Stack-allocated small vectors

# Error handling
color-eyre = "0.6"          # Rich error context (already in project)
```

**Dependency Rationale:**

| Crate | Version | Maintained By | Notes |
|-------|---------|---------------|-------|
| ratatui | 0.30.0 | ratatui org | Major Dec 2025 release, built-in panic hooks |
| crossterm | 0.29.0 | crossterm-rs | Bundled with ratatui, best cross-platform |
| tui-textarea | 0.7.0 | rhysd | Standard for multi-line editing |
| tui-tree-widget | 0.22 | EdJoPaTo | Best tree widget for ratatui |
| tui-widgets | 0.5.0 | Joshka (ratatui) | Includes scrollview, maintained by core team |
| arboard | 3.5.0 | 1Password | Best clipboard crate, Wayland support critical |
| syntect | 5.3.0 | trishume | Mature, pure-Rust with `default-fancy` |

**Why NOT:**
- `better-panic` → ratatui 0.30 has built-in panic hooks via `ratatui::init()`
- `tree-sitter-highlight` → grammar version compatibility issues
- `termion` → Unix-only, no Windows support

### 1.3 Core Data Structures

```rust
// tree.rs
pub struct TreeNode {
    pub change_id: String,
    pub commit_id: CommitId,
    pub unique_prefix_len: usize,
    pub description: String,
    pub bookmarks: Vec<String>,
    pub is_working_copy: bool,
    pub is_selected: bool,      // For multi-select operations
    pub parent_ids: Vec<String>,
    pub children: Vec<usize>,   // Indices into flat vec
    pub depth: usize,           // Indentation level
    pub is_trunk: bool,         // Is this master/main/trunk?

    // Status indicators
    pub has_conflicts: bool,    // ⚠ symbol - commit has unresolved conflicts
    pub is_dirty: bool,         // ● symbol - working copy has uncommitted changes (only for @)
    pub files_changed: Option<u32>,  // +N files changed count

    // Remote tracking (for bookmarked commits)
    pub ahead_of_remote: Option<u32>,   // ↑N commits ahead
    pub behind_remote: Option<u32>,     // ↓N commits behind
}

pub struct TreeState {
    pub nodes: Vec<TreeNode>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub trunk_index: Option<usize>,  // Show trunk as base
    pub selected_for_action: Vec<usize>,  // Multi-select for abandon
}

// app.rs
pub enum Mode {
    Normal,
    ActionMenu(ActionMenuState),  // Enter key - popup with all actions
    ViewingDiff(DiffState),       // D key - syntax highlighted diff
    Preview(PreviewState),
    Editing(EditingState),
    Confirming(ConfirmState),
    Resolving(ConflictState),
    Rebasing(RebaseModeState),
    MovingBookmark(MovingBookmarkState),
    OperationLog(OperationLogState),
    VerifyMismatch(VerifyMismatchState),  // Result != preview, offer undo
    Help,
}

pub struct VerifyMismatchState {
    pub op_before: OperationId,  // Snapshot to undo to
    pub message: String,
}

pub struct App {
    pub tree: TreeState,
    pub mode: Mode,
    pub jj_repo: JjRepo,
    pub shell: Shell,
    pub message: Option<(String, MessageType)>,
    pub undo_stack: Vec<OperationId>,  // For jj undo
}
```

---

## Phase 2: Tree Display Enhancements

### 2.1 Enhanced Tree Query

Modify the revset to include:
- **Trunk as base**: Show `trunk()` as the root of the tree
- **Siblings**: Show `children(trunk()..@-)` - siblings not in main stack
- **Descendants of @**: Show `descendants(@)` - commits after working copy

```rust
// New revset for comprehensive tree view
fn comprehensive_tree_revset() -> &'static str {
    // trunk + all descendants of trunk that are ancestors of @ or siblings
    "trunk() | descendants(trunk()) & (::@ | @:: | siblings(::@))"
}
```

### 2.2 Tree Rendering Layout

```
┌─ JJ Tree ──────────────────────────────────────────────┐
│ ○ master  origin sync point              ↑2↓1 kp3x    │  <- remote tracking
│ ├── feature-a  Add user auth        +2        mn7y    │
│ │   └── @● (working copy)                     qr9z    │  <- ● = has changes
│ │       └── ⚠ (conflict!)           +1        st4w    │  <- ⚠ = conflicts
│ ├── feature-b  Fix login bug                  uv6x    │  <- sibling
│ └── experiment  Try new API                   ab2c    │  <- sibling
├────────────────────────────────────────────────────────┤
│ [j/k] Nav  [Enter] Actions  [D] Diff  [r] Rebase  [?] Help│
└────────────────────────────────────────────────────────┘
```

**Visual Indicators:**

| Symbol | Meaning | Color |
|--------|---------|-------|
| `@` | Working copy | Cyan |
| `●` | Has uncommitted changes (dirty) | Yellow |
| `⚠` | Has conflicts | Red |
| `↑N` | Commits ahead of remote | Green |
| `↓N` | Commits behind remote | Red |
| `+N` | Files changed | Dim |

---

## Phase 3: Key Bindings & Actions

### Mode Overview

The TUI has several modes, each with different key behaviors:

| Mode | Enter With | Exit With | Purpose |
|------|------------|-----------|---------|
| **Normal** | (default) | - | Navigate tree, trigger actions |
| **Action Menu** | `Enter` | `Esc` or select action | Popup with all available actions for revision |
| **Rebase** | `r` or `s` | `Esc` or `Enter` | Move revisions anywhere in tree |
| **Bookmark Move** | `m` | `Esc` or `Enter` | Move bookmark to different revision |
| **Editing Desc** | `d` | `Esc` or `Enter` | Edit commit message |
| **Diff View** | `D` | `Esc` | View diff (scrollable, syntax highlighted) |
| **Operation Log** | `O` | `Esc` | Browse/restore previous states |
| **Preview** | (after actions) | `Esc` or `Enter` | Confirm/cancel pending operation |
| **Help Menu** | `?` | `?` or `Esc` | View keybindings pane |

**Quit:** Press `Ctrl+c` twice to exit the TUI.

### 3.1 Navigation & View (Normal Mode)

| Key | Action | Description |
|-----|--------|-------------|
| `j` / `↓` | Move down | Navigate to next commit in current stack |
| `k` / `↑` | Move up | Navigate to previous commit in current stack |
| `h` | Move left | Jump to sibling stack (left) |
| `l` | Move right | Jump to sibling stack (right) |
| `g` | Go to top | Jump to trunk |
| `G` | Go to bottom | Jump to last commit |
| `@` | Go to working copy | Jump to @ |
| `Space` | **Show details** | Expand revision info (full SHA, author, date, files changed) |
| `Enter` | **Action menu** | Open popup with all available actions for this revision |
| `D` | **View diff** | Show diff for current revision (scrollable, syntax highlighted) |
| `f` | Toggle full mode | Show/hide commits without bookmarks |
| `e` | **Edit working copy** | Make this revision the working copy (`jj edit`) |
| `y` | **Yank git SHA** | Copy full git commit SHA to clipboard |
| `Y` | **Yank change id** | Copy jj change id (rev) to clipboard |

### 3.1.1 New Commit Operations

| Key | Action | Description |
|-----|--------|-------------|
| `n` | **New commit** | Create new commit on top of current (`jj new`) |
| `c` | **Commit** | Commit working copy changes (`jj commit`) - only if @ has changes |
| `F` | **Fetch** | Fetch from remote (`jj git fetch`) |

### 3.2 Action Menu Popup (`Enter`)

When pressing `Enter` on a revision, show a popup menu with all available actions:

```
┌─ Actions for "fix login" (mn7y) ──────────────────────┐
│                                                        │
│   D  View diff                                         │
│   d  Edit description                                  │
│   e  Edit working copy (jj edit)                       │
│   ─────────────────────────────────────                │
│   r  Rebase (single revision)                          │
│   s  Rebase (with descendants)                         │
│   q  Squash into parent                                │
│   ─────────────────────────────────────                │
│   n  New commit on top                                 │
│   a  Abandon this revision                             │
│   ─────────────────────────────────────                │
│   m  Move bookmark (if present)                        │
│   b  Create bookmark here                              │
│   ─────────────────────────────────────                │
│   y  Copy git SHA                                      │
│   Y  Copy change id                                    │
│                                                        │
│   [Esc] Close                                          │
└────────────────────────────────────────────────────────┘
```

**Implementation:**
```rust
pub struct ActionMenuState {
    pub target_rev: String,
    pub selected_index: usize,
    pub actions: Vec<ActionItem>,
}

pub struct ActionItem {
    pub key: char,
    pub label: String,
    pub enabled: bool,  // Gray out unavailable actions
}

fn build_action_menu(app: &App, rev: &str) -> Vec<ActionItem> {
    let has_bookmark = has_bookmark_at(rev);
    let is_working_copy = is_working_copy(rev);
    let has_changes = has_uncommitted_changes();

    vec![
        ActionItem { key: 'D', label: "View diff".into(), enabled: true },
        ActionItem { key: 'd', label: "Edit description".into(), enabled: true },
        ActionItem { key: 'e', label: "Edit working copy".into(), enabled: !is_working_copy },
        // ... etc
        ActionItem { key: 'm', label: "Move bookmark".into(), enabled: has_bookmark },
    ]
}
```

### 3.3 View Details & Diff

| Key | Action |
|-----|--------|
| `Space` | Toggle detail panel (SHA, author, date, files changed) |
| `D` | View full diff (scrollable, syntax highlighted, `j/k` to scroll, `Esc` to close) |

**Detail Panel (`Space`):**
```
┌─ JJ Tree ──────────────────────────────────────────────┐
│ ○ master  origin sync point                  kp3x     │
│ ├── feature-a  Add user auth                 mn7y     │
│ │   ┌──────────────────────────────────────────────┐  │
│ │   │ Commit: mn7y8z9a0b1c2d3e4f5g6h7i8j9k0l1m2n  │  │
│ │   │ Author: alice@example.com                    │  │
│ │   │ Date:   2024-01-15 14:30:00                  │  │
│ │   │ Files:  +3 -1 (src/auth.rs, src/lib.rs)      │  │
│ │   └──────────────────────────────────────────────┘  │
│ │   └── @ (working copy)                     qr9z     │
└────────────────────────────────────────────────────────┘
```

**Diff View with Syntax Highlighting (`D`):**
```rust
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;

fn view_diff(app: &mut App, rev: &str) -> Result<()> {
    let diff = cmd!(app.shell, "jj diff -r {rev}").read()?;
    let highlighted = highlight_diff(&diff)?;
    app.mode = Mode::ViewingDiff(DiffState {
        content: highlighted,
        scroll: 0,
        syntax_set: SyntaxSet::load_defaults_newlines(),
        theme: ThemeSet::load_defaults().themes["base16-ocean.dark"].clone(),
    });
}

fn highlight_diff(diff: &str) -> Vec<HighlightedLine> {
    // Parse diff, detect file types from headers
    // Apply syntax highlighting per-file section
    // Color + lines green, - lines red
}
```

### 3.3 Edit Description / Working Copy (Action 2)

| Key | Action |
|-----|--------|
| `d` | Edit commit description (inline) |
| `D` | Edit description in $EDITOR |
| `e` | Edit working copy - make this revision @ (`jj edit`) |

**Edit Working Copy (`e`):**
```rust
fn edit_working_copy(app: &mut App, rev: &str) -> Result<()> {
    cmd!(app.shell, "jj edit {rev}").run()?;
    refresh_tree(app)?;
    app.message = Some((format!("Now editing {}", rev), MessageType::Success));
}
```

**Implementation:**
```rust
fn edit_description(app: &mut App, rev: &str) -> Result<()> {
    // Inline editing with tui-textarea
    let current = get_description(rev)?;
    app.mode = Mode::Editing(EditingState {
        textarea: TextArea::new(current.lines().collect()),
        target_rev: rev.to_string(),
    });
}

fn apply_description(app: &mut App, new_desc: &str) -> Result<()> {
    cmd!(app.shell, "jj desc -r {rev} -m {new_desc}").run()?;
}
```

### 3.4 Rebase Mode (Action 3)

Enter a modal interface to move revisions anywhere in the tree.

**Normal Mode → Rebase Mode:**

| Key | Action |
|-----|--------|
| `r` | Enter Rebase Mode - move **single revision** (`-r`) |
| `s` | Enter Rebase Mode - move **revision + descendants** (`-s`) |

**Inside Rebase Mode:**

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate to destination (any revision in tree) |
| `h` / `l` | Move between stacks |
| `Enter` | Confirm - insert after destination (inline by default) |
| `b` | **Toggle branch/offshoot mode** (default: OFF) |
| `Esc` | Cancel and return to Normal Mode |

Position is **implied** by where you navigate - no need to specify "after" or "before" explicitly.

**Branch Mode (toggle with `b`):**

| Mode | Behavior | jj command |
|------|----------|------------|
| **OFF** (default) | Clean inline insertion | `-A dest -B next` (both flags) |
| **ON** | May create branches | `-A dest` alone |

```
Branch OFF (default):           Branch ON:
Moving X to after B:            Moving X to after B:

A → B → X → C → D               A → B → C → D
    (X inserted inline)              └── X  (offshoot!)

Command: jj rebase -r X         Command: jj rebase -r X
         -A B -B C                       -A B
```

**Why `-A` AND `-B` together by default?**
Using both flags guarantees inline insertion:
- `-A B` = X comes after B
- `-B C` = X comes before C (B's child)

This prevents accidental branches when B has multiple children.

**Single Revision (`-r`) vs Source + Descendants (`-s`):**

```
Stack before:          -r (single)           -s (with descendants)
○ A                    ○ A                   ○ A
├── B ← move this      ├── C                 ├── C
│   └── C              ├── B (moved alone)   │   └── D
│       └── D          │   └── D (orphaned)  └── B (moved with C, D)
└── E                  └── E                     └── E
```

- **`r` → `-r` (single)**: Only moves the selected commit; its children get reparented to its parent
- **`s` → `-s` (source)**: Moves the commit AND all its descendants together

**Rebase Mode UI:**
```
┌─ REBASE MODE ─────────────────────────────────────────┐
│ Source: "fix login" (abc123)  [mode: -r single]       │
│ Branch mode: OFF (inline insertion)                   │
│                                                        │
│ ○ master                                              │
│ ├── feature-auth                                      │
│ │   └── [SOURCE] fix login  ← moving this            │
│ ├──►feature-api             ← destination (cursor)   │
│ │   └── add endpoints       ← will stay as child     │
│ └── feature-ui                                        │
│                                                        │
│ Command: jj rebase -r abc123 -A feature-api -B add-endpoints│
│                                                        │
│ [Enter] Confirm   [b] Toggle branch   [Esc] Cancel    │
└────────────────────────────────────────────────────────┘
```

**Implementation:**
```rust
pub enum RebaseMode {
    SingleRevision,        // -r: just this commit
    SourceWithDescendants, // -s: this commit + all descendants
}

pub struct RebaseModeState {
    pub source_rev: String,
    pub mode: RebaseMode,
    pub destination_cursor: usize,
    pub allow_branches: bool,  // default: false (clean inline insertion)
    pub op_before: OperationId, // Snapshot for undo if result doesn't match preview
}

fn enter_rebase_mode(app: &mut App, mode: RebaseMode) -> Result<()> {
    // IMPORTANT: Capture operation ID BEFORE starting rebase
    // This allows undo back to exactly this point if result != preview
    let op_before = get_current_operation_id(&app.shell)?;

    let source = app.current_rev().clone();
    app.mode = Mode::Rebasing(RebaseModeState {
        source_rev: source,
        mode,
        destination_cursor: app.tree.cursor,
        allow_branches: false,  // default: clean stacks
        op_before,
    });
    Ok(())
}

fn confirm_rebase(app: &mut App, state: &RebaseModeState) -> Result<()> {
    let sh = &app.shell;
    let mode_flag = match state.mode {
        RebaseMode::SingleRevision => "-r",
        RebaseMode::SourceWithDescendants => "-s",
    };

    let dest = get_rev_at_cursor(state.destination_cursor);

    if state.allow_branches {
        // Simple: just use -A, may create branch if dest has multiple children
        cmd!(sh, "jj rebase {mode_flag} {source_rev} -A {dest}").run()?;
    } else {
        // Clean inline: use both -A and -B to ensure no branches
        if let Some(next) = get_first_child_of(dest)? {
            // Insert between dest and its child
            cmd!(sh, "jj rebase {mode_flag} {source_rev} -A {dest} -B {next}").run()?;
        } else {
            // dest has no children, just -A is fine
            cmd!(sh, "jj rebase {mode_flag} {source_rev} -A {dest}").run()?;
        }
    }

    // VERIFY: Compare actual result to preview
    // If mismatch, offer immediate undo
    let actual_tree = refresh_and_get_tree(app)?;
    if !matches_preview(&actual_tree, &state.expected_preview) {
        app.mode = Mode::VerifyMismatch(VerifyMismatchState {
            op_before: state.op_before.clone(),
            message: "Result doesn't match preview. Undo?".into(),
        });
    } else {
        app.mode = Mode::Normal;
        app.message = Some(("Rebase completed".into(), MessageType::Success));
    }
    Ok(())
}

/// Check if actual tree topology matches what we previewed
fn matches_preview(actual: &TreeState, expected: &TreeState) -> bool {
    // Compare parent-child relationships for affected nodes
    // Ignore metadata like timestamps, only check structure
    actual.nodes.iter().zip(expected.nodes.iter()).all(|(a, e)| {
        a.change_id == e.change_id && a.parent_ids == e.parent_ids
    })
}
```

**Verify Mismatch Dialog:**
```
┌─ Preview Mismatch ─────────────────────────────────────┐
│                                                        │
│ ⚠ The result doesn't match the preview!               │
│                                                        │
│ This can happen when:                                  │
│   • Another commit had the same parent                 │
│   • A conflict occurred                                │
│   • The commit was already in the target location      │
│                                                        │
│ [u] Undo to before the move                            │
│ [k] Keep the result anyway                             │
│                                                        │
└────────────────────────────────────────────────────────┘
```

```rust
fn handle_verify_mismatch(app: &mut App, state: &VerifyMismatchState, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('u') => {
            // Undo to the snapshot taken before the move started
            cmd!(app.shell, "jj op restore {}", state.op_before).run()?;
            refresh_tree(app)?;
            app.message = Some(("Undone to before the move".into(), MessageType::Info));
            app.mode = Mode::Normal;
        }
        KeyCode::Char('k') | KeyCode::Esc => {
            // Keep the result
            app.message = Some(("Kept the result".into(), MessageType::Info));
            app.mode = Mode::Normal;
        }
        _ => {}
    }
    Ok(())
}
```

**Visual Feedback in Rebase Mode:**
- Source revision: highlighted in **yellow** with `[SOURCE]` marker
- Cursor/destination: highlighted with `►` marker
- Invalid destinations (ancestors when using -s): **dimmed/grayed**
- Status bar shows current command that will be executed

### 3.4.1 Handling `--skip-emptied`

The `--skip-emptied` flag abandons commits that **become empty** as a result of the rebase
(e.g., their changes already exist in the destination). It does NOT abandon commits that
were already empty before the rebase.

**When to use:**

| Operation | `--skip-emptied`? | Reasoning |
|-----------|-------------------|-----------|
| Move up/down in stack | **Ask/Toggle** | Reordering usually doesn't create empty commits, but user should decide |
| Rebase onto trunk (`R` to trunk) | **Yes (default)** | Merged changes on trunk → commits become empty → abandon them |

**Implementation - Toggle in Preview:**
```rust
pub struct PreviewState {
    pub action: PreviewAction,
    pub skip_emptied: bool,  // Toggle with 's' key in preview mode
    // ... other fields
}
```

**Preview UI with Toggle:**
```
┌─ Rebase Preview ──────────────────────────────────────┐
│ Moving "fix login" down in stack                      │
│                                                        │
│ Command: jj rebase -r abc -B def                      │
│                                                        │
│ [x] Skip commits that become empty (--skip-emptied)   │
│                                                        │
│ [Enter] Confirm   [s] Toggle skip   [Esc] Cancel      │
└────────────────────────────────────────────────────────┘
```

**Smart Detection (future enhancement):**
```rust
fn detect_would_become_empty(rev: &str, destination: &str) -> Result<bool> {
    // Compare the tree of rev with the tree at destination
    // If rev's changes already exist at destination, it would become empty
    // Show warning in preview if true
}
```

### 3.5 Multi-Select for Abandon (Action 4)

| Key | Action |
|-----|--------|
| `x` | Toggle selection on current commit (mark for action) |
| `v` | Enter visual/multi-select mode (select range with `j/k`) |
| `a` | Abandon selected (or current if none selected) |
| `Esc` | Clear selection |

Note: `x` marks commits like "X marks the spot" - quick toggle for individual commits.

**Implementation:**
```rust
fn abandon_selected(app: &mut App) -> Result<()> {
    let targets: Vec<_> = app.tree.selected_for_action
        .iter()
        .map(|i| &app.tree.nodes[*i].change_id)
        .collect();

    if targets.is_empty() {
        targets.push(&app.tree.nodes[app.tree.cursor].change_id);
    }

    show_preview(app, PreviewAction::Abandon { revs: targets });
}
```

### 3.6 Squash (Action 5)

| Key | Action |
|-----|--------|
| `q` | Squash into parent |
| `Q` | Squash into... (select target via navigation) |

Note: `s` is reserved for entering Rebase Mode with `-s` (source + descendants).

**Implementation:**
```rust
fn squash_into_parent(app: &mut App) -> Result<()> {
    let source = app.current_rev();
    show_preview(app, PreviewAction::Squash { source, into: "parent" });
}

fn squash_into_target(app: &mut App, source: &str, target: &str) -> Result<()> {
    show_preview(app, PreviewAction::Squash { source, into: target });
}
```

### 3.7 Quick Rebase Shortcuts

For common patterns without entering full Rebase Mode:

| Key | Action |
|-----|--------|
| `t` | Rebase current onto trunk (`jj rebase -r @ -o trunk()`) |
| `T` | Rebase current + descendants onto trunk (`jj rebase -s @ -o trunk()`) |

---

## Phase 4: Preview System

### 4.1 Preview State

Before executing any operation, show a preview of how the tree would change:

```rust
pub enum PreviewAction {
    RebaseInsertAfter { rev: String, target: String },
    RebaseInsertBefore { rev: String, target: String },
    Abandon { revs: Vec<String> },
    Squash { source: String, into: String },
}

pub struct PreviewState {
    pub action: PreviewAction,
    pub current_tree: TreeState,
    pub preview_tree: TreeState,  // Simulated result
    pub command: String,           // jj command that will be run
}
```

### 4.2 Preview UI Layout

```
┌─ Current ─────────────────┬─ After ───────────────────┐
│ ○ master                  │ ○ master                  │
│ ├── feature-a             │ ├── feature-b  ← moved up │
│ │   └── @ feature-b       │ ├── feature-a             │
│ └── feature-c             │ │   └── @                 │
│                           │ └── feature-c             │
├───────────────────────────┴───────────────────────────┤
│ Command: jj rebase -r qr9z -A kp3x                    │
│                                                        │
│ [Enter] Confirm   [Esc] Cancel                        │
└────────────────────────────────────────────────────────┘
```

### 4.3 Preview Simulation

```rust
fn simulate_preview(action: &PreviewAction, current: &TreeState) -> TreeState {
    // Clone and modify tree structure to show expected result
    // This is a local simulation, not an actual jj operation
    match action {
        PreviewAction::RebaseInsertAfter { rev, target } => {
            // Simulate moving rev to be a child of target
            simulate_rebase_after(current, rev, target)
        }
        // ... other actions
    }
}
```

---

## Phase 5: Conflict Resolution Flow

### 5.1 Detect Conflicts After Operation

```rust
fn execute_with_conflict_check(app: &mut App, action: &PreviewAction) -> Result<()> {
    // 1. Record current operation ID for potential undo
    let op_before = get_current_operation_id()?;

    // 2. Execute the action
    execute_action(action)?;

    // 3. Check for conflicts
    if has_conflicts(&app.shell)? {
        app.mode = Mode::Resolving(ConflictState {
            op_before,
            conflicted_files: get_conflicted_files()?,
        });
    } else {
        // 4. Refresh tree and verify it matches preview
        refresh_tree(app)?;
        verify_matches_preview(app)?;
    }
}
```

### 5.2 Automatic Conflict Resolution Flow

The flow for `jj new` → `jj resolve` → `jj squash`:

```rust
pub struct ConflictState {
    pub op_before: OperationId,
    pub conflicted_files: Vec<PathBuf>,
    pub step: ConflictStep,
}

pub enum ConflictStep {
    Showing,           // Show conflict info
    CreatedNew,        // After jj new
    WaitingResolve,    // User resolving in editor
    ReadyToSquash,     // After resolution
}

fn handle_conflict(app: &mut App) -> Result<()> {
    match app.conflict_state.step {
        ConflictStep::Showing => {
            // Show: "Conflicts detected. [n] Create new commit to resolve, [u] Undo"
        }
        ConflictStep::CreatedNew => {
            // Run: jj new
            cmd!(app.shell, "jj new").run()?;
            app.conflict_state.step = ConflictStep::WaitingResolve;
            // Show: "Resolve conflicts, then press [r] or run jj resolve"
        }
        ConflictStep::WaitingResolve => {
            // Option to run jj resolve for each file
            cmd!(app.shell, "jj resolve").run()?;
            app.conflict_state.step = ConflictStep::ReadyToSquash;
        }
        ConflictStep::ReadyToSquash => {
            // Run: jj squash
            cmd!(app.shell, "jj squash").run()?;
            // Exit conflict mode, refresh tree
            app.mode = Mode::Normal;
            refresh_tree(app)?;
        }
    }
}
```

### 5.3 Undo Capability

```rust
fn undo_to_before(app: &mut App) -> Result<()> {
    if let Some(op_id) = app.undo_stack.pop() {
        cmd!(app.shell, "jj op restore {op_id}").run()?;
        refresh_tree(app)?;
        app.message = Some(("Operation undone".into(), MessageType::Info));
    }
}
```

---

## Phase 6: Additional Features

### 6.1 Help Menu Pane (`?` key)

A dedicated pane showing all keybindings, organized by category.

```
┌─ Tree ─────────────────────┬─ Help ──────────────────────────┐
│ ○ master          ↑2↓1     │ NAVIGATION                      │
│ ├── feature-auth           │   j/k     Up/Down in stack      │
│ │   └──►fix login          │   h/l     Left/Right stacks     │
│ │       └── @● wip         │   g/G     Top/Bottom            │
│ │           └── ⚠ conflict │   @       Go to working copy    │
│ └── feature-api            │   Space   Show commit details   │
│     └── add endpoints      │   Enter   Action menu           │
│                            │   D       View diff             │
│                            │   f       Toggle full mode      │
│                            │                                 │
│                            │ CREATE / EDIT                   │
│                            │   n       New commit (jj new)   │
│                            │   c       Commit (jj commit)    │
│                            │   e       Edit working copy (@) │
│                            │   d       Edit description      │
│                            │   y       Yank git SHA          │
│                            │   Y       Yank change id        │
│                            │                                 │
│                            │ REBASE MODE (r/s to enter)      │
│                            │   r       Rebase single (-r)    │
│                            │   s       Rebase + desc (-s)    │
│                            │   t/T     Rebase onto trunk     │
│                            │   (in mode: b = branch toggle)  │
│                            │                                 │
│                            │ ACTIONS                         │
│                            │   q       Squash into parent    │
│                            │   x       Mark for action       │
│                            │   a       Abandon marked        │
│                            │   u       Undo last operation   │
│                            │   O       Operation log         │
│                            │                                 │
│                            │ REMOTE                          │
│                            │   F       Fetch (jj git fetch)  │
│                            │   p       Push bookmark         │
│                            │   P       Push all bookmarks    │
│                            │                                 │
│                            │ BOOKMARKS                       │
│                            │   m       Move bookmark         │
│                            │   b       New bookmark          │
│                            │   B       Delete bookmark       │
│                            │                                 │
│                            │ LAYOUT                          │
│                            │   \       Toggle multi-pane     │
│                            │   Tab     Switch pane focus     │
│                            │   Ctrl+c  Quit (press twice)    │
├────────────────────────────┴─────────────────────────────────┤
│ [?] Close help                                               │
└──────────────────────────────────────────────────────────────┘
```

The help menu opens as a **side pane** (not overlay), so you can still see the tree while reading keybindings.

```rust
fn toggle_help(app: &mut App) {
    match app.mode {
        Mode::Help => app.mode = Mode::Normal,
        _ => app.mode = Mode::Help,
    }
}
```

### 6.2 Operation Log View (`O` key)

jj's unique feature - browse and restore previous repository states.

| Key | Action |
|-----|--------|
| `O` | Open Operation Log panel |
| `u` | Undo to selected operation |
| `Esc` | Close panel |

**Operation Log Panel:**
```
┌─ Operation Log ───────────────────────────────────────┐
│ Operations (newest first):                            │
│                                                        │
│ > 3m ago   rebase -r abc -A def                       │
│   15m ago  commit -m "fix login"                      │
│   1h ago   squash                                     │
│   2h ago   new                                        │
│   3h ago   fetch origin                               │
│                                                        │
│ [Enter] View details   [u] Undo to here   [Esc] Close │
└────────────────────────────────────────────────────────┘
```

```rust
fn show_operation_log(app: &mut App) -> Result<()> {
    let ops = cmd!(app.shell, "jj op log --limit 20").read()?;
    app.mode = Mode::OperationLog(OperationLogState {
        operations: parse_op_log(&ops),
        selected: 0,
    });
}

fn undo_to_operation(app: &mut App, op_id: &str) -> Result<()> {
    cmd!(app.shell, "jj op restore {op_id}").run()?;
    refresh_tree(app)?;
    app.message = Some(("Restored to previous state".into(), MessageType::Success));
}
```

### 6.3 Multi-Pane Layout

Optional side-by-side layout showing tree and diff simultaneously.

| Key | Action |
|-----|--------|
| `\` | Toggle multi-pane layout |
| `Tab` | Switch focus between panes |

**Multi-Pane Layout:**
```
┌─ Tree ─────────────────────┬─ Diff ──────────────────────────┐
│ ○ master                   │ src/auth.rs                     │
│ ├── feature-auth           │ @@ -10,6 +10,8 @@               │
│ │   └──►fix login          │  fn authenticate() {            │
│ │       └── @ wip          │ +    let token = get_token();   │
│ └── feature-api            │ +    validate(token)?;          │
│     └── add endpoints      │      Ok(())                     │
│                            │  }                              │
├────────────────────────────┴─────────────────────────────────┤
│ [j/k] Nav  [Tab] Switch pane  [\] Toggle layout  [?] Help    │
└──────────────────────────────────────────────────────────────┘
```

When in multi-pane mode:
- Left pane: tree navigation (j/k/h/l)
- Right pane: auto-updates to show diff of selected revision
- `Tab` switches keyboard focus for scrolling the diff

### 6.4 Contextual Help Bar

**Always visible** at the bottom, showing relevant keys for current mode.

```rust
fn render_contextual_help(mode: &Mode) -> String {
    match mode {
        Mode::Normal => "[j/k] Nav  [Enter] Actions  [D] Diff  [n] New  [r/s] Rebase  [?] Help",
        Mode::ActionMenu(_) => "[j/k] Select  [Enter] Execute  [Esc] Close",
        Mode::Rebasing(s) => {
            let branch = if s.allow_branches { "ON" } else { "OFF" };
            format!("[j/k] Dest  [b] Branch:{}  [Enter] Confirm  [Esc] Cancel", branch)
        },
        Mode::MovingBookmark(_) => "[j/k] Select dest  [Enter] Drop here  [Esc] Cancel",
        Mode::EditingDesc(_) => "[Enter] Save  [Esc] Cancel",
        Mode::ViewingDiff(_) => "[j/k] Scroll  [Esc] Close",
        Mode::OperationLog(_) => "[j/k] Select  [u] Undo to here  [Esc] Close",
        Mode::VerifyMismatch(_) => "[u] Undo to before move  [k] Keep result",
        Mode::Help => "[?] or [Esc] Close",
        _ => "",
    }
}
```

This ensures users **always know what keys are available** without needing to open help.

### 6.5 Status Bar Messages

```rust
pub enum MessageType {
    Info,
    Success,
    Warning,
    Error,
}

fn show_message(app: &mut App, msg: &str, msg_type: MessageType) {
    app.message = Some((msg.to_string(), msg_type));
    // Auto-clear after 3 seconds
}
```

### 6.6 Bookmarks Quick Actions

| Key | Action |
|-----|--------|
| `m` | **Move bookmark from current revision** (if one exists) - enter Bookmark Move Mode |
| `b` | Create new bookmark on current revision |
| `B` | Delete bookmark (select from list if multiple) |
| `p` | Push bookmark at current revision to remote |
| `P` | Push all bookmarks |

**Move Bookmark Flow (`m` key):**

If the current revision has a bookmark, pressing `m` enters Bookmark Move Mode:
1. The bookmark is "picked up" from the current revision
2. Navigate with `j/k/h/l` to the destination revision
3. Press `Enter` to drop the bookmark at the new location
4. Press `Esc` to cancel

```rust
fn move_bookmark(app: &mut App) -> Result<()> {
    let current = app.current_rev();
    let bookmarks_here = get_bookmarks_at(current)?;

    if bookmarks_here.is_empty() {
        app.message = Some(("No bookmark at current revision".into(), MessageType::Warning));
        return Ok(());
    }

    // If multiple bookmarks, let user select which one
    let bookmark = if bookmarks_here.len() == 1 {
        bookmarks_here[0].clone()
    } else {
        // Show selection dialog
        select_bookmark(&bookmarks_here)?
    };

    app.mode = Mode::MovingBookmark(MovingBookmarkState {
        bookmark,
        source_rev: current.clone(),
        destination_cursor: app.tree.cursor,
    });
}

// After navigation and Enter:
fn apply_bookmark_move(bookmark: &str, target_rev: &str) -> Result<()> {
    cmd!(sh, "jj bookmark set {bookmark} -r {target_rev}").run()?;
}
```

**Bookmark Move Mode UI:**
```
┌─ BOOKMARK MOVE MODE ──────────────────────────────────┐
│ Moving bookmark: "feature-auth"                       │
│                                                        │
│ ○ master                                              │
│ ├── [feature-auth] fix login  ← moving FROM here     │
│ ├──►feature-api               ← destination (cursor) │
│ │   └── add endpoints                                 │
│ └── feature-ui                                        │
│                                                        │
│ [Enter] Drop bookmark here   [Esc] Cancel             │
└────────────────────────────────────────────────────────┘
```

**Create New Bookmark (`b` key):**
```rust
fn create_bookmark(app: &mut App) -> Result<()> {
    // Prompt for bookmark name (inline text input)
    app.mode = Mode::CreatingBookmark(CreatingBookmarkState {
        name_input: String::new(),
        target_rev: app.current_rev().clone(),
    });
}
```

---

## Phase 7: Implementation Order

### Sprint 1: Skeleton ✅ COMPLETE
- [x] Add ratatui/crossterm dependencies
- [x] Create basic TUI app structure with event loop
- [x] Port tree data loading from existing `tree()` function
- [x] Render static tree with navigation (j/k)
- [x] Add trunk as base, handle full tree query
- [x] Visual depth computation for non-full mode
- [x] Help dialog (? key)
- [x] Status bar with keybinding hints

**Note:** Keybindings are hardcoded in `app.rs::handle_normal_key()`. The `keybindings.rs` file is a stub for future configurable keymaps.

### Sprint 2: View Operations 🔲 IN PROGRESS

#### Diff Viewing (D key)
- [ ] Add syntect dependency to Cargo.toml
  ```toml
  syntect = { version = "5.3", default-features = false, features = ["default-fancy"] }
  ```
- [ ] Define DiffState struct and DiffLine types
- [ ] Add `Mode::ViewingDiff(DiffState)` to Mode enum
- [ ] Implement `enter_diff_view()` - fetch via `jj diff -r {rev}`
- [ ] Implement `handle_diff_key()` for j/k scroll, Esc close
- [ ] Add `render_diff()` in ui.rs with syntax highlighting

#### Commit Details (Space key)
- [ ] Add `expanded: bool` field to track expanded state
- [ ] Toggle expansion on Space press
- [ ] Render expanded details below selected commit (no overlay)
- [ ] Show: full SHA, author, date, files changed

#### Scroll Support (Ctrl+u/d)
- [ ] Track KeyModifiers in key handling
- [ ] Add Ctrl+u handler for half-page up
- [ ] Add Ctrl+d handler for half-page down
- [ ] Implement `page_up(amount)` and `page_down(amount)` on TreeState

#### Multi-pane Layout (\\ key)
- [ ] Add `split_view: bool` to App struct
- [ ] Toggle with backslash key
- [ ] Split layout horizontally when enabled
- [ ] Auto-update right pane diff when cursor moves
- [ ] Tab to switch keyboard focus between panes

### Sprint 3: Edit Operations 🔲 NOT STARTED
- [ ] Implement description editing (`d` key) with tui-textarea
- [ ] Add multi-select mode (`x` to mark, `v` for visual)
- [ ] Implement abandon with preview (`a` key)
- [ ] Edit working copy (`e` key) - `jj edit`
- [ ] New commit (`n` key) - `jj new`
- [ ] Commit changes (`c` key) - `jj commit`

### Sprint 4: Rebase Operations 🔲 NOT STARTED
- [ ] Rebase mode entry (`r` for single, `s` for source+descendants)
- [ ] Branch mode toggle (`b` in rebase mode)
- [ ] Quick rebase onto trunk (`t`/`T` keys)
- [ ] Preview system (side-by-side before/after)
- [ ] Verify result matches preview, offer undo if not

### Sprint 5: Squash & Conflicts 🔲 NOT STARTED
- [ ] Squash into parent (`q` key)
- [ ] Squash into target (`Q` key)
- [ ] Conflict detection after operations
- [ ] Automatic new → resolve → squash flow
- [ ] Operation-based undo (`u` key)
- [ ] Operation log view (`O` key)

### Sprint 6: Polish 🔲 NOT STARTED
- [ ] Bookmark operations (`m` move, `b` create, `B` delete)
- [ ] Remote operations (`F` fetch, `p` push, `P` push all)
- [ ] Clipboard (`y` yank SHA, `Y` yank change id)
- [ ] Action menu popup (Enter key)
- [ ] Status indicators (conflicts ⚠, dirty ●, ahead/behind ↑↓)
- [ ] Configurable keybindings via keymap file (wire up keybindings.rs)

---

## Sprint 2 Implementation Guide

### Adding ViewingDiff Mode - Step by Step

**1. Add syntect to Cargo.toml:**
```toml
syntect = { version = "5.3", default-features = false, features = ["default-fancy"] }
```

**2. Define types in app.rs:**
```rust
pub struct DiffState {
    pub lines: Vec<DiffLine>,
    pub scroll_offset: usize,
    pub rev: String,
}

pub struct DiffLine {
    pub content: String,
    pub style: DiffLineStyle,
}

#[derive(Clone, Copy)]
pub enum DiffLineStyle {
    Header,      // @@ ... @@
    Added,       // + lines (green)
    Removed,     // - lines (red)
    Context,     // unchanged lines
    FileHeader,  // diff --git a/... b/...
}

pub enum Mode {
    Normal,
    Help,
    ViewingDiff(DiffState),  // ADD THIS
}
```

**3. Update handle_key in app.rs:**
```rust
fn handle_key(&mut self, code: KeyCode) {
    match self.mode {
        Mode::Normal => self.handle_normal_key(code),
        Mode::Help => self.handle_help_key(code),
        Mode::ViewingDiff(_) => self.handle_diff_key(code),  // ADD THIS
    }
}

fn handle_normal_key(&mut self, code: KeyCode) {
    match code {
        // ... existing handlers ...
        KeyCode::Char('D') => {
            if let Err(e) = self.enter_diff_view() {
                // handle error - maybe set a message
            }
        }
        _ => {}
    }
}

fn handle_diff_key(&mut self, code: KeyCode) {
    match code {
        KeyCode::Char('j') | KeyCode::Down => {
            if let Mode::ViewingDiff(ref mut state) = self.mode {
                state.scroll_offset = state.scroll_offset.saturating_add(1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Mode::ViewingDiff(ref mut state) = self.mode {
                state.scroll_offset = state.scroll_offset.saturating_sub(1);
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            self.mode = Mode::Normal;
        }
        _ => {}
    }
}

fn enter_diff_view(&mut self) -> Result<()> {
    let rev = self.current_rev();
    let diff_output = cmd!(self.sh, "jj diff -r {rev}").read()?;
    let lines = parse_diff(&diff_output);
    self.mode = Mode::ViewingDiff(DiffState {
        lines,
        scroll_offset: 0,
        rev: rev.to_string(),
    });
    Ok(())
}

fn current_rev(&self) -> &str {
    &self.tree.visible_entries()[self.tree.cursor].node(&self.tree).change_id
}
```

**4. Add diff parsing (could go in a new diff.rs file):**
```rust
fn parse_diff(output: &str) -> Vec<DiffLine> {
    output.lines().map(|line| {
        let style = if line.starts_with("@@") {
            DiffLineStyle::Header
        } else if line.starts_with('+') {
            DiffLineStyle::Added
        } else if line.starts_with('-') {
            DiffLineStyle::Removed
        } else if line.starts_with("diff --git") {
            DiffLineStyle::FileHeader
        } else {
            DiffLineStyle::Context
        };
        DiffLine {
            content: line.to_string(),
            style,
        }
    }).collect()
}
```

**5. Add render_diff in ui.rs:**
```rust
pub fn render(frame: &mut Frame, app: &App) {
    // ... existing layout code ...

    match &app.mode {
        Mode::ViewingDiff(state) => render_diff(frame, state, main_area),
        Mode::Help => {
            render_tree(frame, app, main_area);
            render_help(frame);
        }
        Mode::Normal => render_tree(frame, app, main_area),
    }

    render_status_bar(frame, app, status_area);
}

fn render_diff(frame: &mut Frame, state: &DiffState, area: Rect) {
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Borders, Paragraph};

    let lines: Vec<Line> = state.lines
        .iter()
        .skip(state.scroll_offset)
        .map(|dl| {
            let style = match dl.style {
                DiffLineStyle::Added => Style::default().fg(Color::Green),
                DiffLineStyle::Removed => Style::default().fg(Color::Red),
                DiffLineStyle::Header => Style::default().fg(Color::Cyan),
                DiffLineStyle::FileHeader => Style::default().fg(Color::Yellow),
                DiffLineStyle::Context => Style::default(),
            };
            Line::styled(&dl.content, style)
        })
        .collect();

    let block = Block::default()
        .title(format!(" Diff: {} ", state.rev))
        .borders(Borders::ALL);

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}
```

---

## Phase 8: JJ Lib Helpers Extensions

New methods needed in `jj_lib_helpers.rs`:

```rust
impl JjRepo {
    /// Get the trunk commit
    pub fn trunk(&self) -> Result<Commit>;

    /// Check if working copy has conflicts
    pub fn has_conflicts(&self) -> Result<bool>;

    /// Get list of conflicted files
    pub fn conflicted_files(&self) -> Result<Vec<RepoPathBuf>>;

    /// Get current operation ID (for undo)
    pub fn current_operation_id(&self) -> Result<OperationId>;

    /// Get children of a commit
    pub fn children(&self, commit: &Commit) -> Result<Vec<Commit>>;

    /// Rebase a commit (via CLI for now, jj-lib rebase is complex)
    pub fn rebase_via_cli(&self, sh: &Shell, source: &str,
                          after: Option<&str>, before: Option<&str>) -> Result<()>;
}
```

---

## Key Design Decisions

1. **CLI for mutations, lib for reads**: Use `jj-lib` for reading tree state but shell out to `jj` CLI for mutations (rebase, squash, abandon). This avoids complexity of jj-lib's transaction system.

2. **Simulated previews**: Preview trees are calculated locally by simulating the graph transformation, not by actually running jj commands. This keeps previews instant.

3. **Operation-based undo**: Track jj operation IDs before each action, allowing undo to any point.

4. **Progressive conflict resolution**: Guide users through the new → resolve → squash flow with clear steps.

5. **Minimal UI chrome**: Focus on the tree, use bottom status bar for context, overlay dialogs for complex interactions.

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `cmd/Cargo.toml` | Modify | Add ratatui, crossterm, etc. |
| `cmd/src/cmd/jj.rs` | Modify | Add `Tui` subcommand |
| `cmd/src/cmd/jj_tui.rs` | Create | Module entry point (declares submodules) |
| `cmd/src/cmd/jj_tui/app.rs` | Create | Main app state & event loop |
| `cmd/src/cmd/jj_tui/ui.rs` | Create | Ratatui rendering |
| `cmd/src/cmd/jj_tui/tree.rs` | Create | Tree data structures |
| `cmd/src/cmd/jj_tui/actions.rs` | Create | JJ operation handlers |
| `cmd/src/cmd/jj_tui/preview.rs` | Create | Preview simulation |
| `cmd/src/cmd/jj_tui/conflict.rs` | Create | Conflict resolution |
| `cmd/src/cmd/jj_tui/keybindings.rs` | Create | Key event handling |
| `cmd/src/cmd/jj_tui/widgets.rs` | Create | Widgets module entry point |
| `cmd/src/cmd/jj_tui/widgets/*.rs` | Create | Individual widget files |
| `cmd/src/jj_lib_helpers.rs` | Modify | Add new helper methods |
