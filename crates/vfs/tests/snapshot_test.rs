//! Integration tests for .bcss Snapshot serialization, deserialization, and SnapshotFs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use autocompare_vfs::{
    BCSS_SCHEMA_VERSION, LocalFs, SnapshotDocument, SnapshotEntry, SnapshotFs, Vfs, VfsError,
    VfsPath, WalkOptions, create_snapshot, deserialize_bcss, serialize_bcss_compressed,
    serialize_bcss_json,
    snapshot::create_snapshot_with_options,
    trait_def::{FakeRemoteFs, RemoteConnectionState, RemoteEndpoint, RemoteKind, RemoteVfs},
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

#[test]
fn test_snapshot_fs_file_io_and_helpers() {
    let temp_dir = std::env::temp_dir().join(format!("vfs_snap_io_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let doc = SnapshotDocument::new("io_test")
        .with_description("Unit test for snapshot I/O")
        .with_entries(vec![
            SnapshotEntry {
                path: "pkg".to_string(),
                is_dir: true,
                size: 0,
                modified_epoch_secs: Some(1_700_000_000),
                crc32: None,
                readonly: false,
            },
            SnapshotEntry {
                path: "pkg/lib.rs".to_string(),
                is_dir: false,
                size: 120,
                modified_epoch_secs: Some(1_700_000_000),
                crc32: Some(0x1234_5678),
                readonly: false,
            },
        ]);

    let snap_vfs = SnapshotFs::new(doc).unwrap();
    assert_eq!(snap_vfs.file_count(), 1);
    assert_eq!(snap_vfs.dir_count(), 1);
    assert_eq!(snap_vfs.total_size(), 120);
    assert_eq!(snap_vfs.root_name(), "io_test");

    // Save as JSON file
    let json_file = temp_dir.join("snap.json");
    snap_vfs.save_json_file(&json_file).unwrap();
    let loaded_json = SnapshotFs::open_file(&json_file).unwrap();
    assert_eq!(loaded_json.file_count(), 1);

    // Save as compressed .bcss file
    let bcss_file = temp_dir.join("snap.bcss");
    snap_vfs.save_compressed_file(&bcss_file).unwrap();
    let loaded_bcss = SnapshotFs::open_file(&bcss_file).unwrap();
    assert_eq!(loaded_bcss.file_count(), 1);

    // Look up entry
    let lib_path = VfsPath::new("pkg/lib.rs").unwrap();
    let entry = snap_vfs.find_entry(&lib_path).unwrap();
    assert_eq!(entry.size, 120);
    assert_eq!(entry.crc32, Some(0x1234_5678));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_create_snapshot_with_options_shallow() {
    let temp_dir = std::env::temp_dir().join(format!("vfs_snap_opt_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let deep_dir = temp_dir.join("level1").join("level2");
    std::fs::create_dir_all(&deep_dir).unwrap();

    std::fs::write(temp_dir.join("top.txt"), b"top file").unwrap();
    std::fs::write(temp_dir.join("level1").join("mid.txt"), b"mid file").unwrap();
    std::fs::write(deep_dir.join("deep.txt"), b"deep file").unwrap();

    let local_vfs = LocalFs::new(&temp_dir).unwrap();

    // Shallow walk (depth 1)
    let shallow_opt = WalkOptions::shallow();
    let doc_shallow = create_snapshot_with_options(
        &local_vfs,
        &VfsPath::root(),
        "shallow_test",
        None,
        &shallow_opt,
    )
    .unwrap();

    // Shallow should only include top.txt and level1 directory
    assert_eq!(doc_shallow.file_count(), 1);
    assert!(doc_shallow.find_entry("top.txt").is_some());
    assert!(doc_shallow.find_entry("level1/mid.txt").is_none());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_snapshot_document_diff_summary() {
    let doc1 = SnapshotDocument::new("snap1").with_entries(vec![
        SnapshotEntry {
            path: "same.txt".to_string(),
            is_dir: false,
            size: 10,
            modified_epoch_secs: None,
            crc32: Some(100),
            readonly: false,
        },
        SnapshotEntry {
            path: "modified.txt".to_string(),
            is_dir: false,
            size: 20,
            modified_epoch_secs: None,
            crc32: Some(200),
            readonly: false,
        },
        SnapshotEntry {
            path: "left_only.txt".to_string(),
            is_dir: false,
            size: 5,
            modified_epoch_secs: None,
            crc32: None,
            readonly: false,
        },
    ]);

    let doc2 = SnapshotDocument::new("snap2").with_entries(vec![
        SnapshotEntry {
            path: "same.txt".to_string(),
            is_dir: false,
            size: 10,
            modified_epoch_secs: None,
            crc32: Some(100),
            readonly: false,
        },
        SnapshotEntry {
            path: "modified.txt".to_string(),
            is_dir: false,
            size: 25, // different size
            modified_epoch_secs: None,
            crc32: Some(201),
            readonly: false,
        },
        SnapshotEntry {
            path: "right_only.txt".to_string(),
            is_dir: false,
            size: 8,
            modified_epoch_secs: None,
            crc32: None,
            readonly: false,
        },
    ]);

    let diff = doc1.diff_summary(&doc2);
    assert_eq!(diff.matching, 1);
    assert_eq!(diff.modified, 1);
    assert_eq!(diff.unique_left, 1);
    assert_eq!(diff.unique_right, 1);
    assert_eq!(diff.total_entries(), 4);
    assert!(!diff.is_identical());
    assert!(diff.has_differences());

    let detailed = doc1.detailed_diff(&doc2);
    assert_eq!(detailed.identical, vec!["same.txt"]);
    assert_eq!(detailed.modified, vec!["modified.txt"]);
    assert_eq!(detailed.unique_left, vec!["left_only.txt"]);
    assert_eq!(detailed.unique_right, vec!["right_only.txt"]);
    assert_eq!(detailed.summary(), diff);
    assert!(!detailed.is_identical());

    // Identical snapshots
    let diff_self = doc1.diff_summary(&doc1);
    assert!(diff_self.is_identical());
    assert!(!diff_self.has_differences());
    assert_eq!(diff_self.matching, 3);
}

#[test]
fn test_snapshot_document_entry_helpers() {
    let mut doc = SnapshotDocument::new("helper_test").with_entries(vec![
        SnapshotEntry {
            path: "src/lib.rs".to_string(),
            is_dir: false,
            size: 100,
            modified_epoch_secs: None,
            crc32: Some(1),
            readonly: false,
        },
        SnapshotEntry {
            path: "src/main.rs".to_string(),
            is_dir: false,
            size: 200,
            modified_epoch_secs: None,
            crc32: Some(2),
            readonly: false,
        },
        SnapshotEntry {
            path: "docs/readme.md".to_string(),
            is_dir: false,
            size: 50,
            modified_epoch_secs: None,
            crc32: None,
            readonly: false,
        },
    ]);

    // entries_with_extension
    let rs_entries = doc.entries_with_extension("rs");
    assert_eq!(rs_entries.len(), 2);
    let md_entries = doc.entries_with_extension(".MD");
    assert_eq!(md_entries.len(), 1);

    // filter_by_prefix
    let src_entries = doc.filter_by_prefix("src");
    assert_eq!(src_entries.len(), 2);

    // find_entry_mut
    let entry_mut = doc.find_entry_mut("src/lib.rs").unwrap();
    entry_mut.size = 150;
    assert_eq!(doc.find_entry("src/lib.rs").unwrap().size, 150);

    // remove_entry
    let removed = doc.remove_entry("docs/readme.md").unwrap();
    assert_eq!(removed.path, "docs/readme.md");
    assert_eq!(doc.entries.len(), 2);
    assert!(doc.remove_entry("nonexistent").is_none());
}

#[test]
fn test_create_deep_and_shallow_snapshot_functions() {
    let temp_dir =
        std::env::temp_dir().join(format!("vfs_snap_deep_shallow_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let deep = temp_dir.join("a").join("b").join("c");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(temp_dir.join("root.txt"), b"root").unwrap();
    std::fs::write(temp_dir.join("a").join("a.txt"), b"a").unwrap();
    std::fs::write(temp_dir.join("a").join("b").join("b.txt"), b"b").unwrap();
    std::fs::write(deep.join("c.txt"), b"c").unwrap();

    let local_vfs = LocalFs::new(&temp_dir).unwrap();

    // 1. Shallow snapshot (depth 1)
    let shallow = autocompare_vfs::create_shallow_snapshot(
        &local_vfs,
        &VfsPath::root(),
        "shallow_root",
        None,
    )
    .unwrap();
    assert_eq!(shallow.file_count(), 1); // only root.txt
    assert!(shallow.find_entry("root.txt").is_some());
    assert!(shallow.find_entry("a/a.txt").is_none());

    // 2. Deep snapshot bounded by depth 2
    let deep_snap =
        autocompare_vfs::create_deep_snapshot(&local_vfs, &VfsPath::root(), "deep_root", None, 2)
            .unwrap();
    assert!(deep_snap.find_entry("root.txt").is_some());
    assert!(deep_snap.find_entry("a/a.txt").is_some());
    assert!(deep_snap.find_entry("a/b/b.txt").is_none());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_remote_vfs_fake_sftp_webdav() {
    // 1. SFTP endpoint and FakeRemoteFs
    let sftp_endpoint =
        RemoteEndpoint::new(RemoteKind::Sftp, "sftp.internal.local", VfsPath::root())
            .with_user("operator")
            .with_port(2222);

    let remote = FakeRemoteFs::new(sftp_endpoint);
    assert_eq!(remote.protocol(), RemoteKind::Sftp);
    assert_eq!(remote.endpoint().user.as_deref(), Some("operator"));
    assert_eq!(remote.endpoint().port, 2222);
    assert!(!remote.endpoint().tls);
    assert_eq!(remote.connection_state(), RemoteConnectionState::Connected);

    // Ping check
    let ping_latency = remote.ping().unwrap();
    assert!(ping_latency >= std::time::Duration::ZERO);

    // Virtual filesystem operations
    let file_path = VfsPath::new("remote_data.txt").unwrap();
    remote.write_atomic(&file_path, b"sftp payload").unwrap();

    let meta = remote.stat(&file_path).unwrap();
    assert_eq!(meta.len, 12);
    assert!(meta.is_file);

    let content = remote.read_all(&file_path, 1024).unwrap();
    assert_eq!(content, b"sftp payload");

    let list = remote.list_dir(&VfsPath::root()).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list.first().unwrap().name, "remote_data.txt");

    // 2. WebDAV endpoint
    let webdav_endpoint =
        RemoteEndpoint::new(RemoteKind::WebDav, "dav.internal.local", VfsPath::root());
    assert_eq!(webdav_endpoint.kind, RemoteKind::WebDav);
    assert_eq!(webdav_endpoint.port, 443);
    assert!(webdav_endpoint.tls);
}

#[test]
fn test_remote_vfs_disconnect_reconnect_and_ping() {
    let endpoint = RemoteEndpoint::new(RemoteKind::WebDav, "dav.cloud.local", VfsPath::root());
    let mut remote = FakeRemoteFs::new(endpoint);

    let file_path = VfsPath::new("test.dat").unwrap();
    remote.write_atomic(&file_path, b"data bytes").unwrap();

    // Disconnect simulation
    remote.disconnect().unwrap();
    assert_eq!(
        remote.connection_state(),
        RemoteConnectionState::Disconnected
    );

    // Ping should fail when disconnected
    assert!(remote.ping().is_err());

    // IO should fail when disconnected
    assert!(matches!(remote.stat(&file_path), Err(VfsError::Io(_))));
    assert!(matches!(
        remote.write_atomic(&file_path, b"fail"),
        Err(VfsError::Io(_))
    ));
    assert!(matches!(
        remote.list_dir(&VfsPath::root()),
        Err(VfsError::Io(_))
    ));

    // Reconnect
    remote.reconnect().unwrap();
    assert_eq!(remote.connection_state(), RemoteConnectionState::Connected);
    assert!(remote.ping().is_ok());

    // Verify data retained across reconnect
    let read_back = remote.read_all(&file_path, 1024).unwrap();
    assert_eq!(read_back, b"data bytes");
}

#[test]
fn test_vfs_walk_options_and_cycle_refusal() {
    let temp_dir = std::env::temp_dir().join(format!("vfs_walk_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let deep = temp_dir.join("a").join("b").join("c");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("leaf.txt"), b"leaf").unwrap();

    let local_vfs = LocalFs::new(&temp_dir).unwrap();

    // Test WalkOptions presets
    let std_opts = WalkOptions::standard();
    assert_eq!(std_opts.max_depth, None);
    assert!(std_opts.refuse_cycles);
    assert!(!std_opts.follow_symlinks);

    let unbounded = WalkOptions::unbounded();
    assert_eq!(unbounded.max_depth, None);

    let bounded = WalkOptions::bounded(2);
    assert_eq!(bounded.max_depth, Some(2));

    let shallow = WalkOptions::shallow();
    assert_eq!(shallow.max_depth, Some(1));

    // Walk with depth limit = 2
    let entries = local_vfs.walk(&VfsPath::root(), &bounded).unwrap();
    // At root: "a" is depth 1; "a/b" is depth 2; "a/b/c" is depth 3 (excluded)
    assert!(entries.iter().any(|e| e.name == "a"));
    assert!(entries.iter().any(|e| e.name == "b"));
    assert!(!entries.iter().any(|e| e.name == "leaf.txt"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
