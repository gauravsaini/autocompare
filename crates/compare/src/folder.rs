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

/// ODD Phase 4 Leaf B orchestration helpers (crate-internal).
///
/// Spans stay at command-to-algorithm depth: one entry span per public
/// compare plus rare fallback children, never per-item spans. Every
/// attribute is a redacted summary; file contents and raw paths never
/// enter telemetry.
pub(crate) fn odd_enabled() -> bool {
    autocompare_telemetry::Sampling::enabled()
}

/// Process-unique salt for path hashing: stable in-process so one run
/// correlates, never persisted anywhere.
pub(crate) fn odd_salt() -> String {
    format!("odd-compare-v1-{}", std::process::id())
}

/// Opaque token for a filesystem path. The raw path never leaves the process.
pub(crate) fn odd_path_token(path: &Path) -> String {
    autocompare_telemetry::redact::hash_path(&path.display().to_string(), &odd_salt())
}

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

    /// Runs the comparison over two virtual filesystems (LocalFs, Zip,
    /// snapshots) with this engine's options, ignoring the stored roots.
    pub fn run_over_vfs(
        &self,
        left: &dyn autocompare_vfs::Vfs,
        right: &dyn autocompare_vfs::Vfs,
    ) -> Result<FolderComparison> {
        crate::vfs_compare::compare_vfs_folders(left, right, &self.options)
    }

    /// Runs the VFS comparison, checking the `cancel` flag periodically.
    pub fn run_over_vfs_cancellable(
        &self,
        left: &dyn autocompare_vfs::Vfs,
        right: &dyn autocompare_vfs::Vfs,
        cancel: &AtomicBool,
    ) -> Result<FolderComparison> {
        crate::vfs_compare::compare_vfs_folders_cancellable(
            left,
            right,
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
    // ODD Phase 4 Leaf B: one orchestration span for the whole compare.
    // Attributes are redacted summaries only: hashed roots, option
    // flags, result counts. Never per-item spans, never contents.
    let odd_span = tracing::info_span!(
        "folder.compare",
        component = "rs.compare/folder",
        left = %odd_path_token(left),
        right = %odd_path_token(right),
        depth_limit = opts.depth_limit as u64,
        content_compare = opts.content_compare,
        case_insensitive = opts.case_insensitive,
        entries = tracing::field::Empty,
        truncated = tracing::field::Empty,
    );
    let _odd_guard = if odd_enabled() {
        Some(odd_span.enter())
    } else {
        None
    };
    require_dir(left)?;
    require_dir(right)?;

    let mut entries: Vec<FolderEntry> = Vec::new();
    let mut truncated = false;
    let masks = active_masks(opts);
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
                        &masks,
                        &mut entries,
                        &mut stack,
                        cancel,
                    )?;
                }
                for child in left_only {
                    let rel = join_rel(&node.rel, &child.name);
                    one_sided(child, true, rel, child_depth, opts, &masks, &mut entries, &mut stack);
                }
                for child in right_only {
                    let rel = join_rel(&node.rel, &child.name);
                    one_sided(child, false, rel, child_depth, opts, &masks, &mut entries, &mut stack);
                }
            }
            (Some(lc), None) => {
                for child in &lc {
                    let rel = join_rel(&node.rel, &child.name);
                    one_sided(child, true, rel, child_depth, opts, &masks, &mut entries, &mut stack);
                }
            }
            (None, Some(rc)) => {
                for child in &rc {
                    let rel = join_rel(&node.rel, &child.name);
                    one_sided(child, false, rel, child_depth, opts, &masks, &mut entries, &mut stack);
                }
            }
            (None, None) => {}
        }
    }

    entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    let comparison = FolderComparison { entries, truncated };
    odd_span.record("entries", comparison.entries.len() as u64);
    odd_span.record("truncated", comparison.truncated);
    Ok(comparison)
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
    masks: &FileMasks,
    entries: &mut Vec<FolderEntry>,
    stack: &mut Vec<WalkNode>,
    cancel: Option<&AtomicBool>,
) -> Result<()> {
    let lside = probe_side(&lc.path);
    let rside = probe_side(&rc.path);
    // File masks apply to the entry before any status verdict: a masked
    // file pair is dropped entirely. When either side is a directory the
    // walk still descends the directory side(s) so nested matches are
    // not lost; the masked pair itself is never listed.
    if !lside.is_dir || !rside.is_dir {
        let name = file_name_of(&rel);
        if !masks_allow(masks, name, &rel, opts.case_insensitive) {
            if lside.is_dir {
                stack.push(WalkNode {
                    rel: rel.clone(),
                    left: Some(lc.path.clone()),
                    right: None,
                    depth: child_depth,
                });
            }
            if rside.is_dir {
                stack.push(WalkNode {
                    rel,
                    left: None,
                    right: Some(rc.path.clone()),
                    depth: child_depth,
                });
            }
            return Ok(());
        }
    }
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
/// File masks apply before the entry is recorded: masked files are
/// dropped, while directories always pass so their matching children
/// still surface.
#[allow(clippy::too_many_arguments)]
fn one_sided(
    child: &Child,
    on_left: bool,
    rel: String,
    child_depth: usize,
    opts: &FolderOptions,
    masks: &FileMasks,
    entries: &mut Vec<FolderEntry>,
    stack: &mut Vec<WalkNode>,
) {
    let side = probe_side(&child.path);
    if !side.is_dir {
        let name = file_name_of(&rel);
        if !masks_allow(masks, name, &rel, opts.case_insensitive) {
            return;
        }
    }
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
pub(crate) fn glob_match(name: &str, pattern: &str) -> bool {
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

/// BC-Rules style file masks: a semicolon-separated list of simple
/// globs (`*` and `?`). Items prefixed with `-` are excludes, the rest
/// are includes. Each pattern is matched against the file name and the
/// '/'-separated relative path, so `*.ts` filters by name while
/// `src/*.ts` filters by path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileMasks {
    /// Patterns a file must match at least one of (empty = no constraint).
    pub includes: Vec<String>,
    /// Patterns that hide a file when matched (win over includes).
    pub excludes: Vec<String>,
}

impl FileMasks {
    /// Whether any include or exclude pattern is present.
    pub fn is_active(&self) -> bool {
        !self.includes.is_empty() || !self.excludes.is_empty()
    }
}

/// Parses a `masks` string into includes and excludes. `None`, empty,
/// or whitespace-only input yields an inactive [`FileMasks`]. Items are
/// split on `;`, trimmed, and empty items are ignored; a `-` prefix
/// (after trimming) marks an exclude. A lone `-` is ignored.
pub fn parse_file_masks(masks: Option<&str>) -> FileMasks {
    let mut out = FileMasks::default();
    let Some(raw) = masks else {
        return out;
    };
    for item in raw.split(';') {
        let item = item.trim();
        if item.is_empty() || item == "-" {
            continue;
        }
        if let Some(pattern) = item.strip_prefix('-') {
            let pattern = pattern.trim();
            if !pattern.is_empty() {
                out.excludes.push(pattern.to_owned());
            }
        } else {
            out.includes.push(item.to_owned());
        }
    }
    out
}

/// Whether a file with this name and relative path passes `masks`.
/// Excludes win: any exclude match hides the file even when an include
/// also matches. When includes are present, at least one must match.
/// Matching is [`glob_match`] against the file name and the full
/// relative path, folded to lowercase when `case_insensitive` is set.
pub fn masks_allow(
    masks: &FileMasks,
    file_name: &str,
    rel_path: &str,
    case_insensitive: bool,
) -> bool {
    if !masks.is_active() {
        return true;
    }
    if masks
        .excludes
        .iter()
        .any(|p| mask_pattern_matches(p, file_name, rel_path, case_insensitive))
    {
        return false;
    }
    if masks.includes.is_empty() {
        return true;
    }
    masks
        .includes
        .iter()
        .any(|p| mask_pattern_matches(p, file_name, rel_path, case_insensitive))
}

/// Matches one mask pattern against the file name and the relative
/// path (either match keeps the file).
fn mask_pattern_matches(
    pattern: &str,
    file_name: &str,
    rel_path: &str,
    case_insensitive: bool,
) -> bool {
    if case_insensitive {
        let pattern = pattern.to_lowercase();
        let file_name = file_name.to_lowercase();
        let rel_path = rel_path.to_lowercase();
        glob_match(&file_name, &pattern) || glob_match(&rel_path, &pattern)
    } else {
        glob_match(file_name, pattern) || glob_match(rel_path, pattern)
    }
}

/// File name part of a '/'-separated relative path.
fn file_name_of(rel_path: &str) -> &str {
    rel_path.rsplit('/').next().unwrap_or(rel_path)
}

/// Masks from `opts`, ready for per-entry checks.
fn active_masks(opts: &FolderOptions) -> FileMasks {
    parse_file_masks(opts.masks.as_deref())
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
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
    fn test_parse_file_masks_splits_includes_and_excludes() {
        let masks = parse_file_masks(Some("*.ts; *.tsx ; -*.min.js;- bundle.js"));
        assert_eq!(masks.includes, vec!["*.ts".to_string(), "*.tsx".to_string()]);
        assert_eq!(
            masks.excludes,
            vec!["*.min.js".to_string(), "bundle.js".to_string()]
        );
        assert!(masks.is_active());

        assert!(!parse_file_masks(None).is_active());
        assert!(!parse_file_masks(Some("")).is_active());
        assert!(!parse_file_masks(Some("  ; - ; ")).is_active());
    }

    #[test]
    fn test_masks_allow_include_only() {
        let masks = parse_file_masks(Some("*.ts;*.tsx"));
        assert!(masks_allow(&masks, "a.ts", "a.ts", false));
        assert!(masks_allow(&masks, "b.tsx", "sub/b.tsx", false));
        assert!(!masks_allow(&masks, "c.js", "c.js", false));
    }

    #[test]
    fn test_masks_allow_exclude_only() {
        let masks = parse_file_masks(Some("-*.min.js"));
        assert!(!masks_allow(&masks, "app.min.js", "app.min.js", false));
        assert!(masks_allow(&masks, "app.js", "app.js", false));
    }

    #[test]
    fn test_masks_allow_mixed_exclude_wins() {
        let masks = parse_file_masks(Some("*.ts;-*.test.ts"));
        assert!(masks_allow(&masks, "main.ts", "main.ts", false));
        assert!(!masks_allow(&masks, "main.test.ts", "main.test.ts", false));
        assert!(!masks_allow(&masks, "util.js", "util.js", false));
    }

    #[test]
    fn test_masks_allow_matches_relative_path() {
        let masks = parse_file_masks(Some("src/*.ts"));
        assert!(masks_allow(&masks, "a.ts", "src/a.ts", false));
        assert!(!masks_allow(&masks, "a.ts", "lib/a.ts", false));
    }

    #[test]
    fn test_masks_allow_question_wildcard_and_case_option() {
        let masks = parse_file_masks(Some("file?.ts"));
        assert!(masks_allow(&masks, "file1.ts", "file1.ts", false));
        assert!(!masks_allow(&masks, "file12.ts", "file12.ts", false));

        let masks = parse_file_masks(Some("*.TS"));
        assert!(!masks_allow(&masks, "a.ts", "a.ts", false));
        assert!(masks_allow(&masks, "a.ts", "a.ts", true));
    }

    #[test]
    fn test_masks_allow_inactive_passes_everything() {
        let masks = parse_file_masks(None);
        assert!(masks_allow(&masks, "anything.bin", "sub/anything.bin", false));
    }

    #[test]
    fn odd_redaction_never_leaks_paths_or_contents() {
        // Path tokens are opaque, stable in-process, and distinct per path.
        let token_a = odd_path_token(std::path::Path::new("/secret/home/user/tokens.txt"));
        let token_b = odd_path_token(std::path::Path::new("/secret/home/user/tokens.txt"));
        assert!(!token_a.is_empty());
        assert_eq!(token_a, token_b);
        assert!(!token_a.contains("secret"));
        assert!(!token_a.contains("tokens"));
        let other = odd_path_token(std::path::Path::new("/secret/home/user/other.txt"));
        assert_ne!(token_a, other);
        // Byte summaries carry shape only: hash prefix, size, line count.
        let summary = autocompare_telemetry::redact::summarize_bytes(b"super-secret-contents");
        assert_eq!(summary.bytes, 21);
        assert_eq!(summary.lines, 0);
        assert_eq!(summary.sha12.len(), 12);
        let view = format!("{summary:?}");
        assert!(!view.contains("super-secret"));
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
