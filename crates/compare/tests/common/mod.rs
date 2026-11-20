//! Shared helpers for the integration tests: unique temp dirs,
//! deterministic fixtures and entry lookups. Every helper is exercised
//! by the smoke test below, so no binary needs its own allow.

#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use autocompare_compare::FolderEntry;

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Creates a unique, empty temp directory for one test.
pub fn temp_root() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "autocompare-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("failed to create the temp dir");
    dir
}

/// Best-effort cleanup; removal errors are ignored.
pub fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

/// Writes a text fixture, creating parent directories as needed.
pub fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create the parent dir");
    }
    std::fs::write(path, contents).expect("failed to write the fixture");
}

/// Writes a raw-bytes fixture, creating parent directories as needed.
pub fn write_bytes(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create the parent dir");
    }
    std::fs::write(path, contents).expect("failed to write the fixture");
}

/// A fixed timestamp in the past, for equal-mtime fixtures.
pub fn fixed_time() -> SystemTime {
    UNIX_EPOCH
        .checked_add(Duration::from_secs(1_700_000_000))
        .unwrap_or(UNIX_EPOCH)
}

/// Overwrites a file's modified time for deterministic fixtures.
pub fn set_mtime(path: &Path, modified: SystemTime) {
    let file = std::fs::File::options()
        .write(true)
        .open(path)
        .expect("failed to open the fixture to set times");
    file.set_times(std::fs::FileTimes::new().set_modified(modified))
        .expect("failed to set the fixture mtime");
}

/// The relative paths of all entries, in reported order.
pub fn rel_paths(entries: &[FolderEntry]) -> Vec<String> {
    entries.iter().map(|entry| entry.rel_path.clone()).collect()
}

/// Finds the entry with the given relative path.
pub fn entry<'a>(entries: &'a [FolderEntry], rel: &str) -> &'a FolderEntry {
    entries
        .iter()
        .find(|e| e.rel_path == rel)
        .expect("expected entry is missing")
}

#[test]
fn smoke_exercises_all_shared_helpers() {
    use autocompare_compare::{FolderStatus, NodeInfo};

    let dir = temp_root();
    let text_path = dir.join("note.txt");
    write(&text_path, "hello");
    assert!(text_path.exists());
    let bin_path = dir.join("blob.bin");
    write_bytes(&bin_path, &[0, 1, 2, 3]);
    assert!(bin_path.exists());

    let stamp = fixed_time();
    set_mtime(&bin_path, stamp);

    let entries = vec![
        FolderEntry {
            rel_path: "note.txt".to_string(),
            is_dir: false,
            left: Some(NodeInfo {
                size: 5,
                modified: stamp,
            }),
            right: None,
            status: FolderStatus::LeftOnly,
        },
        FolderEntry {
            rel_path: "blob.bin".to_string(),
            is_dir: false,
            left: None,
            right: None,
            status: FolderStatus::RightOnly,
        },
    ];
    assert_eq!(
        rel_paths(&entries),
        vec!["note.txt".to_string(), "blob.bin".to_string()]
    );
    assert_eq!(entry(&entries, "blob.bin").rel_path, "blob.bin");

    cleanup(&dir);
}
