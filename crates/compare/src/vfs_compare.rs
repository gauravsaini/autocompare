//! Folder-tree comparison over any [`Vfs`] pair (LocalFs, Zip, snapshots).
//!
//! Same verdicts as the path walk in [`crate::folder`]: missing on one
//! side is `LeftOnly`/`RightOnly`, both-dirs walk on, both-files follow
//! the size, then content or mtime rules, mixed kinds are `Different`.
//! Symlinks are never followed and never byte-compared; a child whose
//! stat fails keeps a lenient fallback entry instead of failing the
//! walk, while a failed directory listing is a hard error like the
//! path walk. Depth limiting is honest: a cut directory marks the
//! result truncated only when it really has a child.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use autocompare_vfs::{Vfs, VfsPath};

use crate::folder::glob_match;
use crate::{Error, FolderComparison, FolderEntry, FolderOptions, FolderStatus, NodeInfo, Result};

/// Streaming chunk size for VFS byte equality.
const CHUNK: usize = 64 * 1024;

/// Compares two virtual-filesystem trees from their roots.
pub fn compare_vfs_folders(
    left: &dyn Vfs,
    right: &dyn Vfs,
    opts: &FolderOptions,
) -> Result<FolderComparison> {
    compare_vfs_folders_cancellable(left, right, opts, None)
}

/// Compares two virtual-filesystem trees with cancellation support.
pub fn compare_vfs_folders_cancellable(
    left: &dyn Vfs,
    right: &dyn Vfs,
    opts: &FolderOptions,
    cancel: Option<&AtomicBool>,
) -> Result<FolderComparison> {
    require_vfs_dir(left, "left")?;
    require_vfs_dir(right, "right")?;

    let mut entries: Vec<FolderEntry> = Vec::new();
    let mut truncated = false;
    let mut stack: Vec<WalkNode> = Vec::new();
    stack.push(WalkNode {
        rel: VfsPath::root(),
        left: true,
        right: true,
        depth: 0,
    });

    while let Some(node) = stack.pop() {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }
        let child_depth = match node.depth.checked_add(1) {
            Some(cd) if cd <= opts.depth_limit => cd,
            _ => {
                if has_any_vfs_child(left, &node, true)? || has_any_vfs_child(right, &node, false)?
                {
                    truncated = true;
                }
                continue;
            }
        };

        let left_children = match node.left {
            true => Some(list_vfs_children(left, &node.rel, opts)?),
            false => None,
        };
        let right_children = match node.right {
            true => Some(list_vfs_children(right, &node.rel, opts)?),
            false => None,
        };

        match (left_children, right_children) {
            (Some(lc), Some(rc)) => {
                let (pairs, left_only, right_only) =
                    pair_vfs_children(&lc, &rc, opts.case_insensitive);
                for (lchild, rchild) in pairs {
                    let rel = join_child(&node.rel, &lchild.name);
                    classify_vfs_pair(
                        left,
                        right,
                        lchild,
                        rchild,
                        rel,
                        child_depth,
                        opts,
                        &mut entries,
                        &mut stack,
                        cancel,
                    )?;
                }
                for child in left_only {
                    let rel = join_child(&node.rel, &child.name);
                    one_sided_vfs(child, true, rel, child_depth, &mut entries, &mut stack);
                }
                for child in right_only {
                    let rel = join_child(&node.rel, &child.name);
                    one_sided_vfs(child, false, rel, child_depth, &mut entries, &mut stack);
                }
            }
            (Some(lc), None) => {
                for child in &lc {
                    let rel = join_child(&node.rel, &child.name);
                    one_sided_vfs(child, true, rel, child_depth, &mut entries, &mut stack);
                }
            }
            (None, Some(rc)) => {
                for child in &rc {
                    let rel = join_child(&node.rel, &child.name);
                    one_sided_vfs(child, false, rel, child_depth, &mut entries, &mut stack);
                }
            }
            (None, None) => {}
        }
    }

    entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(FolderComparison { entries, truncated })
}

/// A directory pair (or one-sided directory) pending the walk.
struct WalkNode {
    /// Virtual path of the directory; root for the compared roots.
    rel: VfsPath,
    /// Whether the directory exists on the left side.
    left: bool,
    /// Whether the directory exists on the right side.
    right: bool,
    depth: usize,
}

