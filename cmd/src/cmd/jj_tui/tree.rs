use crate::jj_lib_helpers::JjRepo;
use eyre::Result;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct BookmarkInfo {
    pub name: String,
    pub is_diverged: bool,
}

#[derive(Clone, Debug)]
pub struct TreeNode {
    pub change_id: String,
    pub unique_prefix_len: usize,
    pub description: String,
    pub bookmarks: Vec<BookmarkInfo>,
    pub is_working_copy: bool,
    pub parent_ids: Vec<String>,
    pub depth: usize,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: String,
}

impl TreeNode {
    #[allow(dead_code)]
    pub fn display_name(&self) -> String {
        if self.bookmarks.is_empty() {
            format!("({})", self.change_id)
        } else {
            self.bookmarks.iter().map(|b| b.name.as_str()).collect::<Vec<_>>().join(" ")
        }
    }

    pub fn is_visible(&self, full_mode: bool) -> bool {
        full_mode || !self.bookmarks.is_empty() || self.is_working_copy
    }

    /// Get bookmark names as strings (for compatibility)
    pub fn bookmark_names(&self) -> Vec<String> {
        self.bookmarks.iter().map(|b| b.name.clone()).collect()
    }

    /// Check if any bookmark has the given name
    pub fn has_bookmark(&self, name: &str) -> bool {
        self.bookmarks.iter().any(|b| b.name == name)
    }
}

pub struct VisibleEntry {
    pub node_index: usize,
    pub visual_depth: usize,
}

pub struct TreeState {
    pub nodes: Vec<TreeNode>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub full_mode: bool,
    pub expanded_entry: Option<usize>,
    children_map: HashMap<String, Vec<String>>,
    pub visible_entries: Vec<VisibleEntry>,
    pub selected: HashSet<usize>,
    pub selection_anchor: Option<usize>,
}

impl TreeState {
    pub fn load(jj_repo: &JjRepo) -> Result<Self> {
        let working_copy = jj_repo.working_copy_commit()?;
        let working_copy_id = jj_repo.shortest_change_id(&working_copy, 4)?;

        // same revset as CLI: base | descendants(roots(base..@)) | @::
        let commits = jj_repo.eval_revset("trunk() | descendants(roots(trunk()..@)) | @::")?;

        let mut commit_map: HashMap<String, TreeNode> = HashMap::new();
        let mut children_map: HashMap<String, Vec<String>> = HashMap::new();

        for commit in &commits {
            let (change_id, unique_prefix_len) = jj_repo.change_id_with_prefix_len(commit, 4)?;
            let bookmarks: Vec<BookmarkInfo> = jj_repo
                .bookmarks_with_state(commit)
                .into_iter()
                .map(|(name, is_diverged)| BookmarkInfo { name, is_diverged })
                .collect();
            let description = JjRepo::description_first_line(commit);

            let parents = jj_repo.parent_commits(commit)?;
            let parent_ids: Vec<String> = parents
                .iter()
                .filter_map(|p| jj_repo.shortest_change_id(p, 4).ok())
                .collect();

            let is_working_copy = change_id == working_copy_id;

            let author_name = JjRepo::author_name(commit);
            let author_email = JjRepo::author_email(commit);
            let timestamp = JjRepo::author_timestamp_relative(commit);

            let node = TreeNode {
                change_id: change_id.clone(),
                unique_prefix_len,
                description,
                bookmarks,
                is_working_copy,
                parent_ids: parent_ids.clone(),
                depth: 0,
                author_name,
                author_email,
                timestamp,
            };

            commit_map.insert(change_id.clone(), node);

            for parent_id in parent_ids {
                children_map
                    .entry(parent_id)
                    .or_default()
                    .push(change_id.clone());
            }
        }

        if commit_map.is_empty() {
            return Ok(Self {
                nodes: Vec::new(),
                cursor: 0,
                scroll_offset: 0,
                full_mode: true,
                expanded_entry: None,
                children_map: HashMap::new(),
                visible_entries: Vec::new(),
                selected: HashSet::new(),
                selection_anchor: None,
            });
        }

        // get trunk change_id for root detection
        let trunk_id = jj_repo
            .eval_revset_single("trunk()")
            .ok()
            .and_then(|c| jj_repo.shortest_change_id(&c, 4).ok());

        // find roots (commits whose parents aren't in our set, OR trunk itself)
        let revs_in_set: HashSet<&str> = commit_map.keys().map(|s| s.as_str()).collect();
        let mut roots: Vec<String> = commit_map
            .values()
            .filter(|c| {
                // always include trunk as root
                if let Some(ref tid) = trunk_id {
                    if c.change_id == *tid {
                        return true;
                    }
                }
                c.parent_ids
                    .iter()
                    .all(|p| !revs_in_set.contains(p.as_str()))
            })
            .map(|c| c.change_id.clone())
            .collect();
        roots.sort();

        let mut nodes = Vec::new();
        let mut visited = HashSet::new();

        fn traverse(
            change_id: &str,
            commit_map: &HashMap<String, TreeNode>,
            children_map: &HashMap<String, Vec<String>>,
            nodes: &mut Vec<TreeNode>,
            visited: &mut HashSet<String>,
            depth: usize,
        ) {
            if visited.contains(change_id) {
                return;
            }
            visited.insert(change_id.to_string());

            if let Some(node) = commit_map.get(change_id) {
                let mut node = node.clone();
                node.depth = depth;
                nodes.push(node);

                if let Some(children) = children_map.get(change_id) {
                    let mut sorted_children = children.clone();
                    sorted_children.sort();
                    for child in sorted_children {
                        traverse(&child, commit_map, children_map, nodes, visited, depth + 1);
                    }
                }
            }
        }

        for root in &roots {
            traverse(
                root,
                &commit_map,
                &children_map,
                &mut nodes,
                &mut visited,
                0,
            );
        }

        let visible_entries = Self::compute_visible_entries(&nodes, true);

        Ok(Self {
            nodes,
            cursor: 0,
            scroll_offset: 0,
            full_mode: true,
            expanded_entry: None,
            children_map,
            visible_entries,
            selected: HashSet::new(),
            selection_anchor: None,
        })
    }

