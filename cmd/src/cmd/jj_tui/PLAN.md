# JJ Tree TUI - Comprehensive Implementation Plan

## Overview

Building a ratatui-based TUI for jj that extends the existing `jj tree` command with interactive capabilities for managing commits.

---

## Phase 1: Foundation & Core Architecture

### 1.1 Project Structure

```
cmd/src/
├── cmd/
│   ├── jj.rs              # Existing (keep CLI commands)
│   └── jj_tui/            # New TUI module
│       ├── mod.rs         # Module entry, App struct
│       ├── app.rs         # Application state & event loop
│       ├── ui.rs          # Ratatui rendering
│       ├── tree.rs        # Tree data model
│       ├── actions.rs     # JJ operations (rebase, squash, etc.)
│       ├── preview.rs     # Preview state management
│       ├── conflict.rs    # Conflict resolution flow
│       ├── keybindings.rs # Key handling
│       └── widgets/       # Custom ratatui widgets
│           ├── tree_view.rs
│           ├── diff_view.rs
│           ├── preview_panel.rs
│           └── help_dialog.rs
└── jj_lib_helpers.rs      # Extend with new operations
```

### 1.2 Dependencies to Add

```toml
# Cargo.toml additions
ratatui = { version = "0.30", features = ["crossterm"] }
crossterm = { version = "0.29", features = ["event-stream"] }
tui-textarea = "0.7"        # For commit message editing
better-panic = "0.3"        # Panic handler for TUI cleanup
unicode-width = "0.2"       # Text width calculations
```