/// A named child seen through one VFS.
struct VfsChild {
    name: String,
    path: VfsPath,
    is_dir: bool,
    is_symlink: bool,
    info: NodeInfo,
    /// The real mtime when the VFS provides one.
    mtime: Option<SystemTime>,
}

/// Metadata summary reused from a listed child without a second stat.
fn child_summary(entry: &autocompare_vfs::VfsEntry) -> VfsChild {
    let meta = &entry.meta;
    VfsChild {
        name: entry.name.clone(),
        path: entry.path.clone(),
        is_dir: meta.is_dir,
        is_symlink: meta.is_symlink,
        info: NodeInfo {
            size: if meta.is_dir { 0 } else { meta.len },
            modified: meta.modified.unwrap_or(UNIX_EPOCH),
        },
        mtime: meta.modified,
    }
}

/// Verifies that a VFS root exists and is a directory.
fn require_vfs_dir(fs: &dyn Vfs, side: &str) -> Result<()> {
    match fs.stat(&VfsPath::root()) {
        Ok(meta) if meta.is_dir => Ok(()),
        Ok(_) => Err(Error::Missing {
            path: side.to_owned(),
        }),
        Err(e) => Err(map_vfs_error(e, side)),
    }
}

/// Maps a VFS error to a comparison error, keeping plain-English text.
fn map_vfs_error(err: autocompare_vfs::VfsError, path: &str) -> Error {
    use autocompare_vfs::VfsError as E;
    match err {
        E::NotFound(_) => Error::Missing {
            path: path.to_owned(),
        },
        E::NotADirectory(_) => Error::Missing {
            path: path.to_owned(),
        },
        E::Io(source) => Error::Io {
            path: path.to_owned(),
            source,
        },
        other => Error::Io {
            path: path.to_owned(),
            source: std::io::Error::other(other.to_string()),
        },
    }
}

/// Lists the direct children of `dir`, applying include/exclude name
/// filters and sorting by name for determinism.
fn list_vfs_children(fs: &dyn Vfs, dir: &VfsPath, opts: &FolderOptions) -> Result<Vec<VfsChild>> {
    let shown = if dir.is_root() {
        "/".to_owned()
    } else {
        dir.as_str().to_owned()
    };
    let listed = fs.list_dir(dir).map_err(|e| map_vfs_error(e, &shown))?;
    let mut children = Vec::new();
    for entry in &listed {
        if let Some(pattern) = opts.exclude.as_deref()
            && glob_match(&entry.name, pattern)
        {
            continue;
        }
        if let Some(pattern) = opts.include.as_deref()
            && !glob_match(&entry.name, pattern)
        {
            continue;
        }
        children.push(child_summary(entry));
    }
    children.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(children)
}

