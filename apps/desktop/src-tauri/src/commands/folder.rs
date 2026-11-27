//! Folder-compare IPC commands (Wave 1a leaf).
//!
//! # Ownership
//!
//! This module is owned by Wave 1a. Wave 1R owns `commands/mod.rs` and the
//! `lib.rs` handler registration, so this file is intentionally **not**
//! referenced from anywhere yet; that is fine and the crate still builds.
//!
//! # Wiring (Wave 1R handoff)
//!
//! 1. Add to `apps/desktop/src-tauri/Cargo.toml`:
//!    `autocompare-compare = { path = "../../../crates/compare" }`.
//! 2. Create `commands/mod.rs` with `pub mod folder;` (and `pub mod text;`).
//! 3. Register each `#[tauri::command]` below in the `lib.rs`
//!    `generate_handler![...]` table.
//!
//! # Paged-fetch rule (locked)
//!
//! The full [`FolderComparison`](autocompare_compare::FolderComparison)
//! stays in Rust memory (module-level snapshot cache). The UI only ever
//! receives `(start, limit)` windows via [`folder_get_tree`], so whole
//! trees are never serialized over IPC. [`folder_cancel`] drops the
//! in-flight cancel flag and/or the cached snapshot. Every handler
//! returns errors as [`String`] and never panics.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

use autocompare_compare::{
    FolderComparison, FolderEntry, FolderOptions, FolderStatus, NodeInfo,
    compare_folders_cancellable,
};
use serde::{Deserialize, Serialize};

/// Maximum entries returned by one [`folder_get_tree`] call. Bounds every
/// IPC payload regardless of tree size.
const MAX_PAGE: u64 = 500;

/// Status filter values accepted by [`folder_get_tree`].
const FILTER_ALL: &str = "all";
const FILTER_CHANGED: &str = "changed";

/// Snapshot summary returned by [`folder_compare`]. Never carries entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderCompareDto {
    pub session: String,
    pub left_root: String,
    pub right_root: String,
    pub total: u64,
    pub truncated: bool,
}

/// One side's metadata. `modified_ms` is millis since the Unix epoch
/// (`None` when the platform reports no usable modification time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfoDto {
    pub size: u64,
    pub modified_ms: Option<u64>,
}

impl NodeInfoDto {
    fn from_info(info: &NodeInfo) -> Self {
        let modified_ms = info
            .modified
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_millis() as u64);
        Self {
            size: info.size,
            modified_ms,
        }
    }
}

/// Verdict for one tree row. Wire names are camelCase (`"leftOnly"`,
/// `"newerLeft"`, ...), matching `folder.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FolderStatusDto {
    Same,
    Different,
    NewerLeft,
    NewerRight,
    LeftOnly,
    RightOnly,
}

impl FolderStatusDto {
    fn from_status(status: FolderStatus) -> Self {
        match status {
            FolderStatus::Same => FolderStatusDto::Same,
            FolderStatus::Different => FolderStatusDto::Different,
            FolderStatus::NewerLeft => FolderStatusDto::NewerLeft,
            FolderStatus::NewerRight => FolderStatusDto::NewerRight,
            FolderStatus::LeftOnly => FolderStatusDto::LeftOnly,
            FolderStatus::RightOnly => FolderStatusDto::RightOnly,
        }
    }

    fn parse_filter(raw: &str) -> Result<Option<Self>, String> {
        match raw {
            FILTER_ALL | FILTER_CHANGED => Ok(None),
            "same" => Ok(Some(FolderStatusDto::Same)),
            "different" => Ok(Some(FolderStatusDto::Different)),
            "newerLeft" => Ok(Some(FolderStatusDto::NewerLeft)),
            "newerRight" => Ok(Some(FolderStatusDto::NewerRight)),
            "leftOnly" => Ok(Some(FolderStatusDto::LeftOnly)),
            "rightOnly" => Ok(Some(FolderStatusDto::RightOnly)),
            _ => Err(format!(
                "unknown folder filter '{raw}'; expected \"all\", \"changed\", \
                 \"same\", \"different\", \"newerLeft\", \"newerRight\", \
                 \"leftOnly\" or \"rightOnly\""
            )),
        }
    }
}

