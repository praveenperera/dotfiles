# Preview Rendering Refactor Plan

## Executive Summary

The current preview rendering system in the jj TUI has grown complex and difficult to maintain. The `build_rebase_preview()` function is 135 lines of imperative index manipulation with two separate code paths. This document outlines a refactoring to a **tree-based data model** that separates concerns, simplifies rendering, and enables unit testing without visual verification.

**Key Benefits:**
- Single source of truth for tree structure
- Operations compose naturally (rebase, squash, fork, merge)
- Depth calculation emerges from traversal (no manual tracking)
- One unified rendering function (currently two near-duplicates)
- Testable without visual verification

---

## Current State Analysis

### Problem 1: Two Separate Code Paths

`build_rebase_preview()` in `ui.rs` (lines 254-389) has completely different implementations for `RebaseType::Single` vs `RebaseType::WithDescendants`:

```rust
// Single mode: ~40 lines of manual index manipulation
if rebase_type == RebaseType::Single {
    for (idx, entry) in app.tree.visible_entries.iter().enumerate() {
        if idx == source_index { continue; }
        let depth = if idx > dest_cursor && idx < source_index {
            entry.visual_depth + 1
        } else {
            entry.visual_depth
        };
        // ... build preview entry
    }
    return preview;
}

// WithDescendants mode: ~70 lines of different manipulation
let mut moving_indices = HashSet::new();
// ... completely different logic
```

Every bug fix must be applied twice. The git history shows multiple fixes to this code.

### Problem 2: Manual Index Arithmetic

Depth calculations use raw index comparisons that are error-prone:

```rust
// "entries between dest and source shift down by 1 depth"
let depth = if idx > dest_cursor && idx < source_index {
    entry.visual_depth + 1
} else {
    entry.visual_depth
};
```

This arithmetic is hard to reason about and has been a source of bugs.

### Problem 3: Two Nearly-Identical Rendering Functions

`render_tree_line_with_markers()` (lines 178-251) and `render_tree_line_rebase()` (lines 431-504) share ~80% of their code:
- Same indent calculation
- Same connector logic
- Same change_id coloring structure
- Same bookmark display
- Same description formatting

Only markers and background colors differ.

### Problem 4: No Clean Path to Extend

Adding new operations (squash preview, `-A`/`-B` flags) would require yet another code path. The current squash mode has no visual tree transformation.

---

## Proposed Architecture

### Core Principle: Separate Structure from Rendering

```
TreeNode[]              # Raw commit data (immutable after load)
     ↓
TreeRelations           # Tree structure (can be cloned & modified)
     ↓
Preview                 # Modified structure + roles for styling
     ↓
Vec<DisplaySlot>        # Flat list ready to render
     ↓
render_tree_line()      # Single unified rendering function
```

### Data Structures

```rust
/// Unique identifier for a node - index into node storage
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct NodeId(pub usize);

/// Tree structure that can be freely manipulated
#[derive(Clone, Debug)]
pub struct TreeRelations {
    /// Root nodes (no parent in visible set)
    roots: Vec<NodeId>,
    /// Parent of each node
    parent: HashMap<NodeId, NodeId>,
    /// Ordered children of each node
    children: HashMap<NodeId, Vec<NodeId>>,
}

/// How to place a node relative to another
#[derive(Clone, Debug)]
pub enum Placement {
    /// Become a child of target
    /// jj: rebase -o {target}
    ChildOf(NodeId),

    /// Become sibling after target (creates fork)
    /// jj: rebase -A {target}
    SiblingAfter(NodeId),

    /// Insert as parent of target
    /// jj: rebase -B {target}
    InsertAbove(NodeId),

    /// Insert between target and its children (clean inline)
    /// jj: rebase -A {target} -B {child}
    InlineAfter(NodeId),
}

/// Visual role for styling
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeRole {
    Normal,
    Source,      // Commit being moved/squashed
    Destination, // Target of operation
    Moving,      // Descendant moving with source
    Removed,     // Source being absorbed (squash)
    Shifted,     // Position changed due to operation
}

/// Display-ready entry
#[derive(Clone, Debug)]
pub struct DisplaySlot {
    pub node: NodeId,
    pub depth: usize,
    pub visual_depth: usize,
    pub role: NodeRole,
}

/// Complete preview state
pub struct Preview {
    pub relations: TreeRelations,
    pub slots: Vec<DisplaySlot>,
    pub roles: HashMap<NodeId, NodeRole>,
}
```

