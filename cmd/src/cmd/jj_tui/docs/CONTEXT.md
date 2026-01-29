# JJ TUI - Context for Development

Important context and learnings for continuing development on this TUI.

---

## Module Structure

The TUI lives in `cmd/src/cmd/jj_tui/` with entry point at `cmd/src/cmd/jj_tui.rs`.

**Key files:**
- `jj_tui.rs` - Module declarations and `pub fn run(sh: &Shell)`
- `jj_tui/app.rs` - `App` struct, event loop, mode enum, key handling
- `jj_tui/tree.rs` - `TreeNode`, `TreeState`, `VisibleEntry`, tree loading
- `jj_tui/ui.rs` - All ratatui rendering functions
- `jj_tui/keybindings.rs` - Stub for future keymap configuration

---

## Critical Implementation Details

### 1. Revset for Tree Loading

The revset MUST include trunk itself to show the full tree:

```rust
// CORRECT - includes trunk as root
"trunk() | descendants(roots(trunk()..@)) | @::"

// WRONG - misses trunk, tree appears truncated
"descendants(roots(trunk()..@))"
```

### 2. Root Detection

Trunk must be explicitly detected and treated as a root:

```rust
let trunk_id = jj_repo
    .eval_revset_single("trunk()")
    .ok()
    .and_then(|c| jj_repo.shortest_change_id(&c, 4).ok());

// In root filtering:
if let Some(ref tid) = trunk_id {
    if c.change_id == *tid {
        return true;  // Always include trunk as root
    }
}
```

### 3. Visual Depths for Non-Full Mode

When hiding commits without bookmarks, visual depths must be recomputed:

```rust
pub struct VisibleEntry {
    pub node_index: usize,
    pub visual_depth: usize,  // NOT the same as node.depth
}

// In full mode: visual_depth = node.depth (structural)
// In non-full mode: visual_depth = count of visible ancestors
```

This prevents excessive indentation when intermediate commits are hidden.

### 4. crossterm Import

Use ratatui's re-export, NOT a direct dependency:

```rust
// CORRECT
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

// WRONG - requires adding crossterm to Cargo.toml
use crossterm::event::{...};
```

### 5. ratatui 0.30 Terminal Setup

Use the new simplified API with built-in panic hooks:

```rust
pub fn run(&mut self) -> Result<()> {
    let mut terminal = ratatui::init();  // Sets up terminal + panic hooks
    let result = self.run_loop(&mut terminal);
    ratatui::restore();  // Restores terminal state
    result
}
```

---

## Existing Infrastructure

### JjRepo (jj_lib_helpers.rs)

The `JjRepo` struct provides jj-lib access for **reading** data:

```rust
let jj_repo = JjRepo::load(None)?;  // Load from current directory

// Useful methods:
jj_repo.working_copy_commit()?;
jj_repo.eval_revset("revset_string")?;  // Returns Vec<Commit>
jj_repo.eval_revset_single("@")?;       // Returns single Commit
jj_repo.shortest_change_id(&commit, min_len)?;
jj_repo.change_id_with_prefix_len(&commit, min_len)?;  // Returns (id, unique_len)
jj_repo.bookmarks_at(&commit);  // Returns Vec<String>
jj_repo.parent_commits(&commit)?;
JjRepo::description_first_line(&commit);
```

**Important:** `JjRepo` has **NO diff functionality**. Use CLI for diffs:

```rust
// Get diff for a revision
let diff = cmd!(sh, "jj diff -r {rev}").read()?;

// Get diff with specific file
let diff = cmd!(sh, "jj diff -r {rev} -- {file}").read()?;
```

### Shell Commands (xshell)

For mutations and diffs, use `xshell` via the shell stored in `App`:

```rust
use xshell::{cmd, Shell};

// Mutations
cmd!(sh, "jj rebase -r {rev} -A {dest}").run()?;
cmd!(sh, "jj edit {rev}").run()?;

// Reading output (for diffs)
let output = cmd!(sh, "jj diff -r {rev}").read()?;
```

### Scroll Infrastructure

The event loop already calls `update_scroll()` with viewport height:

```rust
// In app.rs run_loop()
let viewport_height = terminal.size()?.height.saturating_sub(3) as usize;
self.tree.update_scroll(viewport_height);
```

This keeps cursor visible. To add page scrolling (Ctrl+u/d), add handlers that move cursor by `viewport_height / 2`.

---

## Rendering Architecture

### Frame Layout

```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(1)])
    .split(frame.area());

render_tree(frame, app, chunks[0]);      // Main tree area
render_status_bar(frame, app, chunks[1]); // Bottom status bar

if matches!(app.mode, Mode::Help) {
    render_help(frame);  // Overlay popup
}
```

### Tree Line Rendering

Each visible entry is rendered with:
- Indent based on visual_depth
- Connector (`├──` or nothing for root)
- Working copy marker (`@`)
- Bookmarks (cyan) or change_id (magenta prefix + dimmed suffix)
- Description (dimmed)
- Selection highlight (background color)

---

## Testing

Build and test from the cmd directory:

```bash
cd /Users/praveen/code/dotfiles/cmd
cargo build --release
cargo clippy

# Test TUI (requires jj repo)
cd /path/to/jj/repo
/Users/praveen/code/dotfiles/cmd/target/release/cmd jj

# Test CLI still works
cmd jj tree --full
```

---

## Sprint 2 Implementation Notes (Complete)

### 1. Diff viewing (D key)

