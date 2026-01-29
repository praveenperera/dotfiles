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
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }
tui-textarea = "0.7"        # For commit message editing
better-panic = "0.3"        # Panic handler for TUI cleanup
unicode-width = "0.2"       # Text width calculations
```

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
| `J` (Shift+J) | Move revision down (rebase -B) |
| `K` (Shift+K) | Move revision up (rebase -A) |
| `R` | Rebase onto... (prompt for target) |

**Move Down = Insert Before Next Sibling:**
```rust
fn move_down(app: &mut App, rev: &str) -> Result<()> {
    // jj rebase -r REV -B NEXT_SIBLING
    // This inserts REV as a parent of the next sibling
    let next = find_next_sibling(rev)?;
    show_preview(app, PreviewAction::RebaseInsertBefore { rev, target: next });
}
```

**Move Up = Insert After Previous:**
```rust
fn move_up(app: &mut App, rev: &str) -> Result<()> {
    // jj rebase -r REV -A PREV
    // This inserts REV as a child of the previous commit
    let prev = find_previous_commit(rev)?;
    show_preview(app, PreviewAction::RebaseInsertAfter { rev, target: prev });
}
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
| `b` | Create/set bookmark on current |
| `B` | Delete bookmark |
| `p` | Push current bookmark |

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