### Core Operations

```rust
impl TreeRelations {
    /// Remove node from current position (keeps children attached to node)
    fn detach(&mut self, node: NodeId) {
        if let Some(parent) = self.parent.remove(&node) {
            self.children.get_mut(&parent)
                .map(|c| c.retain(|&n| n != node));
        }
        self.roots.retain(|&n| n != node);
    }

    /// Attach node at new position
    fn attach(&mut self, node: NodeId, placement: Placement) {
        match placement {
            Placement::ChildOf(parent) => {
                self.parent.insert(node, parent);
                self.children.entry(parent).or_default().push(node);
            }
            Placement::SiblingAfter(sibling) => {
                if let Some(&parent) = self.parent.get(&sibling) {
                    self.parent.insert(node, parent);
                    let siblings = self.children.entry(parent).or_default();
                    let pos = siblings.iter().position(|&n| n == sibling)
                        .map(|i| i + 1).unwrap_or(siblings.len());
                    siblings.insert(pos, node);
                } else {
                    // sibling is root - node becomes root after it
                    let pos = self.roots.iter().position(|&n| n == sibling)
                        .map(|i| i + 1).unwrap_or(self.roots.len());
                    self.roots.insert(pos, node);
                }
            }
            Placement::InsertAbove(target) => {
                // Node takes target's position, target becomes child of node
                if let Some(parent) = self.parent.remove(&target) {
                    self.parent.insert(node, parent);
                    self.children.get_mut(&parent).map(|c| {
                        if let Some(pos) = c.iter().position(|&n| n == target) {
                            c[pos] = node;
                        }
                    });
                } else {
                    if let Some(pos) = self.roots.iter().position(|&n| n == target) {
                        self.roots[pos] = node;
                    }
                }
                self.parent.insert(target, node);
                self.children.entry(node).or_default().push(target);
            }
            Placement::InlineAfter(target) => {
                // Insert between target and its children
                let children = self.children.remove(&target).unwrap_or_default();
                self.parent.insert(node, target);
                self.children.entry(target).or_default().push(node);
                for child in &children {
                    self.parent.insert(*child, node);
                }
                self.children.insert(node, children);
            }
        }
    }

    /// Move a node to a new position
    pub fn rebase(&mut self, node: NodeId, placement: Placement) {
        self.detach(node);
        self.attach(node, placement);
    }

    /// Move node and all descendants together
    pub fn rebase_with_descendants(&mut self, node: NodeId, placement: Placement) {
        // Descendants stay attached - only detach/attach the root
        self.detach(node);
        self.attach(node, placement);
    }

    /// Squash: merge source into dest, reparent source's children
    pub fn squash(&mut self, source: NodeId, dest: NodeId) {
        let children: Vec<_> = self.children.get(&source)
            .cloned().unwrap_or_default();
        for child in children {
            self.rebase(child, Placement::ChildOf(dest));
        }
        self.detach(source);
    }

    /// DFS traversal yielding (node, depth)
    pub fn dfs_iter(&self) -> impl Iterator<Item = (NodeId, usize)> + '_ {
        DfsIterator::new(self)
    }
}
```

---

## jj Rebase Flags Reference

### Core Flags

