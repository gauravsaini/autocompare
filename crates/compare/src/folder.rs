//! Folder-tree comparison: an iterative, symlink-safe walk.

#![forbid(unsafe_code)]
//!
//! Documented choices for rare situations:
//!
//! * The two roots must exist and be directories (checked with
//!   `std::fs::metadata`, so a symlink given directly as a root is
//!   resolved); anything else is [`Error::Missing`]. Root-level
//!   metadata failures are returned as errors, unlike child-level
//!   failures (below).
//! * The walk itself uses `std::fs::symlink_metadata` exclusively:
//!   symlinks are never followed, never treated as directories and
//!   therefore never walked, so symlink cycles cannot loop the walk.
//!   With `content_compare`, a node that is a symlink on either side
//!   is never opened for a byte compare; it falls back to the
//!   size/mtime rules so the "never follow" promise holds in every
//!   code path.
//! * If a child's metadata fails (typically a race where the entry
//!   vanished between `read_dir` and the metadata call), the walk
//!   does not fail: the child is still listed with a fallback
//!   [`NodeInfo`](crate::NodeInfo) (size 0, `UNIX_EPOCH`) and
//!   `is_dir: false`, and the normal classification rules run on
//!   those defaults. A dir-vs-file pair reports `is_dir` from the
//!   left side.
//! * One-sided directories are still walked; their children are all
//!   one-sided too. Depth limiting is honest: when a directory at
//!   the cut depth exists, the walk does not descend, but it does
//!   check (without walking) whether it has any child and marks the
//!   result truncated only then.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use autocompare_vfs::LocalFs;

use crate::binary::files_equal_cancellable;
use crate::{Error, FolderComparison, FolderEntry, FolderOptions, FolderStatus, NodeInfo, Result};

/// Folder comparison engine over physical directories or [`LocalFs`].
#[derive(Debug, Clone)]
pub struct FolderCompare {
    left_root: PathBuf,
    right_root: PathBuf,
    options: FolderOptions,
}

impl FolderCompare {
    /// Creates a new `FolderCompare` between two directory paths, validating that they exist and are directories.
    pub fn new(
        left: impl AsRef<Path>,
        right: impl AsRef<Path>,
        options: FolderOptions,
    ) -> Result<Self> {
        let left_path = left.as_ref();
        let right_path = right.as_ref();
        require_dir(left_path)?;
        require_dir(right_path)?;
        Ok(Self {
            left_root: left_path.to_path_buf(),
            right_root: right_path.to_path_buf(),
            options,
        })
    }

    /// Creates a `FolderCompare` from existing [`LocalFs`] instances.
    pub fn from_local_fs(left_fs: &LocalFs, right_fs: &LocalFs, options: FolderOptions) -> Self {
        Self {
            left_root: left_fs.root_path().to_path_buf(),
            right_root: right_fs.root_path().to_path_buf(),
            options,
        }
    }

    /// Left root directory.
    pub fn left_root(&self) -> &Path {
        &self.left_root
    }

    /// Right root directory.
    pub fn right_root(&self) -> &Path {
        &self.right_root
    }

    /// Comparison options.
    pub fn options(&self) -> &FolderOptions {
        &self.options
    }

    /// Runs the comparison to completion.
    pub fn run(&self) -> Result<FolderComparison> {
        compare_folders_inner(&self.left_root, &self.right_root, &self.options, None)
    }

    /// Runs the comparison, checking the `cancel` flag periodically.
    pub fn run_cancellable(&self, cancel: &AtomicBool) -> Result<FolderComparison> {
        compare_folders_inner(
            &self.left_root,
            &self.right_root,
            &self.options,
            Some(cancel),
        )
    }