/// One row of a folder comparison for the UI tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderEntryDto {
    pub rel_path: String,
    pub is_dir: bool,
    pub left: Option<NodeInfoDto>,
    pub right: Option<NodeInfoDto>,
    pub status: FolderStatusDto,
}

impl FolderEntryDto {
    fn from_entry(entry: &FolderEntry) -> Self {
        Self {
            rel_path: entry.rel_path.clone(),
            is_dir: entry.is_dir,
            left: entry.left.as_ref().map(NodeInfoDto::from_info),
            right: entry.right.as_ref().map(NodeInfoDto::from_info),
            status: FolderStatusDto::from_status(entry.status),
        }
    }
}

/// One `(start, limit)` window over the snapshot entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreePage {
    pub session: String,
    pub start: u64,
    pub total: u64,
    pub filter: String,
    pub entries: Vec<FolderEntryDto>,
}

/// Receipt returned by [`folder_cancel`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderCancelDto {
    pub session: String,
    pub cancelled: bool,
}

/// Receipt returned by [`folder_copy_entry`] and [`folder_delete_entry`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderActionReceipt {
    pub session: String,
    pub rel_path: String,
    pub action: String,
    pub bytes_affected: u64,
}

/// Cached full folder comparison. Lives in Rust memory only; the UI pages
/// through it via [`folder_get_tree`].
#[derive(Debug, Clone)]
struct StoredFolder {
    left_root: PathBuf,
    right_root: PathBuf,
    comparison: FolderComparison,
}

fn folder_store() -> &'static Mutex<HashMap<String, StoredFolder>> {
    static STORE: OnceLock<Mutex<HashMap<String, StoredFolder>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Cancel flags for currently running [`folder_compare`] calls, keyed by
/// the session id announced before the walk starts so a concurrent
/// [`folder_cancel`] invoke can interrupt it.
fn inflight_store() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static FLAGS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    FLAGS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_session_id() -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed).saturating_add(1);
    format!("folder-{n}")
}

fn lock_snapshots() -> Result<std::sync::MutexGuard<'static, HashMap<String, StoredFolder>>, String>
{
    folder_store()
        .lock()
        .map_err(|_| "folder snapshot cache is unavailable".to_string())
}

fn lock_inflight()
-> Result<std::sync::MutexGuard<'static, HashMap<String, Arc<AtomicBool>>>, String> {
    inflight_store()
        .lock()
        .map_err(|_| "folder job registry is unavailable".to_string())
}

fn snapshot(session: &str) -> Result<StoredFolder, String> {
    let guard = lock_snapshots()?;
    guard
        .get(session)
        .cloned()
        .ok_or_else(|| format!("unknown folder session '{session}'"))
}

/// Clamps `(start, limit)` to `[0, total]`, capping `limit` at
/// [`MAX_PAGE`]. Out-of-range `start` yields an empty window, never an
/// error. Overflow-safe: `start + limit` saturates.
pub fn clamp_window(total: u64, start: u64, limit: u64) -> (u64, u64) {
    let limit = limit.min(MAX_PAGE);
    if start >= total || limit == 0 {
        return (total.min(start), 0);
    }
    let end = start.saturating_add(limit).min(total);
    (start, end.saturating_sub(start))
}

/// Whether an entry passes `filter` (`"all"` passes everything,
/// `"changed"` passes every non-`Same` status, otherwise the status must
/// equal the named one). Unknown filters are errors, never silent `None`s.
pub fn entry_matches(status: FolderStatusDto, filter: &str) -> Result<bool, String> {
    if filter == FILTER_ALL {
        return Ok(true);
    }
    if filter == FILTER_CHANGED {
        return Ok(status != FolderStatusDto::Same);
    }
    let wanted = FolderStatusDto::parse_filter(filter)?;
    Ok(wanted.is_some_and(|w| w == status))
}

