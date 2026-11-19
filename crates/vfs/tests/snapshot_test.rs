//! Integration tests for .bcss Snapshot serialization, deserialization, and SnapshotFs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use autocompare_vfs::{
    BCSS_SCHEMA_VERSION, LocalFs, SnapshotDocument, SnapshotEntry, SnapshotFs, Vfs, VfsError,
    VfsPath, create_snapshot, deserialize_bcss, serialize_bcss_compressed, serialize_bcss_json,
};

#[test]
fn test_snapshot_document_round_trip() {
    let doc = SnapshotDocument {
        schema_version: BCSS_SCHEMA_VERSION,
        created_at_epoch_secs: 1_700_000_000,
        generator: "autocompare-vfs v0.5.0".to_string(),
        root_name: "test-snapshot".to_string(),
        description: Some("Test BCSS Snapshot".to_string()),
        entries: vec![
            SnapshotEntry {
                path: "folder".to_string(),
                is_dir: true,
                size: 0,
                modified_epoch_secs: Some(1_700_000_000),
                crc32: None,
                readonly: false,
            },
            SnapshotEntry {
                path: "folder/data.bin".to_string(),
                is_dir: false,
                size: 2048,
                modified_epoch_secs: Some(1_700_000_000),
                crc32: Some(0xDEAD_BEEF),
                readonly: true,
            },
        ],
    };

    // 1. JSON round-trip
    let json_bytes = serialize_bcss_json(&doc).unwrap();
    let parsed_doc = deserialize_bcss(&json_bytes).unwrap();
    assert_eq!(doc, parsed_doc);

    // 2. Gzip-compressed round-trip
    let gz_bytes = serialize_bcss_compressed(&doc).unwrap();
    let parsed_gz_doc = deserialize_bcss(&gz_bytes).unwrap();
    assert_eq!(doc, parsed_gz_doc);

    // 3. Schema version mismatch
    let mut invalid_doc = doc.clone();
    invalid_doc.schema_version = 999;
    let invalid_json = serde_json::to_vec(&invalid_doc).unwrap();
    let err = deserialize_bcss(&invalid_json).unwrap_err();
    assert!(matches!(
        err,
        VfsError::SchemaVersionMismatch {
            expected: BCSS_SCHEMA_VERSION,
            found: 999
        }
    ));
}

#[test]
fn test_create_snapshot_and_browse_snapshot_fs() {
    let temp_dir = std::env::temp_dir().join(format!("vfs_snap_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("src")).unwrap();

    let file_path = temp_dir.join("src").join("main.rs");
    std::fs::write(&file_path, b"fn main() {}").unwrap();

    let local_vfs = LocalFs::new(&temp_dir).unwrap();
    let doc = create_snapshot(
        &local_vfs,
        &VfsPath::root(),
        "my_project",
        Some("Created by test".to_string()),
    )
    .unwrap();

    assert_eq!(doc.schema_version, BCSS_SCHEMA_VERSION);
    assert_eq!(doc.root_name, "my_project");
    assert!(!doc.entries.is_empty());

    // Verify CRC32 was computed for main.rs
    let main_entry = doc
        .entries
        .iter()
        .find(|e| e.path == "src/main.rs")
        .expect("should have src/main.rs");
    assert_eq!(main_entry.size, 12);
    assert!(main_entry.crc32.is_some());

    // Load into SnapshotFs
    let snap_vfs = SnapshotFs::new(doc).unwrap();
    assert!(snap_vfs.is_read_only());

    // List root of snapshot
    let root_entries = snap_vfs.list_dir(&VfsPath::root()).unwrap();
    assert_eq!(root_entries.len(), 1);
    assert_eq!(root_entries.first().unwrap().name, "src");

    // List src/ of snapshot
    let src_path = VfsPath::new("src").unwrap();
    let src_entries = snap_vfs.list_dir(&src_path).unwrap();
    assert_eq!(src_entries.len(), 1);
    assert_eq!(src_entries.first().unwrap().name, "main.rs");

    // Stat file in snapshot
    let main_path = VfsPath::new("src/main.rs").unwrap();
    let main_meta = snap_vfs.stat(&main_path).unwrap();
    assert_eq!(main_meta.len, 12);
    assert!(main_meta.crc32.is_some());

    // Reading stream in snapshot returns UnsupportedFormat (metadata only)
    let read_res = snap_vfs.read_stream(&main_path, 1024);
    assert!(matches!(read_res, Err(VfsError::UnsupportedFormat(_))));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