    /// Compares two modification times with a duration tolerance (such as FAT 2-second timestamp tolerance).
    pub fn compare_mtimes_with_tolerance(
        left: Option<SystemTime>,
        right: Option<SystemTime>,
        tolerance: std::time::Duration,
    ) -> FolderStatus {
        match (left, right) {
            (Some(lt), Some(rt)) => {
                if let Ok(duration) = lt.duration_since(rt) {
                    if duration > tolerance {
                        FolderStatus::NewerLeft
                    } else {
                        FolderStatus::Same
                    }
                } else if let Ok(duration) = rt.duration_since(lt) {
                    if duration > tolerance {
                        FolderStatus::NewerRight
                    } else {
                        FolderStatus::Same
                    }
                } else {
                    FolderStatus::Same
                }
            }
            _ => FolderStatus::Same,
        }
    }
}

/// A directory pair (or one-sided directory) pending the walk.
struct WalkNode {
    /// Path relative to the compared roots; empty for the roots.
    rel: String,
    /// The left directory, when it exists on this side.
    left: Option<PathBuf>,
    /// The right directory, when it exists on this side.
    right: Option<PathBuf>,
    depth: usize,
}

/// A named child of a compared directory.
struct Child {
    name: String,
    path: PathBuf,
}

/// Metadata summary for one side of a node.
struct SideInfo {
    is_dir: bool,
    is_symlink: bool,
    info: NodeInfo,
    /// The real mtime when the platform provides it.
    mtime: Option<SystemTime>,
}

/// Compares two folder trees. See the module documentation.
pub(crate) fn compare_folders(
    left: &Path,
    right: &Path,
    opts: &FolderOptions,
) -> Result<FolderComparison> {
    compare_folders_inner(left, right, opts, None)
}

/// Compares two folder trees with cancellation support.
pub fn compare_folders_cancellable(
    left: &Path,
    right: &Path,
    opts: &FolderOptions,
    cancel: &AtomicBool,
) -> Result<FolderComparison> {
    compare_folders_inner(left, right, opts, Some(cancel))
}

fn compare_folders_inner(
    left: &Path,
    right: &Path,
    opts: &FolderOptions,
    cancel: Option<&AtomicBool>,
) -> Result<FolderComparison> {
    require_dir(left)?;
    require_dir(right)?;

    let mut entries: Vec<FolderEntry> = Vec::new();
    let mut truncated = false;
    let mut stack: Vec<WalkNode> = vec![WalkNode {
        rel: String::new(),
        left: Some(left.to_path_buf()),
        right: Some(right.to_path_buf()),
        depth: 0,
    }];

    while let Some(node) = stack.pop() {
        if let Some(c) = cancel
            && c.load(Ordering::Relaxed)
        {
            return Err(Error::Cancelled);
        }

        let child_depth = match node.depth.checked_add(1) {
            Some(cd) if cd <= opts.depth_limit => cd,
            _ => {
                // Children would exceed the depth limit: do not walk
                // them, but record honestly whether any exist.
                if has_any_child(&node.left)? || has_any_child(&node.right)? {
                    truncated = true;
                }
                continue;
            }
        };

        let left_children = match &node.left {
            Some(dir) => Some(list_children(dir, opts)?),
            None => None,
        };
        let right_children = match &node.right {
            Some(dir) => Some(list_children(dir, opts)?),
            None => None,
        };

        match (left_children, right_children) {
            (Some(lc), Some(rc)) => {
                let (pairs, left_only, right_only) = pair_children(&lc, &rc, opts.case_insensitive);
                for (lc_child, rc_child) in pairs {
                    let rel = join_rel(&node.rel, &lc_child.name);
                    classify_pair(
                        lc_child,
                        rc_child,
                        rel,
                        child_depth,
                        opts,
                        &mut entries,
                        &mut stack,
                        cancel,
                    )?;
                }
                for child in left_only {
                    let rel = join_rel(&node.rel, &child.name);
                    one_sided(child, true, rel, child_depth, &mut entries, &mut stack);
                }
                for child in right_only {
                    let rel = join_rel(&node.rel, &child.name);
                    one_sided(child, false, rel, child_depth, &mut entries, &mut stack);
                }
            }
            (Some(lc), None) => {
                for child in &lc {
                    let rel = join_rel(&node.rel, &child.name);
                    one_sided(child, true, rel, child_depth, &mut entries, &mut stack);
                }
            }
            (None, Some(rc)) => {
                for child in &rc {
                    let rel = join_rel(&node.rel, &child.name);
                    one_sided(child, false, rel, child_depth, &mut entries, &mut stack);
                }
            }
            (None, None) => {}
        }
    }

    entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(FolderComparison { entries, truncated })
}