/// Compares two folder trees and caches the full result under a new
/// snapshot id. Only the summary travels back over IPC.
///
/// `masks` is an optional BC-Rules style file-mask list (semicolon
/// separated, `*`/`?` wildcards, `-` prefix excludes) matched against
/// file names and relative paths. `None` (or empty) disables mask
/// filtering; masks apply at compare time, before any `filter` used
/// by [`folder_get_tree`].
///
/// `contentCompare` / `content_compare` (either spelling wins) maps to
/// engine [`FolderOptions::content_compare`]: byte-compare files whose
/// sizes match (symlink-aware, never follows links). Omitted (or both
/// `false`) keeps the legacy mtime/size fast path. `quickCompare` /
/// `quick_compare` is the inverse alias (`true` = fast path); an
/// explicit `contentCompare` wins over `quickCompare`.
///
/// The walk runs through [`compare_folders_cancellable`] against a flag
/// registered up front, so a concurrent [`folder_cancel`] for the returned
/// session id can interrupt it.
#[tauri::command]
#[allow(non_snake_case)]
pub fn folder_compare(
    left: String,
    right: String,
    masks: Option<String>,
    contentCompare: Option<bool>,
    content_compare: Option<bool>,
    quickCompare: Option<bool>,
    quick_compare: Option<bool>,
) -> Result<FolderCompareDto, String> {
    if left.is_empty() {
        return Err("left path must not be empty".to_string());
    }
    if right.is_empty() {
        return Err("right path must not be empty".to_string());
    }
    // Empty/whitespace-only masks mean "no filtering", matching the
    // engine's backward-compatible default.
    let masks = masks.and_then(|m| {
        if m.trim().is_empty() {
            None
        } else {
            Some(m)
        }
    });
    let session = next_session_id();
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut inflight = lock_inflight()?;
        inflight.insert(session.clone(), Arc::clone(&cancel));
    }
    let options = FolderOptions {
        masks,
        content_compare: contentCompare
            .or(content_compare)
            .or_else(|| quickCompare.or(quick_compare).map(|q| !q))
            .unwrap_or(false),
        ..FolderOptions::default()
    };
    let outcome = compare_folders_cancellable(
        std::path::Path::new(&left),
        std::path::Path::new(&right),
        &options,
        &cancel,
    );
    {
        let mut inflight = lock_inflight()?;
        inflight.remove(&session);
    }
    let comparison = outcome.map_err(|e| e.to_string())?;
    let dto = FolderCompareDto {
        session: session.clone(),
        left_root: left.clone(),
        right_root: right.clone(),
        total: comparison.entries.len() as u64,
        truncated: comparison.truncated,
    };
    {
        let mut guard = lock_snapshots()?;
        guard.insert(
            session,
            StoredFolder {
                left_root: PathBuf::from(&left),
                right_root: PathBuf::from(&right),
                comparison,
            },
        );
    }
    Ok(dto)
}

/// Returns one `(start, limit)` window of snapshot entries, optionally
/// narrowed by `filter` (`"all"`, `"changed"`, or a status name).
#[tauri::command]
pub fn folder_get_tree(
    session: String,
    start: u64,
    limit: u64,
    filter: String,
) -> Result<TreePage, String> {
    // Validate the filter before touching the snapshot so typos surface
    // even on empty trees.
    FolderStatusDto::parse_filter(&filter)?;
    let cached = snapshot(&session)?;
    let mut matching: Vec<&FolderEntry> = Vec::new();
    for entry in &cached.comparison.entries {
        let status = FolderStatusDto::from_status(entry.status);
        let mut keep = false;
        if let Ok(passes) = entry_matches(status, &filter) {
            keep = passes;
        }
        if keep {
            matching.push(entry);
        }
    }
    let total = matching.len() as u64;
    let (from, len) = clamp_window(total, start, limit);
    let from_usize = usize::try_from(from).unwrap_or(usize::MAX);
    let len_usize = usize::try_from(len).unwrap_or(0);
    let entries: Vec<FolderEntryDto> = matching
        .iter()
        .skip(from_usize)
        .take(len_usize)
        .map(|e| FolderEntryDto::from_entry(e))
        .collect();
    Ok(TreePage {
        session,
        start: from,
        total,
        filter,
        entries,
    })
}

/// Cancels an in-flight [`folder_compare`] for `session` and/or evicts its
/// cached snapshot. `cancelled` is true when a running job or a snapshot
/// was actually found; unknown sessions report `false`, never an error.
#[tauri::command]
pub fn folder_cancel(session: String) -> Result<FolderCancelDto, String> {
    let mut cancelled = false;
    {
        let inflight = lock_inflight()?;
        if let Some(flag) = inflight.get(&session) {
            flag.store(true, Ordering::SeqCst);
            cancelled = true;
        }
    }
    {
        let mut guard = lock_snapshots()?;
        if guard.remove(&session).is_some() {
            cancelled = true;
        }
    }
    Ok(FolderCancelDto { session, cancelled })
}

