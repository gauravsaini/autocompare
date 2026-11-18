//! Folder-tree comparison: an iterative, symlink-safe walk.
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
use std::time::{SystemTime, UNIX_EPOCH};

use crate::binary::files_equal;
use crate::{Error, FolderComparison, FolderEntry, FolderOptions, FolderStatus, NodeInfo, Result};

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
fn classify_pair(
    lc: &Child,
    rc: &Child,
    rel: String,
    child_depth: usize,
    opts: &FolderOptions,
    entries: &mut Vec<FolderEntry>,
    stack: &mut Vec<WalkNode>,
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
            status = if files_equal(&lc.path, &rc.path)? {
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