**Why Crossterm?** (vs Termion/Termwiz)
- Cross-platform: Linux/Mac/Windows (Termion is Unix-only)
- Most popular backend in ratatui ecosystem
- Supports underline colors (Termion doesn't)
- Default in ratatui, best documentation/examples
- ratatui 0.30+ has `crossterm_0_29` feature flag for version flexibility

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
    Preview(PreviewState),
    Editing(EditingState),
    Confirming(ConfirmState),
    Resolving(ConflictState),
    Help,
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
│ ○ master  origin sync point                  kp3x     │
│ ├── feature-a  Add user auth        +2       mn7y     │
│ │   └── @ (working copy)                     qr9z     │
│ │       └── (wip branch)            +1       st4w     │
│ ├── feature-b  Fix login bug                 uv6x     │  <- sibling
│ └── experiment  Try new API                  ab2c     │  <- sibling
├────────────────────────────────────────────────────────┤
│ [j/k] Navigate  [d] Diff  [e] Edit  [r] Rebase  [?] Help│
└────────────────────────────────────────────────────────┘
```

---

## Phase 3: Key Bindings & Actions

### 3.1 Navigation & View

| Key | Action | Description |
|-----|--------|-------------|
| `j` / `↓` | Move down | Navigate to next commit |
| `k` / `↑` | Move up | Navigate to previous commit |
| `g` | Go to top | Jump to trunk |
| `G` | Go to bottom | Jump to last commit |
| `@` | Go to working copy | Jump to @ |
| `Enter` | Toggle expand | Expand/collapse children (optional) |
| `f` | Toggle full mode | Show/hide commits without bookmarks |

### 3.2 View Diff (Action 1)

| Key | Action |
|-----|--------|
| `d` | View diff of selected revision |
| `D` | View diff in external tool (delta/difftastic) |

**Implementation:**
```rust
fn view_diff(app: &App, rev: &str) -> Result<()> {
    // Show diff in a scrollable panel or spawn pager
    let diff = cmd!(app.shell, "jj diff -r {rev} --color=always").read()?;
    app.mode = Mode::ViewingDiff(DiffState { content: diff, scroll: 0 });
}
```

### 3.3 Rename/Edit Commit Message (Action 2)

| Key | Action |
|-----|--------|
| `e` | Edit commit message |
| `E` | Edit in $EDITOR |

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

### 3.4 Move Revision Up/Down (Action 3)

| Key | Action |
|-----|--------|
| `J` (Shift+J) | Move single revision down (`rebase -r ... -B`) |
| `K` (Shift+K) | Move single revision up (`rebase -r ... -A`) |
| `Ctrl+J` | Move revision + descendants down (`rebase -s ... -B`) |
| `Ctrl+K` | Move revision + descendants up (`rebase -s ... -A`) |
| `R` | Rebase onto... (prompt for target, choose -r or -s) |

**Single Revision (`-r`) vs Source + Descendants (`-s`):**

```
Stack before:          -r (single)           -s (with descendants)
○ A                    ○ A                   ○ A
├── B ← move down      ├── C                 ├── C
│   └── C              ├── B (moved alone)   │   └── D
│       └── D          │   └── D (orphaned)  └── B (moved with C, D)
└── E                  └── E                     └── E
```

- **`-r` (single)**: Only moves the selected commit; its children get reparented
- **`-s` (source)**: Moves the commit AND all its descendants together

**Implementation:**
```rust
pub enum RebaseMode {
    SingleRevision,      // -r: just this commit
    SourceWithDescendants, // -s: this commit + all descendants
}

fn move_down(app: &mut App, rev: &str, mode: RebaseMode) -> Result<()> {
    let flag = match mode {
        RebaseMode::SingleRevision => "-r",
        RebaseMode::SourceWithDescendants => "-s",
    };
    // jj rebase {flag} REV -B NEXT_SIBLING
    let next = find_next_sibling(rev)?;
    show_preview(app, PreviewAction::Rebase {
        rev, target: next, mode, direction: Direction::Before
    });
}

fn move_up(app: &mut App, rev: &str, mode: RebaseMode) -> Result<()> {
    let flag = match mode {
        RebaseMode::SingleRevision => "-r",
        RebaseMode::SourceWithDescendants => "-s",
    };
    // jj rebase {flag} REV -A PREV
    let prev = find_previous_commit(rev)?;
    show_preview(app, PreviewAction::Rebase {
        rev, target: prev, mode, direction: Direction::After
    });
}
```

**Preview shows which mode:**
```
┌─ Rebase Preview ──────────────────────────────────────┐
│ Moving "fix login" + 2 descendants down              │
│ Mode: -s (source with descendants)                    │
│                                                        │
│ Command: jj rebase -s abc -B def                      │
└────────────────────────────────────────────────────────┘
```

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
| `Space` | Toggle selection on current commit |
| `v` | Enter visual/multi-select mode |
| `a` | Abandon selected (or current if none selected) |
| `Esc` | Clear selection |

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
| `s` | Squash into parent |
| `S` | Squash into... (select target) |

**Implementation:**
```rust
fn squash_into(app: &mut App, source: &str, target: &str) -> Result<()> {
    show_preview(app, PreviewAction::Squash { source, into: target });
}
```

### 3.7 Cross-Stack Movement (Rebase Mode)

Moving revisions between different stacks (not just up/down within the same stack).

| Key | Action |
|-----|--------|
| `r` | Enter **Rebase Mode** - select destination for current revision |
| `Esc` | Exit Rebase Mode |

**Rebase Mode:**

When you press `r`, the TUI enters a special mode where:
1. Current revision is highlighted as "source"
2. Navigate with `j/k` to any revision in the tree (including other stacks)
3. Press `Enter` to rebase source onto the selected destination
4. Press `a` for "after" (`-A`), `b` for "before" (`-B`), `d` for "destination" (`-d`)

```
┌─ Rebase Mode ─────────────────────────────────────────┐
│ Moving: "fix login" (abc123)                          │
│                                                        │
│ ○ master                                              │
│ ├── feature-auth                                      │
│ │   └── [SOURCE] fix login  ← you are moving this    │
│ ├── feature-api             ← cursor here            │
│ │   └── add endpoints                                 │
│ └── feature-ui                                        │
│                                                        │
│ [a] Insert after   [b] Insert before   [d] As child   │
│ [s] Toggle -s mode   [Esc] Cancel                     │
└────────────────────────────────────────────────────────┘
```

**Implementation:**
```rust
pub struct RebaseModeState {
    pub source_rev: String,
    pub source_mode: RebaseMode,  // -r or -s
    pub destination_cursor: usize,
}

pub enum RebasePosition {
    After,   // -A: insert as child of destination
    Before,  // -B: insert as parent of destination
    Onto,    // -d: destination becomes parent (standard rebase)
}

fn enter_rebase_mode(app: &mut App) {
    let source = app.current_rev().clone();
    app.mode = Mode::Rebasing(RebaseModeState {
        source_rev: source,
        source_mode: RebaseMode::SingleRevision,
        destination_cursor: app.tree.cursor,
    });
}

fn execute_cross_stack_rebase(
    source: &str,
    dest: &str,
    mode: RebaseMode,
    position: RebasePosition
) -> Result<()> {
    let flag = match mode {
        RebaseMode::SingleRevision => "-r",
        RebaseMode::SourceWithDescendants => "-s",
    };
    let pos_flag = match position {
        RebasePosition::After => "-A",
        RebasePosition::Before => "-B",
        RebasePosition::Onto => "-d",
    };
    cmd!(sh, "jj rebase {flag} {source} {pos_flag} {dest}").run()?;
}
```

**Visual Feedback:**

When in Rebase Mode, the tree view changes:
- Source revision: highlighted in yellow with `[SOURCE]` marker
- Valid destinations: normal color
- Invalid destinations (ancestors of source when using -s): dimmed/grayed out
- Cursor position: highlighted with different color

**Quick Cross-Stack Shortcuts:**

For common patterns, provide shortcuts without entering full Rebase Mode:

| Key | Action |
|-----|--------|
| `t` | Rebase current onto trunk (like `jj ro` but for single rev) |
| `T` | Rebase current + descendants onto trunk |

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
        cmd!(app.shell, "jj undo --to {op_id}").run()?;
        refresh_tree(app)?;
        app.message = Some(("Operation undone".into(), MessageType::Info));
    }
}
```

---

## Phase 6: Additional Features

### 6.1 Help Dialog

```rust
fn show_help(app: &mut App) {
    app.mode = Mode::Help;
}

// Rendered as overlay:
// ┌─ Help ─────────────────────────┐
// │ Navigation                     │
// │   j/k     Up/Down              │
// │   g/G     Top/Bottom           │
// │   @       Go to working copy   │
// │                                │
// │ Actions                        │
// │   d       View diff            │
// │   e       Edit message         │
// │   J/K     Move down/up         │
// │   s       Squash into parent   │
// │   Space   Toggle select        │
// │   a       Abandon selected     │
// │                                │
// │ [q] Close                      │
// └────────────────────────────────┘
```

### 6.2 Status Bar Messages

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

### 6.3 Bookmarks Quick Actions

| Key | Action |
|-----|--------|
| `b` | Create new bookmark on current revision |
| `B` | Delete bookmark (select from list if multiple) |
| `m` | **Move bookmark to current revision** |
| `p` | Push current bookmark to remote |
| `P` | Push all bookmarks |

**Move Bookmark Flow (`m` key):**

This is a common operation - moving an existing bookmark to a different revision.

```rust
fn move_bookmark(app: &mut App) -> Result<()> {
    // 1. Show list of all bookmarks
    // 2. User selects which bookmark to move
    // 3. Bookmark is set to current cursor position
    let bookmarks = get_all_bookmarks()?;
    app.mode = Mode::SelectingBookmark(SelectBookmarkState {
        bookmarks,
        selected: 0,
        action: BookmarkAction::MoveToCurrent,
    });
}

// After selection:
fn apply_bookmark_move(bookmark: &str, target_rev: &str) -> Result<()> {
    cmd!(sh, "jj bookmark set {bookmark} -r {target_rev}").run()?;
}
```

**UI for Bookmark Selection:**
```
┌─ Move Bookmark ───────────────────────────────────────┐
│ Select bookmark to move to "fix login" (abc123):      │
│                                                        │
│ > feature-auth     (currently on def456)              │
│   feature-api      (currently on ghi789)              │
│   wip-refactor     (currently on jkl012)              │
│                                                        │
│ [Enter] Move   [Esc] Cancel   [n] New bookmark        │
└────────────────────────────────────────────────────────┘
```

**Alternative: Mark and Move Pattern**

For power users who want to move bookmarks without a dialog:

| Key | Action |
|-----|--------|
| `'` (quote) | Mark current revision |
| `m` then `'` | Move bookmark from marked to cursor |

```rust
// Mark a revision for later reference
fn mark_revision(app: &mut App) {
    app.marked_revision = Some(app.current_rev().clone());
    app.message = Some(("Marked revision".into(), MessageType::Info));
}
```

---

## Phase 7: Implementation Order

### Sprint 1: Skeleton
1. Add ratatui/crossterm dependencies
2. Create basic TUI app structure with event loop
3. Port tree data loading from existing `tree()` function
4. Render static tree with navigation (j/k)
5. Add trunk as base, handle full tree query

### Sprint 2: View Operations
1. Implement diff viewing (`d` key)
2. Add help dialog (`?` key)
3. Implement status bar messages
4. Add scroll support for long trees

### Sprint 3: Edit Operations
1. Implement description editing (`e` key)
2. Add multi-select mode (`Space`, `v`)
3. Implement abandon with preview (`a` key)

### Sprint 4: Rebase Operations
1. Implement move up/down (`J`/`K`)
2. Build preview system (side-by-side before/after)
3. Add confirmation flow

### Sprint 5: Squash & Conflicts
1. Implement squash operations (`s`/`S`)
2. Build conflict detection
3. Implement automatic new → resolve → squash flow
4. Add undo capability

### Sprint 6: Polish
1. Add bookmark operations
2. Verify preview matches actual result
3. Error handling & edge cases
4. Testing with various repo states

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
| `cmd/src/cmd/jj_tui/mod.rs` | Create | Module entry point |
| `cmd/src/cmd/jj_tui/app.rs` | Create | Main app state & event loop |
| `cmd/src/cmd/jj_tui/ui.rs` | Create | Ratatui rendering |
| `cmd/src/cmd/jj_tui/tree.rs` | Create | Tree data structures |
| `cmd/src/cmd/jj_tui/actions.rs` | Create | JJ operation handlers |
| `cmd/src/cmd/jj_tui/preview.rs` | Create | Preview simulation |
| `cmd/src/cmd/jj_tui/conflict.rs` | Create | Conflict resolution |
| `cmd/src/cmd/jj_tui/keybindings.rs` | Create | Key event handling |
| `cmd/src/jj_lib_helpers.rs` | Modify | Add new helper methods |