| Flag | Purpose | Description |
|------|---------|-------------|
| `-r` | Revision selection | Rebase single revision(s), descendants auto-fill gap |
| `-s` | Source + descendants | Rebase revision and all its descendants as a unit |
| `-o` | Destination | Target to rebase onto (standard rebase) |
| `-A` | Insert after | Position flag: insert after target (becomes sibling) |
| `-B` | Insert before | Position flag: insert before target (becomes parent of target) |

### Flag Combinations and Effects

#### Standard Rebase (`-o` only)
```
Before:               After (jj rebase -r C -o A):
A                     A
├── B                 ├── B
│   └── C ← source    └── C ← moved
└── D                 └── D
```
Source becomes **child of** destination.

#### Insert After (`-A` without `-B`)
```
Before:               After (jj rebase -r D -A B):
A                     A
├── B                 └── B
│   └── C                 ├── C
└── D ← source            └── D ← moved (sibling of C)
```
Source becomes **sibling after** target. Creates a fork point.

#### Insert Before (`-B`)
```
Before:               After (jj rebase -r D -B C):
A                     A
├── B                 └── B
│   └── C                 └── D ← moved (now parent of C)
└── D ← source                └── C
```
Source becomes **parent of** target. Inserts into the chain.

#### Inline Insertion (`-A` with `-B`)
```
Before:               After (jj rebase -r D -A B -B C):
A                     A
├── B                 └── B
│   └── C                 └── D ← moved (between B and C)
└── D ← source                └── C
```
Source inserts **between** target and its children. Clean linearization.

### Mapping to Placement Enum

| jj Command | Placement |
|------------|-----------|
| `rebase -o X` | `ChildOf(X)` |
| `rebase -A X` | `SiblingAfter(X)` |
| `rebase -B X` | `InsertAbove(X)` |
| `rebase -A X -B Y` | `InlineAfter(X)` |

---

## Implementation Plan

### Phase 1: Core Data Structures

**Files:** Create `/home/user/dotfiles/cmd/src/cmd/jj_tui/preview.rs`

**Tasks:**
1. Implement `NodeId`, `TreeRelations`, `Placement`, `NodeRole`, `DisplaySlot`
2. Implement `TreeRelations::detach()`, `attach()`, `rebase()`
3. Implement `DfsIterator` for traversal
4. Add unit tests (see Test Plan)

**Why:** Establishes the foundation. All other work depends on these primitives being correct.

### Phase 2: Preview Builder

**Files:** Continue in `preview.rs`

**Tasks:**
1. Implement `TreeRelations::from_tree_state()` to convert existing state
2. Implement `PreviewBuilder::rebase_preview()` for both `-r` and `-s` modes
3. Implement `PreviewBuilder::squash_preview()`
4. Add unit tests for preview generation

**Why:** The builder provides a clean API for creating previews from operations.

### Phase 3: Unified Rendering

**Files:** Modify `ui.rs`

**Tasks:**
1. Create single `render_tree_line(node, depth, role)` function
2. Remove `render_tree_line_with_markers()` (lines 178-251)
3. Remove `render_tree_line_rebase()` (lines 431-504)
4. Update `render_tree_with_preview()` to use `Preview` type

**Why:** Eliminates code duplication. Role-based styling is cleaner than mode-specific branches.

### Phase 4: Integration

**Files:** Modify `ui.rs`, `app.rs`

**Tasks:**
1. Replace `build_rebase_preview()` calls with `PreviewBuilder::rebase_preview()`
2. Add squash preview using `PreviewBuilder::squash_preview()`
3. Update bookmark move mode to use new structure
4. Manual testing of all modes

**Why:** Connects new preview system to existing application flow.

### Phase 5: Cleanup

**Files:** `ui.rs`, `app.rs`, `CONTEXT.md`

**Tasks:**
1. Remove old `PreviewEntry` struct
2. Remove `build_rebase_preview()` function
3. Remove `compute_moving_indices()` from `app.rs`
4. Update documentation

**Why:** Remove dead code, update docs to reflect new architecture.