/// Safely joins a relative path onto `root`, rejecting absolute paths or `..` traversal.
fn resolve_inside_root(root: &Path, rel_path: &str) -> Result<PathBuf, String> {
    let trimmed = rel_path.trim();
    if trimmed.is_empty() {
        return Err("relative path must not be empty".to_string());
    }
    let rel = Path::new(trimmed);
    let mut out = root.to_path_buf();
    for comp in rel.components() {
        match comp {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("unsafe relative path rejected: {rel_path}"));
            }
        }
    }
    Ok(out)
}

/// Copies a file or directory recursively without ever following symlinks.
fn copy_recursive_no_symlinks(src: &Path, dst: &Path) -> Result<u64, String> {
    let meta = std::fs::symlink_metadata(src)
        .map_err(|e| format!("reading metadata for {} failed: {e}", src.display()))?;
    if meta.file_type().is_symlink() {
        return Err(format!("refusing to follow symlink: {}", src.display()));
    }
    if meta.is_dir() {
        std::fs::create_dir_all(dst)
            .map_err(|e| format!("creating directory {} failed: {e}", dst.display()))?;
        let mut total: u64 = 0;
        let entries = std::fs::read_dir(src)
            .map_err(|e| format!("reading directory {} failed: {e}", src.display()))?;
        for item in entries {
            let entry = item.map_err(|e| format!("reading dir entry failed: {e}"))?;
            let child_src = entry.path();
            let child_dst = dst.join(entry.file_name());
            total = total.saturating_add(copy_recursive_no_symlinks(&child_src, &child_dst)?);
        }
        return Ok(total);
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("creating parent directory {} failed: {e}", parent.display()))?;
    }
    std::fs::copy(src, dst)
        .map_err(|e| format!("copying {} to {} failed: {e}", src.display(), dst.display()))
}

/// Copies a single file or directory entry between left and right roots (`"leftToRight"` | `"rightToLeft"`).
#[tauri::command]
pub fn folder_copy_entry(
    session: String,
    rel_path: String,
    dir: String,
) -> Result<FolderActionReceipt, String> {
    let cached = snapshot(&session)?;
    let (src_root, dst_root) = match dir.as_str() {
        "leftToRight" => (&cached.left_root, &cached.right_root),
        "rightToLeft" => (&cached.right_root, &cached.left_root),
        other => return Err(format!("invalid copy direction '{other}'")),
    };
    let src = resolve_inside_root(src_root, &rel_path)?;
    let dst = resolve_inside_root(dst_root, &rel_path)?;
    let bytes_affected = copy_recursive_no_symlinks(&src, &dst)?;
    Ok(FolderActionReceipt {
        session,
        rel_path,
        action: dir,
        bytes_affected,
    })
}