/// Pairs children of the two sides by name: an exact-name pass first,
/// then a lowercased pass when `case_insensitive` is set.
fn pair_vfs_children<'a>(
    left: &'a [VfsChild],
    right: &'a [VfsChild],
    case_insensitive: bool,
) -> (
    Vec<(&'a VfsChild, &'a VfsChild)>,
    Vec<&'a VfsChild>,
    Vec<&'a VfsChild>,
) {
    let mut pairs: Vec<(&'a VfsChild, &'a VfsChild)> = Vec::new();
    let mut left_matched: BTreeSet<usize> = BTreeSet::new();

    let mut right_exact: BTreeMap<&str, VecDeque<&'a VfsChild>> = BTreeMap::new();
    for child in right {
        right_exact
            .entry(child.name.as_str())
            .or_default()
            .push_back(child);
    }

    for (i, lc) in left.iter().enumerate() {
        if let Some(rc) = right_exact
            .get_mut(lc.name.as_str())
            .and_then(VecDeque::pop_front)
        {
            pairs.push((lc, rc));
            left_matched.insert(i);
        }
    }

    let mut right_ci: BTreeMap<String, VecDeque<&'a VfsChild>> = BTreeMap::new();
    let mut right_only: Vec<&'a VfsChild> = Vec::new();
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

    let mut left_only: Vec<&'a VfsChild> = Vec::new();
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
fn classify_vfs_pair(
    left: &dyn Vfs,
    right: &dyn Vfs,
    lc: &VfsChild,
    rc: &VfsChild,
    rel: VfsPath,
    child_depth: usize,
    opts: &FolderOptions,
    entries: &mut Vec<FolderEntry>,
    stack: &mut Vec<WalkNode>,
    cancel: Option<&AtomicBool>,
) -> Result<()> {
    let rel_path = rel.as_str().to_owned();
    let mut walk: Option<WalkNode> = None;
    let status;

    if lc.is_dir && rc.is_dir {
        status = newer_status(lc.mtime, rc.mtime);
        walk = Some(WalkNode {
            rel,
            left: true,
            right: true,
            depth: child_depth,
        });
    } else if !lc.is_dir && !rc.is_dir {
        if lc.info.size != rc.info.size {
            status = FolderStatus::Different;
        } else if opts.content_compare && !lc.is_symlink && !rc.is_symlink {
            status = if vfs_files_equal(left, &lc.path, right, &rc.path, cancel)? {
                FolderStatus::Same
            } else {
                FolderStatus::Different
            };
        } else {
            status = newer_status(lc.mtime, rc.mtime);
        }
    } else {
        status = FolderStatus::Different;
    }

    entries.push(FolderEntry {
        rel_path,
        is_dir: lc.is_dir,
        left: Some(lc.info),
        right: Some(rc.info),
        status,
    });
    if let Some(node) = walk {
        stack.push(node);
    }
    Ok(())
}

/// Records a one-sided node and walks it when it is a directory.
fn one_sided_vfs(
    child: &VfsChild,
    on_left: bool,
    rel: VfsPath,
    child_depth: usize,
    entries: &mut Vec<FolderEntry>,
    stack: &mut Vec<WalkNode>,
) {
    let rel_path = rel.as_str().to_owned();
    let (left_info, right_info, status) = if on_left {
        (Some(child.info), None, FolderStatus::LeftOnly)
    } else {
        (None, Some(child.info), FolderStatus::RightOnly)
    };
    if child.is_dir {
        stack.push(WalkNode {
            rel,
            left: on_left,
            right: !on_left,
            depth: child_depth,
        });
    }
    entries.push(FolderEntry {
        rel_path,
        is_dir: child.is_dir,
        left: left_info,
        right: right_info,
        status,
    });
}

/// Streams two VFS files in chunks and reports byte equality. A read
/// failure maps to [`Error::Io`]; symlinks never reach here.
fn vfs_files_equal(
    left: &dyn Vfs,
    left_path: &VfsPath,
    right: &dyn Vfs,
    right_path: &VfsPath,
    cancel: Option<&AtomicBool>,
) -> Result<bool> {
    let shown_l = left_path.as_str().to_owned();
    let shown_r = right_path.as_str().to_owned();
    let mut lreader = left
        .read_stream(left_path, u64::MAX)
        .map_err(|e| map_vfs_error(e, &shown_l))?;
    let mut rreader = right
        .read_stream(right_path, u64::MAX)
        .map_err(|e| map_vfs_error(e, &shown_r))?;
    let mut lbuf = vec![0u8; CHUNK];
    let mut rbuf = vec![0u8; CHUNK];
    loop {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }
        let llen = lreader.read(&mut lbuf).map_err(|e| Error::Io {
            path: shown_l.clone(),
            source: e,
        })?;
        let rlen = rreader.read(&mut rbuf).map_err(|e| Error::Io {
            path: shown_r.clone(),
            source: e,
        })?;
        if llen != rlen {
            return Ok(false);
        }
        if llen == 0 {
            return Ok(true);
        }
        if lbuf.get(..llen) != rbuf.get(..rlen) {
            return Ok(false);
        }
    }
}

/// Whether the existing sides of a node have any child at all,
/// without walking deeper. Used at the depth cut.
fn has_any_vfs_child(fs: &dyn Vfs, node: &WalkNode, is_left: bool) -> Result<bool> {
    if (is_left && !node.left) || (!is_left && !node.right) {
        return Ok(false);
    }
    let shown = if node.rel.is_root() {
        "/".to_owned()
    } else {
        node.rel.as_str().to_owned()
    };
    let listed = fs
        .list_dir(&node.rel)
        .map_err(|e| map_vfs_error(e, &shown))?;
    Ok(!listed.is_empty())
}

/// Joins a child name onto a virtual directory path.
fn join_child(parent: &VfsPath, name: &str) -> VfsPath {
    if parent.is_root() {
        VfsPath::new(name).unwrap_or_else(|_| VfsPath::root())
    } else {
        parent.join(name).unwrap_or_else(|_| VfsPath::root())
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
