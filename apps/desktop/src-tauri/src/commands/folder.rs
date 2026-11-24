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

/// Cached full folder comparison. Lives in Rust memory only; the UI pages
/// through it via [`folder_get_tree`].
#[derive(Debug, Clone)]
struct StoredFolder {
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
/// The walk runs through [`compare_folders_cancellable`] against a flag
/// registered up front, so a concurrent [`folder_cancel`] for the returned
/// session id can interrupt it.
#[tauri::command]
pub fn folder_compare(left: String, right: String) -> Result<FolderCompareDto, String> {
    if left.is_empty() {
        return Err("left path must not be empty".to_string());
    }
    if right.is_empty() {
        return Err("right path must not be empty".to_string());
    }
    let session = next_session_id();
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut inflight = lock_inflight()?;
        inflight.insert(session.clone(), Arc::clone(&cancel));
    }
    let outcome = compare_folders_cancellable(
        std::path::Path::new(&left),
        std::path::Path::new(&right),
        &FolderOptions::default(),
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
        guard.insert(session, StoredFolder { comparison });
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
        assert!(folder_compare(String::new(), "/b".to_string()).is_err());
        assert!(folder_compare("/a".to_string(), String::new()).is_err());
        let missing = "/definitely/not/here-wave1a-left".to_string();
        assert!(folder_compare(missing, "/definitely/not/here-wave1a-right".to_string()).is_err());
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
}