/// Verifies that `path` exists and is a directory.
fn require_dir(path: &Path) -> Result<()> {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(Error::Missing {
            path: path.display().to_string(),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::Missing {
            path: path.display().to_string(),
        }),
        Err(e) => Err(Error::Io {
            path: path.display().to_string(),
            source: e,
        }),
    }
}

/// Lists the direct children of `dir`, applying the include/exclude
/// name filters (exclude prunes; include must match) and sorted by
/// name for determinism.
fn list_children(dir: &Path, opts: &FolderOptions) -> Result<Vec<Child>> {
    let reader = std::fs::read_dir(dir).map_err(|e| Error::Io {
        path: dir.display().to_string(),
        source: e,
    })?;
    let mut children = Vec::new();
    for entry in reader {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                return Err(Error::Io {
                    path: dir.display().to_string(),
                    source: e,
                });
            }
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(pattern) = opts.exclude.as_deref()
            && glob_match(&name, pattern)
        {
            continue;
        }
        if let Some(pattern) = opts.include.as_deref()
            && !glob_match(&name, pattern)
        {
            continue;
        }
        children.push(Child {
            name,
            path: entry.path(),
        });
    }
    children.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(children)
}

/// Pairs children of the two sides by name: an exact-name pass
/// first, then — when `case_insensitive` is set — a pass over the
/// leftovers matching lowercased names, first match wins. Returns the
/// pairs plus the unpaired children of each side.
fn pair_children<'a>(
    left: &'a [Child],
    right: &'a [Child],
    case_insensitive: bool,
) -> (Vec<(&'a Child, &'a Child)>, Vec<&'a Child>, Vec<&'a Child>) {
    let mut pairs: Vec<(&'a Child, &'a Child)> = Vec::new();
    let mut left_matched: BTreeSet<usize> = BTreeSet::new();

    // Right children grouped by exact name; names are unique per side
    // in practice, and on a rare lossy-decoding collision the first
    // (sorted) entry wins.
    let mut right_exact: BTreeMap<&str, VecDeque<&'a Child>> = BTreeMap::new();
    for child in right {
        right_exact
            .entry(child.name.as_str())
            .or_default()
            .push_back(child);
    }

    // Exact-name pass.
    for (i, lc) in left.iter().enumerate() {
        if let Some(rc) = right_exact
            .get_mut(lc.name.as_str())
            .and_then(VecDeque::pop_front)
        {
            pairs.push((lc, rc));
            left_matched.insert(i);
        }
    }

    // Right children that survived the exact pass, either indexed by
    // lowercased name or straight to the unmatched list.
    let mut right_ci: BTreeMap<String, VecDeque<&'a Child>> = BTreeMap::new();
    let mut right_only: Vec<&'a Child> = Vec::new();
    for (_, queue) in right_exact {
        for rc in queue {
            if case_insensitive {
                right_ci
                    .entry(rc.name.to_lowercase())
                    .or_default()
                    .push_back(rc);
            } else {
                right_only.push(rc);
            }
        }
    }

    // Case-insensitive pass over the leftovers.
    if case_insensitive {
        for (i, lc) in left.iter().enumerate() {
            if left_matched.contains(&i) {
                continue;
            }
            let key = lc.name.to_lowercase();
            if let Some(rc) = right_ci.get_mut(&key).and_then(VecDeque::pop_front) {
                pairs.push((lc, rc));
                left_matched.insert(i);
            }
        }
        for (_, queue) in right_ci {
            right_only.extend(queue);
        }
    }

    let mut left_only: Vec<&'a Child> = Vec::new();
    for (i, lc) in left.iter().enumerate() {
        if !left_matched.contains(&i) {
            left_only.push(lc);
        }
    }

    (pairs, left_only, right_only)
}