/// Deletes a single file or directory entry on `side` (`"left"` | `"right"`).
#[tauri::command]
pub fn folder_delete_entry(
    session: String,
    rel_path: String,
    side: String,
) -> Result<FolderActionReceipt, String> {
    let cached = snapshot(&session)?;
    let root = match side.as_str() {
        "left" => &cached.left_root,
        "right" => &cached.right_root,
        other => return Err(format!("invalid folder side '{other}'")),
    };
    let target = resolve_inside_root(root, &rel_path)?;
    let meta = std::fs::symlink_metadata(&target)
        .map_err(|e| format!("reading metadata for {} failed: {e}", target.display()))?;
    if meta.file_type().is_symlink() {
        return Err(format!("refusing to delete symlink: {}", target.display()));
    }
    let bytes_affected = meta.len();
    if meta.is_dir() {
        std::fs::remove_dir_all(&target)
            .map_err(|e| format!("deleting directory {} failed: {e}", target.display()))?;
    } else {
        std::fs::remove_file(&target)
            .map_err(|e| format!("deleting file {} failed: {e}", target.display()))?;
    }
    Ok(FolderActionReceipt {
        session,
        rel_path,
        action: format!("delete:{side}"),
        bytes_affected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(rel: &str, status: FolderStatus) -> FolderEntry {
        FolderEntry {
            rel_path: rel.to_string(),
            is_dir: false,
            left: None,
            right: None,
            status,
        }
    }

    #[test]
    fn clamp_window_edges() {
        assert_eq!(clamp_window(10, 0, 5), (0, 5));
        assert_eq!(clamp_window(10, 8, 5), (8, 2));
        assert_eq!(clamp_window(10, 10, 5), (10, 0));
        assert_eq!(clamp_window(10, 0, 0), (0, 0));
        assert_eq!(clamp_window(0, 0, 5), (0, 0));
        assert_eq!(clamp_window(3, 0, 10_000), (0, 3));
        assert_eq!(clamp_window(10, u64::MAX, u64::MAX), (10, 0));
    }

    #[test]
    fn entry_matches_filters() {
        assert_eq!(entry_matches(FolderStatusDto::Same, "all"), Ok(true));
        assert_eq!(entry_matches(FolderStatusDto::Different, "all"), Ok(true));
        assert_eq!(entry_matches(FolderStatusDto::Same, "changed"), Ok(false));
        assert_eq!(
            entry_matches(FolderStatusDto::LeftOnly, "changed"),
            Ok(true)
        );
        assert_eq!(
            entry_matches(FolderStatusDto::NewerLeft, "newerLeft"),
            Ok(true)
        );
        assert_eq!(
            entry_matches(FolderStatusDto::NewerRight, "newerLeft"),
            Ok(false)
        );
        assert!(entry_matches(FolderStatusDto::Same, "bogus").is_err());
    }

    #[test]
    fn status_mapping_covers_every_variant() {
        assert_eq!(
            FolderStatusDto::from_status(FolderStatus::Same),
            FolderStatusDto::Same
        );
        assert_eq!(
            FolderStatusDto::from_status(FolderStatus::Different),
            FolderStatusDto::Different
        );
        assert_eq!(
            FolderStatusDto::from_status(FolderStatus::NewerLeft),
            FolderStatusDto::NewerLeft
        );
        assert_eq!(
            FolderStatusDto::from_status(FolderStatus::NewerRight),
            FolderStatusDto::NewerRight
        );
        assert_eq!(
            FolderStatusDto::from_status(FolderStatus::LeftOnly),
            FolderStatusDto::LeftOnly
        );
        assert_eq!(
            FolderStatusDto::from_status(FolderStatus::RightOnly),
            FolderStatusDto::RightOnly
        );
    }

    #[test]
    fn entry_dto_shape_round_trips_through_json() {
        let dto = FolderEntryDto::from_entry(&entry("a/b.txt", FolderStatus::NewerLeft));
        assert_eq!(dto.status, FolderStatusDto::NewerLeft);
        assert!(!dto.is_dir);
        let json = serde_json::to_string(&dto).unwrap_or_default();
        assert!(json.contains("\"relPath\""));
        assert!(json.contains("\"isDir\""));
        assert!(json.contains("\"newerLeft\""));
        let back: FolderEntryDto = serde_json::from_str(&json)
            .unwrap_or_else(|_| FolderEntryDto::from_entry(&entry("x", FolderStatus::Same)));
        assert_eq!(back, dto);
    }

    #[test]
    fn error_paths_on_missing_dirs_and_sessions() {
        assert!(folder_compare(String::new(), "/b".to_string(), None, None, None, None, None).is_err());
        assert!(folder_compare("/a".to_string(), String::new(), None, None, None, None, None).is_err());
        let missing = "/definitely/not/here-wave1a-left".to_string();
        assert!(folder_compare(
            missing,
            "/definitely/not/here-wave1a-right".to_string(),
            None,
            None,
            None,
            None,
            None
        )
        .is_err());
        assert!(folder_compare(
            "/definitely/not/here-wave1a-left".to_string(),
            "/definitely/not/here-wave1a-right".to_string(),
            Some("*.ts".to_string()),
            None,
            None,
            None,
            None
        )
        .is_err());
        assert!(folder_get_tree("no-such-session".to_string(), 0, 10, "all".to_string()).is_err());
        assert!(
            folder_get_tree("no-such-session".to_string(), 0, 10, "bogus".to_string()).is_err()
        );
    }

    #[test]
    fn cancel_unknown_session_reports_false() {
        let receipt = folder_cancel("no-such-session-wave1a".to_string());
        assert_eq!(receipt.map(|r| r.cancelled), Ok(false));
    }

    fn mask_fixture_roots(tag: &str) -> (String, String, String) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "autocompare-folder-masks-{}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed),
            tag
        ));
        let left = base.join("left");
        let right = base.join("right");
        for name in ["a.ts", "b.js", "app.min.js"] {
            for root in [&left, &right] {
                std::fs::create_dir_all(root).unwrap_or(());
                let path = root.join(name);
                std::fs::write(&path, "shared\n").unwrap_or(());
                // Pin both sides to the same mtime so verdicts are `Same`
                // and the status-filter composition check is deterministic.
                if let Ok(file) = std::fs::File::options().write(true).open(&path) {
                    let stamp = std::time::UNIX_EPOCH
                        .checked_add(std::time::Duration::from_secs(1_700_000_000))
                        .unwrap_or(std::time::UNIX_EPOCH);
                    let _ = file.set_times(
                        std::fs::FileTimes::new().set_modified(stamp),
                    );
                }
            }
        }
        (
            base.display().to_string(),
            left.display().to_string(),
            right.display().to_string(),
        )
    }

    fn tree_paths(session: &str) -> Vec<String> {
        let page = folder_get_tree(session.to_string(), 0, 500, "all".to_string())
            .unwrap_or_else(|_| panic!("tree fetch must succeed for {session}"));
        page.entries.iter().map(|e| e.rel_path.clone()).collect()
    }

    #[test]
    fn masks_param_filters_before_status_filter() {
        // No masks: everything is listed (backward compatible).
        let (base_none, left, right) = mask_fixture_roots("none");
        let dto = folder_compare(left.clone(), right.clone(), None, None, None, None, None).unwrap_or_else(|_| {
            panic!("compare without masks must succeed");
        });
        assert_eq!(dto.total, 3);
        assert_eq!(tree_paths(&dto.session), vec!["a.ts", "app.min.js", "b.js"]);
        folder_cancel(dto.session).unwrap_or_else(|_| panic!("cancel must succeed"));
        std::fs::remove_dir_all(&base_none).unwrap_or(());

        // Include-only: only *.ts survives.
        let (base_inc, left, right) = mask_fixture_roots("inc");
        let dto = folder_compare(left, right, Some("*.ts".to_string()), None, None, None, None)
            .unwrap_or_else(|_| panic!("compare with masks must succeed"));
        assert_eq!(dto.total, 1);
        assert_eq!(tree_paths(&dto.session), vec!["a.ts"]);
        // Status filter composes on the masked snapshot.
        let changed = folder_get_tree(dto.session.clone(), 0, 500, "changed".to_string())
            .unwrap_or_else(|_| panic!("filtered fetch must succeed"));
        assert!(changed.entries.is_empty());
        folder_cancel(dto.session).unwrap_or_else(|_| panic!("cancel must succeed"));
        std::fs::remove_dir_all(&base_inc).unwrap_or(());

        // Exclude-only: the minified bundle is hidden.
        let (base_exc, left, right) = mask_fixture_roots("exc");
        let dto = folder_compare(left, right, Some("-*.min.js".to_string()), None, None, None, None)
            .unwrap_or_else(|_| panic!("compare with masks must succeed"));
        assert_eq!(dto.total, 2);
        assert_eq!(tree_paths(&dto.session), vec!["a.ts", "b.js"]);
        folder_cancel(dto.session).unwrap_or_else(|_| panic!("cancel must succeed"));
        std::fs::remove_dir_all(&base_exc).unwrap_or(());

        // Mixed: include *.js but still drop the minified bundle.
        let (base_mix, left, right) = mask_fixture_roots("mix");
        let dto = folder_compare(left, right, Some("*.js;-*.min.js".to_string()), None, None, None, None)
            .unwrap_or_else(|_| panic!("compare with masks must succeed"));
        assert_eq!(dto.total, 1);
        assert_eq!(tree_paths(&dto.session), vec!["b.js"]);
        folder_cancel(dto.session).unwrap_or_else(|_| panic!("cancel must succeed"));
        std::fs::remove_dir_all(&base_mix).unwrap_or(());

        // Empty masks string behaves like None.
        let (base_empty, left, right) = mask_fixture_roots("empty");
        let dto = folder_compare(left, right, Some("  ".to_string()), None, None, None, None)
            .unwrap_or_else(|_| panic!("compare with empty masks must succeed"));
        assert_eq!(dto.total, 3);
        folder_cancel(dto.session).unwrap_or_else(|_| panic!("cancel must succeed"));
        std::fs::remove_dir_all(&base_empty).unwrap_or(());
    }

    #[test]
    fn content_compare_flag_byte_compares_same_size_files() {
        // Same size + same mtime + different bytes: fast path reports
        // `Same`, content path reports `Different`.
        let base = std::env::temp_dir().join(format!(
            "autocompare-folder-content-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let left = base.join("left");
        let right = base.join("right");
        std::fs::create_dir_all(&left).unwrap_or(());
        std::fs::create_dir_all(&right).unwrap_or(());
        std::fs::write(left.join("same.bin"), b"aaaa").unwrap_or(());
        std::fs::write(right.join("same.bin"), b"bbbb").unwrap_or(());
        let stamp = std::time::UNIX_EPOCH
            .checked_add(std::time::Duration::from_secs(1_700_000_000))
            .unwrap_or(std::time::UNIX_EPOCH);
        for root in [&left, &right] {
            if let Ok(file) = std::fs::File::options()
                .write(true)
                .open(root.join("same.bin"))
            {
                let _ = file.set_times(
                    std::fs::FileTimes::new().set_modified(stamp),
                );
            }
        }
        let l = left.display().to_string();
        let r = right.display().to_string();
        // Fast path (default): mtime equal => Same.
        let dto = folder_compare(l.clone(), r.clone(), None, None, None, None, None)
            .unwrap_or_else(|_| panic!("fast compare must succeed"));
        assert_eq!(dto.total, 1);
        let page = folder_get_tree(dto.session.clone(), 0, 500, "all".to_string())
            .unwrap_or_else(|_| panic!("tree fetch must succeed"));
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].status, FolderStatusDto::Same);
        folder_cancel(dto.session).unwrap_or_else(|_| panic!("cancel must succeed"));
        // Content path (camelCase spelling): bytes differ => Different.
        let dto = folder_compare(l.clone(), r.clone(), None, Some(true), None, None, None)
            .unwrap_or_else(|_| panic!("content compare must succeed"));
        let page = folder_get_tree(dto.session.clone(), 0, 500, "all".to_string())
            .unwrap_or_else(|_| panic!("tree fetch must succeed"));
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].status, FolderStatusDto::Different);
        folder_cancel(dto.session).unwrap_or_else(|_| panic!("cancel must succeed"));
        // Quick alias (quick=true => fast path => Same again).
        let dto = folder_compare(l, r, None, None, None, Some(true), None)
            .unwrap_or_else(|_| panic!("quick compare must succeed"));
        let page = folder_get_tree(dto.session.clone(), 0, 500, "all".to_string())
            .unwrap_or_else(|_| panic!("tree fetch must succeed"));
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].status, FolderStatusDto::Same);
        folder_cancel(dto.session).unwrap_or_else(|_| panic!("cancel must succeed"));
        std::fs::remove_dir_all(&base).unwrap_or(());
    }

    #[test]
    fn copy_and_delete_entry_work_and_block_traversal() {
        let (base, left, right) = mask_fixture_roots("copy-del");
        let extra = PathBuf::from(&left).join("only_left.txt");
        let _ = std::fs::write(&extra, "hello world");
        let dto = folder_compare(left.clone(), right.clone(), None, None, None, None, None)
            .expect("compare must succeed");
        assert!(folder_copy_entry(dto.session.clone(), "../escape.txt".to_string(), "leftToRight".to_string()).is_err());
        let copied = folder_copy_entry(dto.session.clone(), "only_left.txt".to_string(), "leftToRight".to_string());
        assert!(copied.is_ok());
        assert!(PathBuf::from(&right).join("only_left.txt").exists());
        let deleted = folder_delete_entry(dto.session.clone(), "only_left.txt".to_string(), "right".to_string());
        assert!(deleted.is_ok());
        assert!(!PathBuf::from(&right).join("only_left.txt").exists());
        let _ = folder_cancel(dto.session);
        let _ = std::fs::remove_dir_all(&base);
    }
}