Mode::ViewingDiff is a simple enum variant (Copy), with DiffState stored separately:
```rust
pub enum Mode {
    Normal,
    Help,
    ViewingDiff,  // State stored in App.diff_state
}

pub struct App {
    // ...
    pub diff_state: Option<DiffState>,
}
```

### 2. Details panel (Space key)

Expanded state tracked per cursor position, not per node:
```rust
pub struct TreeState {
    pub expanded_entry: Option<usize>,  // Index of expanded entry
}
```

This simplifies implementation - only one entry can be expanded at a time

### 3. Scroll support (Ctrl+u/d)

KeyModifiers from ratatui::crossterm must be imported and checked:
```rust
use ratatui::crossterm::event::{KeyModifiers, ...};

fn handle_normal_key(&mut self, key: event::KeyEvent, viewport_height: usize) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('u') if ctrl => self.tree.page_up(viewport_height / 2),
        KeyCode::Char('d') if ctrl => self.tree.page_down(viewport_height / 2),
        // ...
    }
}
```

### 4. Multi-pane layout (\\ key)

Split view renders a placeholder in the diff pane:
```rust
fn render_diff_pane(frame: &mut Frame, _app: &App, area: Rect) {
    // Shows "Press D to view full diff"
}
```

Future enhancement: cache diff data and auto-update when cursor moves

## Sprint 3 Implementation Notes (Complete)

### 1. Edit Description (d key)

Multi-line text editor with cursor support:
```rust
pub struct EditingState {
    pub text: String,
    pub cursor: usize,       // byte offset
    pub target_rev: String,
    pub original_desc: String,
}
```

Saves with `Ctrl+Enter`, cancels with `Esc`. Uses `jj desc -r {rev} -m {desc}` to save.

### 2. Edit Working Copy (e key)

Uses `jj edit {rev}` to switch working copy to selected revision.

### 3. New Commit / Commit (n/c keys)

- `n` creates new commit after selected with `jj new {rev}`
- `c` commits working copy with `jj commit -m {desc}`

### 4. Multi-select Mode (x/v keys)

Selection tracked in TreeState:
```rust
pub selected: HashSet<usize>,      // indices of selected visible entries
pub selection_anchor: Option<usize>, // for visual mode range selection
```

- `x` toggles individual selection
- `v` enters visual mode, `j`/`k` extends selection from anchor to cursor

### 5. Abandon with Confirmation (a key)

Confirmation dialog pattern:
```rust
pub struct ConfirmState {
    pub action: ConfirmAction,
    pub message: String,
    pub revs: Vec<String>,  // affected revisions
}
```

Uses `jj abandon {revset}` where revset joins selected revisions with ` | `.

## Sprint 4 Implementation Notes (Rebase Operations)

### 1. Rebase Mode (r/s keys)

Rebase mode state:
```rust
pub struct RebaseState {
    pub source_rev: String,           // revision being moved
    pub rebase_type: RebaseType,      // Single or WithDescendants
    pub dest_cursor: usize,           // destination cursor position
    pub allow_branches: bool,         // toggle for branch mode
    pub op_before: String,            // operation ID for undo
}

pub enum RebaseType {
    Single,          // -r: just this revision
    WithDescendants, // -s: revision + all descendants
}
```

### 2. Rebase Execution

Clean inline rebase (default):
```rust
// insert between dest and its first child
jj rebase -r {source} -A {dest} -B {next}
```

Branch mode (allow_branches = true):
```rust
// simple -A only, creates branch point
jj rebase -r {source} -A {dest}
```

### 3. Quick Rebase (t/T keys)

- `t` rebases current revision onto trunk: `jj rebase -r @ -d trunk()`
- `T` rebases current + descendants onto trunk: `jj rebase -s @ -d trunk()`

### 4. Undo Support (u key)

After rebase, stores `op_before` operation ID. If conflicts detected or user presses `u`:
```rust
jj op restore {op_before}
```

---

### 5. Rebase Preview Rendering

In rebase mode, the tree shows a **preview** of the final state. The behavior differs between 'r' (single) and 's' (with descendants) modes:

**'r' mode (single rebase):**
- Initial dest is set to source's parent, so tree looks unchanged when first pressing 'r'
- On j/k navigation, source moves to after dest with `dest_depth + 1`
- Entries between dest and source shift down by 1 depth level
- All other entries keep their original depth
- Only source is highlighted as moving

**'s' mode (rebase with descendants):**
- Source AND all descendants move together after dest
- Source gets `dest_depth + 1`, descendants keep relative depth from source
- Entries between dest and source shift down by the size of the moving stack (so they slot in after the descendants)
- Uses structural depth (node.depth) for descendant detection, not visual_depth
- This fixes depth calculation in non-full mode where visual_depth is compressed

**Visual indicators:**
- Destination is highlighted with `►` marker and cursor style
- Moving commits are highlighted with yellow change_id and distinct background

This is handled by `build_rebase_preview()` in ui.rs.

---

## JJ Command Syntax Notes

**Important:** jj CLI flags can be tricky. Here are correct patterns:

```rust
// Limiting output - use --limit, NOT -l
jj log --limit 1 -T change_id --no-graph
jj op log --limit 1 -T id --no-graph

// Operation log template - use "id", NOT "operation_id"
jj op log -T id

// Restoring to previous operation - use "jj op restore", NOT "jj undo --to"
jj op restore {op_id}
```

---

## Known Quirks

1. The CLI `tree` command has a `--from` flag that TUI doesn't support yet
2. `jjtf` is likely an alias for `cmd jj tree --full`
3. The user prefers `color-eyre` over `anyhow` for error handling
4. Comments should not end with periods per CLAUDE.md
5. Don't add "Created by Claude" to files