/// Classifies a paired node: both directories walk children, both
/// non-directories follow the file rules (size, then content or
/// mtime), mixed kinds are `Different`.
#[allow(clippy::too_many_arguments)]
fn classify_pair(
    lc: &Child,
    rc: &Child,
    rel: String,
    child_depth: usize,
    opts: &FolderOptions,
    entries: &mut Vec<FolderEntry>,
    stack: &mut Vec<WalkNode>,
    cancel: Option<&AtomicBool>,
) -> Result<()> {
    let lside = probe_side(&lc.path);
    let rside = probe_side(&rc.path);
    let mut walk: Option<WalkNode> = None;
    let status;

    if lside.is_dir && rside.is_dir {
        status = newer_status(lside.mtime, rside.mtime);
        walk = Some(WalkNode {
            rel: rel.clone(),
            left: Some(lc.path.clone()),
            right: Some(rc.path.clone()),
            depth: child_depth,
        });
    } else if !lside.is_dir && !rside.is_dir {
        if lside.info.size != rside.info.size {
            status = FolderStatus::Different;
        } else if opts.content_compare && !lside.is_symlink && !rside.is_symlink {
            status = if files_equal_cancellable(&lc.path, &rc.path, cancel)? {
                FolderStatus::Same
            } else {
                FolderStatus::Different
            };
        } else {
            status = newer_status(lside.mtime, rside.mtime);
        }
    } else {
        // Mixed kinds (file vs directory, or a symlink on one side).
        status = FolderStatus::Different;
    }

    entries.push(FolderEntry {
        rel_path: rel,
        is_dir: lside.is_dir,
        left: Some(lside.info),
        right: Some(rside.info),
        status,
    });
    if let Some(node) = walk {
        stack.push(node);
    }
    Ok(())
}

/// Records a one-sided node and walks it when it is a directory.
fn one_sided(
    child: &Child,
    on_left: bool,
    rel: String,
    child_depth: usize,
    entries: &mut Vec<FolderEntry>,
    stack: &mut Vec<WalkNode>,
) {
    let side = probe_side(&child.path);
    let (left_info, right_info, status) = if on_left {
        (Some(side.info), None, FolderStatus::LeftOnly)
    } else {
        (None, Some(side.info), FolderStatus::RightOnly)
    };
    if side.is_dir {
        stack.push(WalkNode {
            rel: rel.clone(),
            left: if on_left {
                Some(child.path.clone())
            } else {
                None
            },
            right: if on_left {
                None
            } else {
                Some(child.path.clone())
            },
            depth: child_depth,
        });
    }
    entries.push(FolderEntry {
        rel_path: rel,
        is_dir: side.is_dir,
        left: left_info,
        right: right_info,
        status,
    });
}

/// Metadata for one side of a node. A metadata failure (rare race)
/// does not fail the walk: the fallback documented in the module
/// docs keeps the node listed and classifiable.
fn probe_side(path: &Path) -> SideInfo {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => SideInfo {
            is_dir: meta.is_dir(),
            is_symlink: meta.file_type().is_symlink(),
            info: node_info(&meta),
            mtime: meta.modified().ok(),
        },
        Err(_) => SideInfo {
            is_dir: false,
            is_symlink: false,
            info: NodeInfo {
                size: 0,
                modified: UNIX_EPOCH,
            },
            mtime: None,
        },
    }
}

/// Size and modified time for one side; size is 0 for directories
/// and metadata len() otherwise (for a symlink, the length of the
/// target path string). A missing mtime falls back to UNIX_EPOCH and
/// never fails the walk.
fn node_info(meta: &Metadata) -> NodeInfo {
    NodeInfo {
        size: if meta.is_dir() { 0 } else { meta.len() },
        modified: meta.modified().unwrap_or(UNIX_EPOCH),
    }
}

/// Strictly-newer comparison; equal times or missing information on
/// either side classify as `Same`.
fn newer_status(left: Option<SystemTime>, right: Option<SystemTime>) -> FolderStatus {
    match (left, right) {
        (Some(lt), Some(rt)) if lt > rt => FolderStatus::NewerLeft,
        (Some(lt), Some(rt)) if rt > lt => FolderStatus::NewerRight,
        _ => FolderStatus::Same,
    }
}