---

## Test Plan

### Unit Tests (No Visual Verification Needed)

Create `/home/user/dotfiles/cmd/src/cmd/jj_tui/preview.rs` with `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // === Test Fixtures ===

    fn linear_tree() -> TreeRelations {
        // A → B → C → D
        let mut tr = TreeRelations::new();
        tr.roots.push(NodeId(0)); // A
        tr.add_child(NodeId(0), NodeId(1)); // A → B
        tr.add_child(NodeId(1), NodeId(2)); // B → C
        tr.add_child(NodeId(2), NodeId(3)); // C → D
        tr
    }

    fn forked_tree() -> TreeRelations {
        // A ─┬─ B → C
        //    └─ D → E
        let mut tr = TreeRelations::new();
        tr.roots.push(NodeId(0)); // A
        tr.add_child(NodeId(0), NodeId(1)); // A → B
        tr.add_child(NodeId(1), NodeId(2)); // B → C
        tr.add_child(NodeId(0), NodeId(3)); // A → D
        tr.add_child(NodeId(3), NodeId(4)); // D → E
        tr
    }

    // === Detach Tests ===

    #[test]
    fn detach_removes_from_parent() {
        let mut tr = linear_tree();
        tr.detach(NodeId(1)); // Remove B
        assert!(!tr.children[&NodeId(0)].contains(&NodeId(1)));
        assert_eq!(tr.parent.get(&NodeId(1)), None);
    }

    #[test]
    fn detach_preserves_descendants() {
        let mut tr = linear_tree();
        tr.detach(NodeId(1)); // Remove B
        // C should still be child of B
        assert_eq!(tr.parent.get(&NodeId(2)), Some(&NodeId(1)));
    }

    #[test]
    fn detach_root() {
        let mut tr = linear_tree();
        tr.detach(NodeId(0)); // Remove root A
        assert!(!tr.roots.contains(&NodeId(0)));
    }

    // === Rebase ChildOf Tests ===

    #[test]
    fn rebase_child_of_basic() {
        let mut tr = linear_tree();
        // Move D to be child of A
        tr.rebase(NodeId(3), Placement::ChildOf(NodeId(0)));
        assert_eq!(tr.parent[&NodeId(3)], NodeId(0));
        assert!(tr.children[&NodeId(0)].contains(&NodeId(3)));
    }

    #[test]
    fn rebase_child_of_removes_from_old_parent() {
        let mut tr = linear_tree();
        tr.rebase(NodeId(3), Placement::ChildOf(NodeId(0)));
        assert!(!tr.children[&NodeId(2)].contains(&NodeId(3)));
    }

    // === Rebase SiblingAfter Tests ===

    #[test]
    fn rebase_sibling_after_same_parent() {
        let mut tr = linear_tree();
        // Move D to be sibling of B (both children of A)
        tr.rebase(NodeId(3), Placement::SiblingAfter(NodeId(1)));
        assert_eq!(tr.parent[&NodeId(3)], NodeId(0));
    }

    #[test]
    fn rebase_sibling_after_ordering() {
        let mut tr = linear_tree();
        tr.rebase(NodeId(3), Placement::SiblingAfter(NodeId(1)));
        let children = &tr.children[&NodeId(0)];
        let b_pos = children.iter().position(|&n| n == NodeId(1)).unwrap();
        let d_pos = children.iter().position(|&n| n == NodeId(3)).unwrap();
        assert!(d_pos > b_pos, "D should come after B");
    }

    #[test]
    fn rebase_sibling_after_root() {
        let mut tr = linear_tree();
        // Move B to be sibling of root A
        tr.rebase(NodeId(1), Placement::SiblingAfter(NodeId(0)));
        assert!(tr.roots.contains(&NodeId(1)));
        let a_pos = tr.roots.iter().position(|&n| n == NodeId(0)).unwrap();
        let b_pos = tr.roots.iter().position(|&n| n == NodeId(1)).unwrap();
        assert!(b_pos > a_pos);
    }

    // === Rebase InsertAbove Tests ===

    #[test]
    fn rebase_insert_above_basic() {
        let mut tr = linear_tree();
        // Move D above B (D becomes parent of B)
        tr.rebase(NodeId(3), Placement::InsertAbove(NodeId(1)));
        assert_eq!(tr.parent[&NodeId(1)], NodeId(3)); // B's parent is now D
        assert_eq!(tr.parent[&NodeId(3)], NodeId(0)); // D's parent is A
    }

    #[test]
    fn rebase_insert_above_root() {
        let mut tr = linear_tree();
        // Move D above A (D becomes new root, A becomes child)
        tr.rebase(NodeId(3), Placement::InsertAbove(NodeId(0)));
        assert!(tr.roots.contains(&NodeId(3)));
        assert!(!tr.roots.contains(&NodeId(0)));
        assert_eq!(tr.parent[&NodeId(0)], NodeId(3));
    }

    // === Rebase InlineAfter Tests ===

    #[test]
    fn rebase_inline_after() {
        let mut tr = linear_tree();
        // Move D inline after A (between A and B)
        tr.rebase(NodeId(3), Placement::InlineAfter(NodeId(0)));
        assert_eq!(tr.parent[&NodeId(3)], NodeId(0)); // D is child of A
        assert_eq!(tr.parent[&NodeId(1)], NodeId(3)); // B is now child of D
    }

    #[test]
    fn rebase_inline_after_preserves_all_children() {
        let mut tr = forked_tree();
        // Move E inline after A (between A and [B, D])
        tr.rebase(NodeId(4), Placement::InlineAfter(NodeId(0)));
        // E should now have both B and D as children
        assert!(tr.children[&NodeId(4)].contains(&NodeId(1)));
        assert!(tr.children[&NodeId(4)].contains(&NodeId(3)));
    }

    // === Squash Tests ===

    #[test]
    fn squash_reparents_children() {
        let mut tr = linear_tree();
        // Squash C into B (D becomes child of B)
        tr.squash(NodeId(2), NodeId(1));
        assert_eq!(tr.parent[&NodeId(3)], NodeId(1));
    }

    #[test]
    fn squash_removes_source() {
        let mut tr = linear_tree();
        tr.squash(NodeId(2), NodeId(1));
        assert!(!tr.children.values().any(|c| c.contains(&NodeId(2))));
        assert!(!tr.parent.contains_key(&NodeId(2)));
    }

    // === DFS Traversal Tests ===

    #[test]
    fn dfs_order_linear() {
        let tr = linear_tree();
        let order: Vec<NodeId> = tr.dfs_iter().map(|(n, _)| n).collect();
        assert_eq!(order, vec![NodeId(0), NodeId(1), NodeId(2), NodeId(3)]);
    }

    #[test]
    fn dfs_depths_linear() {
        let tr = linear_tree();
        let depths: Vec<usize> = tr.dfs_iter().map(|(_, d)| d).collect();
        assert_eq!(depths, vec![0, 1, 2, 3]);
    }

    #[test]
    fn dfs_order_forked() {
        let tr = forked_tree();
        let order: Vec<NodeId> = tr.dfs_iter().map(|(n, _)| n).collect();
        // A, then B branch, then D branch
        assert_eq!(order, vec![NodeId(0), NodeId(1), NodeId(2), NodeId(3), NodeId(4)]);
    }

    #[test]
    fn dfs_depths_forked() {
        let tr = forked_tree();
        let depths: Vec<usize> = tr.dfs_iter().map(|(_, d)| d).collect();
        // A=0, B=1, C=2, D=1, E=2
        assert_eq!(depths, vec![0, 1, 2, 1, 2]);
    }

    // === Integration Tests ===

    #[test]
    fn linearize_forked_tree() {
        let mut tr = forked_tree();
        // Move D to be child of C (linearizes the fork)
        tr.rebase(NodeId(3), Placement::ChildOf(NodeId(2)));

        let order: Vec<NodeId> = tr.dfs_iter().map(|(n, _)| n).collect();
        assert_eq!(order, vec![NodeId(0), NodeId(1), NodeId(2), NodeId(3), NodeId(4)]);

        let depths: Vec<usize> = tr.dfs_iter().map(|(_, d)| d).collect();
        assert_eq!(depths, vec![0, 1, 2, 3, 4]); // Now linear!
    }

    #[test]
    fn create_fork_from_linear() {
        let mut tr = linear_tree();
        // Move D to be sibling of B (creates fork at A)
        tr.rebase(NodeId(3), Placement::SiblingAfter(NodeId(1)));

        // A should now have two children
        assert_eq!(tr.children[&NodeId(0)].len(), 2);
    }

    #[test]
    fn rebase_with_descendants() {
        let mut tr = linear_tree();
        // Move B (with C, D) to be sibling after A (becomes root)
        tr.rebase_with_descendants(NodeId(1), Placement::SiblingAfter(NodeId(0)));

        // B is now a root
        assert!(tr.roots.contains(&NodeId(1)));
        // C is still child of B
        assert_eq!(tr.parent[&NodeId(2)], NodeId(1));
        // D is still child of C
        assert_eq!(tr.parent[&NodeId(3)], NodeId(2));
    }
}
```

