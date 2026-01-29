use crate::jj_lib_helpers::JjRepo;
use eyre::Result;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct TreeNode {
    pub change_id: String,
    pub unique_prefix_len: usize,
    pub description: String,
    pub bookmarks: Vec<String>,
    pub is_working_copy: bool,
    pub parent_ids: Vec<String>,
    pub depth: usize,
}

impl TreeNode {
    #[allow(dead_code)]
    pub fn display_name(&self) -> String {
        if self.bookmarks.is_empty() {
            format!("({})", self.change_id)
        } else {
            self.bookmarks.join(" ")
        }
    }

    pub fn is_visible(&self, full_mode: bool) -> bool {
        full_mode || !self.bookmarks.is_empty() || self.is_working_copy
    }
}

pub struct TreeState {
    pub nodes: Vec<TreeNode>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub full_mode: bool,
    children_map: HashMap<String, Vec<String>>,
    visible_indices: Vec<usize>,
}

impl TreeState {
    pub fn load(jj_repo: &JjRepo) -> Result<Self> {
        let working_copy = jj_repo.working_copy_commit()?;
        let working_copy_id = jj_repo.shortest_change_id(&working_copy, 4)?;

        let commits = jj_repo.eval_revset("descendants(roots(trunk()..@))")?;

        let mut commit_map: HashMap<String, TreeNode> = HashMap::new();
        let mut children_map: HashMap<String, Vec<String>> = HashMap::new();

        for commit in &commits {
            let (change_id, unique_prefix_len) = jj_repo.change_id_with_prefix_len(commit, 4)?;
            let bookmarks = jj_repo.bookmarks_at(commit);
            let description = JjRepo::description_first_line(commit);

            let parents = jj_repo.parent_commits(commit)?;
            let parent_ids: Vec<String> = parents
                .iter()
                .filter_map(|p| jj_repo.shortest_change_id(p, 4).ok())
                .collect();

            let is_working_copy = change_id == working_copy_id;

            let node = TreeNode {
                change_id: change_id.clone(),
                unique_prefix_len,
                description,
                bookmarks,
                is_working_copy,
                parent_ids: parent_ids.clone(),
                depth: 0,
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
                full_mode: false,
                children_map: HashMap::new(),
                visible_indices: Vec::new(),
            });
        }

        let revs_in_set: HashSet<&str> = commit_map.keys().map(|s| s.as_str()).collect();
        let mut roots: Vec<String> = commit_map
            .values()
            .filter(|c| {
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

        let visible_indices = Self::compute_visible_indices(&nodes, false);

        Ok(Self {
            nodes,
            cursor: 0,
            scroll_offset: 0,
            full_mode: false,
            children_map,
            visible_indices,
        })
    }

    fn compute_visible_indices(nodes: &[TreeNode], full_mode: bool) -> Vec<usize> {
        nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_visible(full_mode))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn visible_nodes(&self) -> impl Iterator<Item = (usize, &TreeNode)> {
        self.visible_indices.iter().map(|&i| (i, &self.nodes[i]))
    }

    pub fn visible_count(&self) -> usize {
        self.visible_indices.len()
    }

    pub fn current_node(&self) -> Option<&TreeNode> {
        self.visible_indices
            .get(self.cursor)
            .and_then(|&i| self.nodes.get(i))
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
        for (i, &node_idx) in self.visible_indices.iter().enumerate() {
            if self.nodes[node_idx].is_working_copy {
                self.cursor = i;
                return;
            }
        }
    }

    pub fn toggle_full_mode(&mut self) {
        self.full_mode = !self.full_mode;
        self.visible_indices = Self::compute_visible_indices(&self.nodes, self.full_mode);

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
}