/// Whether the existing sides of a node have any child at all,
/// without walking deeper. Used at the depth cut.
fn has_any_child(dir: &Option<PathBuf>) -> Result<bool> {
    let Some(path) = dir else {
        return Ok(false);
    };
    let mut reader = std::fs::read_dir(path).map_err(|e| Error::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    match reader.next() {
        Some(Ok(_)) => Ok(true),
        Some(Err(e)) => Err(Error::Io {
            path: path.display().to_string(),
            source: e,
        }),
        None => Ok(false),
    }
}

/// Joins a child name onto a relative path with '/' separators.
fn join_rel(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

/// Simple case-sensitive glob: `*` matches any run of characters
/// (including none) and `?` matches exactly one character.
fn glob_match(name: &str, pattern: &str) -> bool {
    let n: Vec<char> = name.chars().collect();
    let p: Vec<char> = pattern.chars().collect();
    let mut ni: usize = 0;
    let mut pi: usize = 0;
    // Position after the last '*' and the name offset it currently
    // matches, for backtracking.
    let mut star: Option<(usize, usize)> = None;

    while ni < n.len() {
        match p.get(pi) {
            Some('?') => {
                ni += 1;
                pi += 1;
            }
            Some('*') => {
                star = Some((pi + 1, ni));
                pi += 1;
            }
            Some(&pc) => {
                if n.get(ni) == Some(&pc) {
                    ni += 1;
                    pi += 1;
                } else if let Some((sp, sn)) = star {
                    pi = sp;
                    ni = sn + 1;
                    star = Some((sp, sn + 1));
                } else {
                    return false;
                }
            }
            None => {
                if let Some((sp, sn)) = star {
                    pi = sp;
                    ni = sn + 1;
                    star = Some((sp, sn + 1));
                } else {
                    return false;
                }
            }
        }
    }

    while p.get(pi) == Some(&'*') {
        pi += 1;
    }
    pi == p.len()
}

/// A node in a folder tree view with depth and expansion metadata.
#[derive(Debug, Clone)]
pub struct FolderTreeNode<'a> {
    /// The underlying folder entry.
    pub entry: &'a FolderEntry,
    /// Tree depth (0 for root items).
    pub depth: usize,
    /// Whether this directory is expanded in the view.
    pub is_expanded: bool,
    /// Whether this directory has children in the comparison.
    pub has_children: bool,
}

impl<'a> PartialEq for FolderTreeNode<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.entry.rel_path == other.entry.rel_path
            && self.depth == other.depth
            && self.is_expanded == other.is_expanded
            && self.has_children == other.has_children
    }
}

impl<'a> Eq for FolderTreeNode<'a> {}

/// Aggregate counts of entries by their comparison status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FolderStatusCounts {
    /// Items present and matching on both sides.
    pub same: usize,
    /// Items present on both sides with differing content or metadata.
    pub different: usize,
    /// Items present on both sides where left is newer.
    pub newer_left: usize,
    /// Items present on both sides where right is newer.
    pub newer_right: usize,
    /// Items only on the left side.
    pub left_only: usize,
    /// Items only on the right side.
    pub right_only: usize,
}

impl FolderStatusCounts {
    /// Total entries summarized.
    pub fn total(&self) -> usize {
        self.same
            .saturating_add(self.different)
            .saturating_add(self.newer_left)
            .saturating_add(self.newer_right)
            .saturating_add(self.left_only)
            .saturating_add(self.right_only)
    }

    /// Number of entries with any difference (not Same).
    pub fn difference_count(&self) -> usize {
        self.different
            .saturating_add(self.newer_left)
            .saturating_add(self.newer_right)
            .saturating_add(self.left_only)
            .saturating_add(self.right_only)
    }
}

