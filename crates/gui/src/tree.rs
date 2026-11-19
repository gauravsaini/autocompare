//! Hierarchical folder tree drill-down logic.
//!
//! Provides pure filtering and indentation mapping for folder rows.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::collections::BTreeSet;

use crate::viewmodel::FolderRow;

/// One node in the hierarchical folder tree view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderTreeNode {
    /// Underlying folder entry row.
    pub row: FolderRow,
    /// Tree depth (nesting level, starting at 0 for root entries).
    pub depth: usize,
    /// Whether this directory is currently expanded.
    pub is_expanded: bool,
    /// Whether this directory contains any sub-items.
    pub has_children: bool,
}

/// Builds a filtered, indented hierarchical view of folder rows based on expanded directories.
///
/// An entry is visible if all its ancestor directories are contained in `expanded_dirs`.
/// Root entries (depth 0) are always visible.
pub fn folder_tree(rows: &[FolderRow], expanded_dirs: &BTreeSet<String>) -> Vec<FolderTreeNode> {
    let mut dirs_with_children = BTreeSet::new();
    for r in rows {
        if let Some(pos) = r.rel_path.rfind('/')
            && let Some(parent) = r.rel_path.get(..pos)
        {
            dirs_with_children.insert(parent.to_string());
        }
    }

    let mut nodes = Vec::new();
    for row in rows {
        let depth = row.rel_path.chars().filter(|&c| c == '/').count();
        let mut visible = true;

        if depth > 0 {
            let mut current = row.rel_path.as_str();
            while let Some(pos) = current.rfind('/') {
                let Some(parent) = current.get(..pos) else {
                    break;
                };
                if !expanded_dirs.contains(parent) {
                    visible = false;
                    break;
                }
                current = parent;
            }
        }

        if visible {
            let is_expanded = row.is_dir && expanded_dirs.contains(&row.rel_path);
            let has_children = row.is_dir && dirs_with_children.contains(&row.rel_path);
            nodes.push(FolderTreeNode {
                row: row.clone(),
                depth,
                is_expanded,
                has_children,
            });
        }
    }

    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(path: &str, is_dir: bool) -> FolderRow {
        FolderRow {
            rel_path: path.to_string(),
            is_dir,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        }
    }

    #[test]
    fn collapsed_tree_shows_only_root_entries() {
        let rows = vec![
            make_row("src", true),
            make_row("src/lib.rs", false),
            make_row("README.md", false),
        ];
        let expanded = BTreeSet::new();
        let tree = folder_tree(&rows, &expanded);
        assert_eq!(tree.len(), 2);
        let Some(first) = tree.first() else {
            return;
        };
        assert_eq!(first.row.rel_path, "src");
        assert_eq!(first.depth, 0);
        assert!(first.has_children);
        assert!(!first.is_expanded);

        let Some(second) = tree.get(1) else {
            return;
        };
        assert_eq!(second.row.rel_path, "README.md");
        assert_eq!(second.depth, 0);
    }

    #[test]
    fn expanded_directory_reveals_children() {
        let rows = vec![
            make_row("src", true),
            make_row("src/lib.rs", false),
            make_row("src/sub", true),
            make_row("src/sub/mod.rs", false),
        ];
        let mut expanded = BTreeSet::new();
        expanded.insert("src".to_string());

        let tree = folder_tree(&rows, &expanded);
        assert_eq!(tree.len(), 3);
        let Some(sub) = tree.get(2) else {
            return;
        };
        assert_eq!(sub.row.rel_path, "src/sub");
        assert_eq!(sub.depth, 1);
        assert!(sub.has_children);
        assert!(!sub.is_expanded);

        // Now expand nested directory
        expanded.insert("src/sub".to_string());
        let tree_full = folder_tree(&rows, &expanded);
        assert_eq!(tree_full.len(), 4);
        let Some(last) = tree_full.last() else {
            return;
        };
        assert_eq!(last.row.rel_path, "src/sub/mod.rs");
        assert_eq!(last.depth, 2);
    }
}