### Property-Based Test Ideas

```rust
#[test]
fn rebase_preserves_node_count() {
    // After any rebase, total nodes should be unchanged
}

#[test]
fn rebase_preserves_all_nodes() {
    // Every node in original should appear in DFS of result
}

#[test]
fn squash_reduces_node_count_by_one() {
    // After squash, total nodes = original - 1
}

#[test]
fn dfs_visits_all_nodes_exactly_once() {
    // DFS should yield each node exactly once
}
```

---

## Migration Strategy

### Week 1: Parallel Implementation
- Create `preview.rs` with new data structures
- Add comprehensive unit tests
- Keep existing `build_rebase_preview()` working

### Week 2: Integration
- Create adapter: `Preview` → `Vec<PreviewEntry>` (old format)
- Test both paths produce identical output
- Switch rebase mode to new path behind feature flag

### Week 3: Cleanup
- Remove old `PreviewEntry` and `build_rebase_preview()`
- Unify rendering functions
- Add squash preview
- Update documentation

### Optional Feature Flag

```rust
const USE_NEW_PREVIEW: bool = true;

fn render_tree(frame: &mut Frame, app: &App, area: Rect) {
    // ...
    if let (Mode::Rebasing, Some(ref state)) = (&app.mode, &app.rebase_state) {
        if USE_NEW_PREVIEW {
            let preview = PreviewBuilder::new(&app.tree)
                .rebase_preview(&state.source_rev, dest_cursor, state.rebase_type);
            render_with_preview(frame, app, area, &preview);
        } else {
            let preview = build_rebase_preview(app, dest_cursor, &state.source_rev, state.rebase_type);
            render_tree_with_preview(frame, app, area, viewport_height, scroll_offset, &preview);
        }
    }
}
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `preview.rs` | **NEW** - Core data structures, operations, tests |
| `ui.rs` | Replace `build_rebase_preview()`, unify rendering |
| `app.rs` | Remove `compute_moving_indices()`, update types |
| `tree.rs` | Add `TreeRelations::from_tree_state()` |
| `mod.rs` | Add `mod preview;` |
| `CONTEXT.md` | Update preview documentation |

---

## Success Criteria

1. All unit tests pass
2. Visual output matches current behavior (manual verification once)
3. Code is simpler: fewer lines, single rendering path
4. New operations (squash preview, `-A`/`-B`) work correctly
5. No regressions in existing functionality