impl FolderComparison {
    /// Builds a hierarchical view of visible tree nodes given a set of expanded directory paths.
    ///
    /// An entry is visible if all its ancestor directories are contained in `expanded_dirs`.
    /// Root entries (depth 0) are always visible.
    pub fn tree_nodes<'a>(&'a self, expanded_dirs: &BTreeSet<String>) -> Vec<FolderTreeNode<'a>> {
        let mut nodes = Vec::new();
        for entry in &self.entries {
            if Self::is_visible(&entry.rel_path, expanded_dirs) {
                let depth = Self::entry_depth(&entry.rel_path);
                let is_expanded = entry.is_dir && expanded_dirs.contains(&entry.rel_path);
                let has_children = entry.is_dir && self.has_descendants(&entry.rel_path);
                nodes.push(FolderTreeNode {
                    entry,
                    depth,
                    is_expanded,
                    has_children,
                });
            }
        }
        nodes
    }

    /// Returns a set containing all directory relative paths in this comparison.
    pub fn expand_all(&self) -> BTreeSet<String> {
        self.directory_paths()
    }

    /// Returns an empty set, collapsing all directories.
    pub fn collapse_all(&self) -> BTreeSet<String> {
        BTreeSet::new()
    }

    /// Toggles the expanded state of a directory path in `expanded_dirs`.
    pub fn toggle_expanded(expanded_dirs: &mut BTreeSet<String>, dir_rel: &str) {
        if expanded_dirs.contains(dir_rel) {
            expanded_dirs.remove(dir_rel);
        } else {
            expanded_dirs.insert(dir_rel.to_string());
        }
    }

    /// Checks if a directory is currently expanded.
    pub fn is_dir_expanded(expanded_dirs: &BTreeSet<String>, dir_rel: &str) -> bool {
        expanded_dirs.contains(dir_rel)
    }

    /// Collects all directory relative paths in the comparison into a set.
    pub fn directory_paths(&self) -> BTreeSet<String> {
        self.entries
            .iter()
            .filter(|e| e.is_dir)
            .map(|e| e.rel_path.clone())
            .collect()
    }

    /// Checks if an entry with `rel_path` is visible given `expanded_dirs`.
    pub fn is_visible(rel_path: &str, expanded_dirs: &BTreeSet<String>) -> bool {
        for ancestor in Self::ancestors(rel_path) {
            if !expanded_dirs.contains(&ancestor) {
                return false;
            }
        }
        true
    }

    /// Computes the depth of an entry from its relative path (0 for root items).
    pub fn entry_depth(rel_path: &str) -> usize {
        rel_path.chars().filter(|&c| c == '/').count()
    }

    /// Returns the immediate parent directory of `rel_path`, or `None` if at the root.
    pub fn parent_path(rel_path: &str) -> Option<&str> {
        rel_path.rsplit_once('/').map(|(parent, _)| parent)
    }

    /// Returns all ancestor directory relative paths for `rel_path`, shallowest first.
    pub fn ancestors(rel_path: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = rel_path;
        while let Some((parent, _)) = current.rsplit_once('/') {
            result.push(parent.to_string());
            current = parent;
        }
        result.reverse();
        result
    }

    /// Checks whether `ancestor` is an ancestor directory of `rel_path`.
    pub fn is_ancestor_of(ancestor: &str, rel_path: &str) -> bool {
        if ancestor.is_empty() {
            return false;
        }
        if let Some(rest) = rel_path.strip_prefix(ancestor) {
            rest.starts_with('/')
        } else {
            false
        }
    }

    /// Returns whether any entry in the comparison is a descendant of `dir_rel`.
    pub fn has_descendants(&self, dir_rel: &str) -> bool {
        let prefix = format!("{dir_rel}/");
        self.entries.iter().any(|e| e.rel_path.starts_with(&prefix))
    }

    /// Returns all direct children of `dir_rel` (or root items if `dir_rel` is empty).
    pub fn direct_children<'a>(&'a self, dir_rel: &str) -> Vec<&'a FolderEntry> {
        if dir_rel.is_empty() {
            self.entries
                .iter()
                .filter(|e| Self::parent_path(&e.rel_path).is_none())
                .collect()
        } else {
            self.entries
                .iter()
                .filter(|e| Self::parent_path(&e.rel_path) == Some(dir_rel))
                .collect()
        }
    }

    /// Returns all descendant entries under `dir_rel`.
    pub fn all_descendants<'a>(&'a self, dir_rel: &str) -> Vec<&'a FolderEntry> {
        let prefix = format!("{dir_rel}/");
        self.entries
            .iter()
            .filter(|e| e.rel_path.starts_with(&prefix))
            .collect()
    }

    /// Summarizes the count of entries by their `FolderStatus`.
    pub fn status_counts(&self) -> FolderStatusCounts {
        let mut counts = FolderStatusCounts::default();
        for entry in &self.entries {
            match entry.status {
                FolderStatus::Same => counts.same = counts.same.saturating_add(1),
                FolderStatus::Different => counts.different = counts.different.saturating_add(1),
                FolderStatus::NewerLeft => counts.newer_left = counts.newer_left.saturating_add(1),
                FolderStatus::NewerRight => {
                    counts.newer_right = counts.newer_right.saturating_add(1);
                }
                FolderStatus::LeftOnly => counts.left_only = counts.left_only.saturating_add(1),
                FolderStatus::RightOnly => counts.right_only = counts.right_only.saturating_add(1),
            }
        }
        counts
    }

    /// Returns all entries that differ between left and right (status != `FolderStatus::Same`).
    pub fn differences(&self) -> Vec<&FolderEntry> {
        self.entries
            .iter()
            .filter(|e| e.status != FolderStatus::Same)
            .collect()
    }

    /// Returns all entries matching a specific `FolderStatus`.
    pub fn entries_by_status(&self, status: FolderStatus) -> Vec<&FolderEntry> {
        self.entries.iter().filter(|e| e.status == status).collect()
    }

    /// Finds an entry by its relative path.
    pub fn entry_by_path(&self, rel_path: &str) -> Option<&FolderEntry> {
        self.entries.iter().find(|e| e.rel_path == rel_path)
    }

    /// Whether the two folder trees are completely identical (no differences and not truncated).
    pub fn is_identical(&self) -> bool {
        !self.truncated && self.entries.iter().all(|e| e.status == FolderStatus::Same)
    }

    /// Total number of entries in the comparison.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_mtime_tolerance_comparison() {
        let now = SystemTime::now();
        let one_sec_later = now + Duration::from_secs(1);
        let five_secs_later = now + Duration::from_secs(5);

        // Within 2 second tolerance -> considered Same
        let res_tol = FolderCompare::compare_mtimes_with_tolerance(
            Some(one_sec_later),
            Some(now),
            Duration::from_secs(2),
        );
        assert_eq!(res_tol, FolderStatus::Same);

        // Beyond 2 second tolerance -> NewerLeft
        let res_diff = FolderCompare::compare_mtimes_with_tolerance(
            Some(five_secs_later),
            Some(now),
            Duration::from_secs(2),
        );
        assert_eq!(res_diff, FolderStatus::NewerLeft);
    }

    #[test]
    fn test_folder_comparison_helpers() {
        let entry_same = FolderEntry {
            rel_path: "same.txt".to_string(),
            is_dir: false,
            left: None,
            right: None,
            status: FolderStatus::Same,
        };
        let entry_diff = FolderEntry {
            rel_path: "diff.txt".to_string(),
            is_dir: false,
            left: None,
            right: None,
            status: FolderStatus::Different,
        };

        let comparison = FolderComparison {
            entries: vec![entry_same, entry_diff],
            truncated: false,
        };

        assert_eq!(comparison.entry_count(), 2);
        assert_eq!(comparison.differences().len(), 1);
        assert_eq!(comparison.differences()[0].rel_path, "diff.txt");
        assert_eq!(comparison.entries_by_status(FolderStatus::Same).len(), 1);
        assert!(!comparison.is_identical());
        assert!(comparison.entry_by_path("same.txt").is_some());
        assert!(comparison.entry_by_path("missing.txt").is_none());
    }
}