    fn compute_visible_entries(nodes: &[TreeNode], full_mode: bool) -> Vec<VisibleEntry> {
        if full_mode {
            // in full mode, use structural depth
            nodes
                .iter()
                .enumerate()
                .map(|(i, n)| VisibleEntry {
                    node_index: i,
                    visual_depth: n.depth,
                })
                .collect()
        } else {
            // in non-full mode, compute visual depth based on visible ancestors
            let mut entries = Vec::new();
            let mut depth_stack: Vec<usize> = Vec::new(); // stack of structural depths

            for (i, node) in nodes.iter().enumerate() {
                if !node.is_visible(full_mode) {
                    continue;
                }

                // pop stack until we find an ancestor (node with smaller structural depth)
                while let Some(&parent_depth) = depth_stack.last() {
                    if parent_depth < node.depth {
                        break;
                    }
                    depth_stack.pop();
                }

                let visual_depth = depth_stack.len();
                depth_stack.push(node.depth);

                entries.push(VisibleEntry {
                    node_index: i,
                    visual_depth,
                });
            }
            entries
        }
    }

    pub fn visible_nodes(&self) -> impl Iterator<Item = &VisibleEntry> {
        self.visible_entries.iter()
    }

    pub fn get_node(&self, entry: &VisibleEntry) -> &TreeNode {
        &self.nodes[entry.node_index]
    }

    pub fn visible_count(&self) -> usize {
        self.visible_entries.len()
    }

    pub fn current_entry(&self) -> Option<&VisibleEntry> {
        self.visible_entries.get(self.cursor)
    }

    pub fn current_node(&self) -> Option<&TreeNode> {
        self.current_entry().map(|e| &self.nodes[e.node_index])
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.cursor + 1 < self.visible_count() {
            self.cursor += 1;
        }
    }

    pub fn move_cursor_top(&mut self) {
        self.cursor = 0;
        self.scroll_offset = 0;
    }

    pub fn move_cursor_bottom(&mut self) {
        let count = self.visible_count();
        if count > 0 {
            self.cursor = count - 1;
        }
    }

    pub fn jump_to_working_copy(&mut self) {
        for (i, entry) in self.visible_entries.iter().enumerate() {
            if self.nodes[entry.node_index].is_working_copy {
                self.cursor = i;
                return;
            }
        }
    }

    pub fn toggle_full_mode(&mut self) {
        self.full_mode = !self.full_mode;
        self.visible_entries = Self::compute_visible_entries(&self.nodes, self.full_mode);

        if self.cursor >= self.visible_count() {
            self.cursor = self.visible_count().saturating_sub(1);
        }
    }

    pub fn update_scroll(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.cursor - viewport_height + 1;
        }
    }

    #[allow(dead_code)]
    pub fn has_visible_children(&self, change_id: &str) -> bool {
        self.children_map.get(change_id).is_some_and(|children| {
            children.iter().any(|c| {
                self.nodes
                    .iter()
                    .any(|n| n.change_id == *c && n.is_visible(self.full_mode))
            })
        })
    }

    pub fn page_up(&mut self, amount: usize) {
        self.cursor = self.cursor.saturating_sub(amount);
    }

    pub fn page_down(&mut self, amount: usize) {
        let max = self.visible_count().saturating_sub(1);
        self.cursor = (self.cursor + amount).min(max);
    }

    pub fn toggle_expanded(&mut self) {
        if self.expanded_entry == Some(self.cursor) {
            self.expanded_entry = None;
        } else {
            self.expanded_entry = Some(self.cursor);
        }
    }

    pub fn is_expanded(&self, visible_idx: usize) -> bool {
        self.expanded_entry == Some(visible_idx)
    }

    pub fn toggle_selected(&mut self, idx: usize) {
        if self.selected.contains(&idx) {
            self.selected.remove(&idx);
        } else {
            self.selected.insert(idx);
        }
    }

    pub fn select_range(&mut self, from: usize, to: usize) {
        let (start, end) = if from <= to { (from, to) } else { (to, from) };
        for i in start..=end {
            self.selected.insert(i);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.selection_anchor = None;
    }
}
