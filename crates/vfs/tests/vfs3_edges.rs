//! Round-vfs3 edge tests: LocalFS symlink safety, Zip/Tar.gz truncated and
//! hostile archives, and `.bcss` snapshot round-trip plus diffs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::{Cursor, Write};
use std::os::unix::fs::symlink;
use std::path::PathBuf;

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use autocompare_vfs::{
    LocalFs, SnapshotFs, TarGzFs, Vfs, VfsError, VfsPath, WalkOptions, ZipFs,
    archive_tar::create_tar_gz_archive, archive_zip::create_zip_archive, deserialize_bcss,
    serialize_bcss_compressed, serialize_bcss_json,
};

/// Returns an isolated scratch directory for the calling edge test.
fn scratch_dir(case: &str) -> PathBuf {
    std::env::temp_dir().join(format!("vfs3_edge_{case}_{}", std::process::id()))
}

#[test]
fn local_self_loop_cycle_is_refused() {
    let base = scratch_dir("selfloop");
    let _ = fs::remove_dir_all(&base);
    let dir_a = base.join("dir_a");
    fs::create_dir_all(&dir_a).unwrap();
    symlink(&dir_a, dir_a.join("loop")).unwrap();

    let vfs = LocalFs::new(&base).unwrap();

    // Without following symlinks the loop is listed, never traversed.
    let listed = vfs
        .walk(&VfsPath::root(), &WalkOptions::standard())
        .unwrap();
    let link = listed.iter().find(|e| e.name == "loop").unwrap();
    assert!(link.meta.is_symlink);

    // Following the self-loop with cycle refusal must fail loudly.
    let follow = WalkOptions::standard().with_follow_symlinks(true);
    let err = vfs.walk(&VfsPath::root(), &follow).unwrap_err();
    assert!(matches!(err, VfsError::SymlinkCycle(_)));

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn local_dangling_symlink_is_tolerated() {
    let base = scratch_dir("dangling");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    symlink(base.join("no_such_target"), base.join("dangling")).unwrap();

    let vfs = LocalFs::new(&base).unwrap();
    let dangling = VfsPath::new("dangling").unwrap();
    let meta = vfs.stat(&dangling).unwrap();
    assert!(meta.is_symlink);

    // Walks (follow or not) must not fail on a dangling link.
    let plain = vfs
        .walk(&VfsPath::root(), &WalkOptions::standard())
        .unwrap();
    assert!(plain.iter().any(|e| e.name == "dangling"));
    let follow = WalkOptions::standard().with_follow_symlinks(true);
    vfs.walk(&VfsPath::root(), &follow).unwrap();

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn local_refuse_cycles_false_tolerates_loop() {
    let base = scratch_dir("nocycle");
    let _ = fs::remove_dir_all(&base);
    let dir_a = base.join("dir_a");
    let dir_b = dir_a.join("dir_b");
    fs::create_dir_all(&dir_b).unwrap();
    symlink(&dir_a, dir_b.join("cycle_to_a")).unwrap();

    let vfs = LocalFs::new(&base).unwrap();
    let lenient = WalkOptions::standard()
        .with_follow_symlinks(true)
        .with_refuse_cycles(false);
    let entries = vfs.walk(&VfsPath::root(), &lenient).unwrap();
    assert!(entries.iter().any(|e| e.name == "cycle_to_a"));

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn local_delete_symlink_keeps_outside_target() {
    let base = scratch_dir("delsym");
    let outside = std::env::temp_dir().join(format!("vfs3_edge_outside_{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let _ = fs::remove_dir_all(&outside);
    fs::create_dir_all(&base).unwrap();
    fs::create_dir_all(&outside).unwrap();

    let secret = outside.join("secret.txt");
    fs::write(&secret, b"outside secret").unwrap();
    symlink(&secret, base.join("link_to_secret")).unwrap();

    let vfs = LocalFs::new(&base).unwrap();
    vfs.delete(&VfsPath::new("link_to_secret").unwrap())
        .unwrap();

    // The link is gone but the outside target is untouched.
    assert!(!vfs.exists(&VfsPath::new("link_to_secret").unwrap()));
    assert_eq!(fs::read(&secret).unwrap(), b"outside secret");

    let _ = fs::remove_dir_all(&base);
    let _ = fs::remove_dir_all(&outside);
}

#[test]
fn local_in_root_symlink_read_succeeds() {
    let base = scratch_dir("inlink");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    fs::write(base.join("real.txt"), b"real bytes").unwrap();
    symlink(base.join("real.txt"), base.join("link.txt")).unwrap();

    let vfs = LocalFs::new(&base).unwrap();
    let link = VfsPath::new("link.txt").unwrap();
    assert!(vfs.stat(&link).unwrap().is_symlink);
    assert_eq!(vfs.read_all(&link, 1024).unwrap(), b"real bytes");

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn zip_truncated_tail_is_rejected() {
    let p1 = VfsPath::new("a.txt").unwrap();
    let p2 = VfsPath::new("sub/b.txt").unwrap();
    let entries = [
        (&p1, b"alpha payload".as_slice()),
        (&p2, b"beta payload".as_slice()),
    ];
    let mut bytes = create_zip_archive(&entries).unwrap();
    bytes.truncate(bytes.len() / 2);

    let res = ZipFs::from_bytes(bytes);
    assert!(matches!(res, Err(VfsError::CorruptedArchive(_))));
}

#[test]
fn zip_hostile_absolute_path_is_contained() {
    let mut buffer = Vec::new();
    let mut writer = ZipWriter::new(Cursor::new(&mut buffer));
    let options = SimpleFileOptions::default();
    writer.start_file("/abs-vfs3.txt", options).unwrap();
    writer.write_all(b"abs").unwrap();
    writer.start_file("ok.txt", options).unwrap();
    writer.write_all(b"ok").unwrap();
    writer.finish().unwrap();

    let vfs = ZipFs::from_bytes(buffer).unwrap();
    // Leading slashes normalize inside the archive root; nothing escapes.
    let contained = VfsPath::new("abs-vfs3.txt").unwrap();
    assert!(vfs.contains(&contained));
    assert_eq!(vfs.read_all(&contained, 1024).unwrap(), b"abs");
    assert_eq!(vfs.file_count(), 2);
}

#[test]
fn zip_empty_archive_is_empty() {
    let empty: [(&VfsPath, &[u8]); 0] = [];
    let bytes = create_zip_archive(&empty).unwrap();
    let vfs = ZipFs::from_bytes(bytes).unwrap();
    assert!(vfs.is_empty());
    assert_eq!(vfs.file_count(), 0);
    assert!(vfs.list_dir(&VfsPath::root()).unwrap().is_empty());
}

#[test]
fn tar_truncated_gzip_is_rejected() {
    let p1 = VfsPath::new("doc.txt").unwrap();
    let entries = [(&p1, b"tar gzip payload".as_slice())];
    let mut bytes = create_tar_gz_archive(&entries).unwrap();
    bytes.truncate(bytes.len() / 2);

    let res = TarGzFs::from_bytes(&bytes);
    assert!(matches!(res, Err(VfsError::CorruptedArchive(_))));
}

#[test]
fn tar_hostile_absolute_path_is_contained() {
    let mut builder = tar::Builder::new(Vec::new());

    let mut evil = tar::Header::new_gnu();
    evil.set_size(3);
    evil.set_mode(0o644);
    evil.as_mut_bytes()[..17].copy_from_slice(b"/abs-tar-vfs3.txt");
    evil.set_cksum();
    builder.append(&evil, Cursor::new(b"abs")).unwrap();

    let mut safe = tar::Header::new_gnu();
    safe.set_size(2);
    safe.set_mode(0o644);
    safe.set_cksum();
    builder
        .append_data(&mut safe, "ok.txt", b"ok".as_slice())
        .unwrap();

    let bytes = builder.into_inner().unwrap();
    let vfs = TarGzFs::from_bytes(&bytes).unwrap();

    let contained = VfsPath::new("abs-tar-vfs3.txt").unwrap();
    assert!(vfs.contains(&contained));
    assert_eq!(vfs.read_all(&contained, 1024).unwrap(), b"abs");
    assert_eq!(vfs.file_count(), 2);
}

#[test]
fn tar_symlink_entry_is_not_a_file() {
    let mut builder = tar::Builder::new(Vec::new());

    let mut file_header = tar::Header::new_gnu();
    file_header.set_size(5);
    file_header.set_mode(0o644);
    file_header.set_cksum();
    builder
        .append_data(&mut file_header, "hello.txt", b"hello".as_slice())
        .unwrap();

    let mut link_header = tar::Header::new_gnu();
    link_header.set_entry_type(tar::EntryType::Symlink);
    link_header.set_path("hello-link").unwrap();
    link_header.set_link_name("hello.txt").unwrap();
    link_header.set_size(0);
    link_header.set_mode(0o777);
    link_header.set_cksum();
    builder
        .append(&link_header, Cursor::new(Vec::new()))
        .unwrap();

    let bytes = builder.into_inner().unwrap();
    let vfs = TarGzFs::from_bytes(&bytes).unwrap();

    let link_path = VfsPath::new("hello-link").unwrap();
    let meta = vfs.stat(&link_path).unwrap();
    assert!(meta.is_symlink);
    assert!(matches!(
        vfs.read_stream(&link_path, 1024),
        Err(VfsError::NotAFile(_))
    ));
}

#[test]
fn bcss_fixture_round_trip() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("edge-bcss-minimal.json");
    let raw = fs::read(&fixture).unwrap();

    let doc = deserialize_bcss(&raw).unwrap();
    assert_eq!(doc.schema_version, 1);
    assert_eq!(doc.root_name, "vfs3-edge");
    assert_eq!(doc.entries.len(), 3);

    // JSON and gzip forms both round-trip byte-identically at the document level.
    let json = serialize_bcss_json(&doc).unwrap();
    assert_eq!(deserialize_bcss(&json).unwrap(), doc);
    let gz = serialize_bcss_compressed(&doc).unwrap();
    assert_eq!(deserialize_bcss(&gz).unwrap(), doc);

    let snap = SnapshotFs::new(doc).unwrap();
    assert!(snap.is_read_only());
    assert_eq!(snap.file_count(), 2);
    assert_eq!(snap.dir_count(), 1);
    assert_eq!(snap.total_size(), 11);

    let root_entries = snap.list_dir(&VfsPath::root()).unwrap();
    let names: Vec<&str> = root_entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["docs", "empty.bin"]);

    let note = VfsPath::new("docs/note.txt").unwrap();
    let meta = snap.stat(&note).unwrap();
    assert_eq!(meta.crc32, Some(222_957_957));
    assert!(matches!(
        snap.read_stream(&note, 1024),
        Err(VfsError::UnsupportedFormat(_))
    ));
    assert!(matches!(
        snap.read_stream(&VfsPath::new("docs").unwrap(), 1024),
        Err(VfsError::NotAFile(_))
    ));
    assert!(matches!(
        snap.write_atomic(&note, b"x"),
        Err(VfsError::ReadOnly)
    ));
}

#[test]
fn bcss_corrupted_inputs_are_rejected() {
    assert!(matches!(
        deserialize_bcss(b"this is not json"),
        Err(VfsError::CorruptedArchive(_))
    ));
    assert!(matches!(
        deserialize_bcss(b""),
        Err(VfsError::CorruptedArchive(_))
    ));

    let doc = deserialize_bcss(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("edge-bcss-minimal.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut gz = serialize_bcss_compressed(&doc).unwrap();
    gz.truncate(gz.len() / 2);
    assert!(matches!(
        deserialize_bcss(&gz),
        Err(VfsError::CorruptedArchive(_))
    ));
}

#[test]
fn bcss_diff_type_change_counts_modified() {
    use autocompare_vfs::SnapshotDocument;
    use autocompare_vfs::SnapshotEntry;

    let left = SnapshotDocument::new("left").with_entries(vec![SnapshotEntry {
        path: "node".to_string(),
        is_dir: true,
        size: 0,
        modified_epoch_secs: None,
        crc32: None,
        readonly: false,
    }]);
    let right = SnapshotDocument::new("right").with_entries(vec![SnapshotEntry {
        path: "node".to_string(),
        is_dir: false,
        size: 5,
        modified_epoch_secs: None,
        crc32: Some(1),
        readonly: false,
    }]);

    let summary = left.diff_summary(&right);
    assert_eq!(summary.matching, 0);
    assert_eq!(summary.modified, 1);
    assert_eq!(summary.unique_left, 0);
    assert_eq!(summary.unique_right, 0);

    let detailed = left.detailed_diff(&right);
    assert_eq!(detailed.modified, vec!["node".to_string()]);
    assert_eq!(detailed.summary(), summary);

    let identical = left.diff_summary(&left);
    assert!(identical.is_identical());
    assert!(!identical.has_differences());
}
