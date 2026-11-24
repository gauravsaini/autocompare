//! Integration tests for ZipFs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs::{self, File};
use std::io::{Cursor, Write};

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use autocompare_vfs::{
    LocalFs, Vfs, VfsError, VfsPath, WalkOptions, ZipFs,
    archive_zip::{ArchiveFs, create_zip_archive, create_zip_from_vfs},
};

fn create_sample_zip() -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut writer = ZipWriter::new(Cursor::new(&mut buffer));

    let options = SimpleFileOptions::default();

    writer.add_directory("docs/", options).unwrap();
    writer.start_file("docs/readme.txt", options).unwrap();
    writer.write_all(b"Beyond Compare Zip Replica").unwrap();

    writer.start_file("root.txt", options).unwrap();
    writer.write_all(b"Root file bytes").unwrap();

    let _ = writer.finish().unwrap();
    buffer
}

#[test]
fn test_zip_fs_traversal_and_read() {
    let zip_bytes = create_sample_zip();
    let vfs = ZipFs::from_bytes(zip_bytes).unwrap();

    assert!(vfs.is_read_only());

    // Stat root
    let root_meta = vfs.stat(&VfsPath::root()).unwrap();
    assert!(root_meta.is_dir);

    // List root
    let entries = vfs.list_dir(&VfsPath::root()).unwrap();
    assert_eq!(entries.len(), 2);
    let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["docs", "root.txt"]);

    // List docs/
    let docs_path = VfsPath::new("docs").unwrap();
    let docs_entries = vfs.list_dir(&docs_path).unwrap();
    assert_eq!(docs_entries.len(), 1);
    assert_eq!(docs_entries.first().unwrap().name, "readme.txt");

    // Read file
    let file_path = VfsPath::new("docs/readme.txt").unwrap();
    let bytes = vfs.read_all(&file_path, 1024).unwrap();
    assert_eq!(bytes, b"Beyond Compare Zip Replica");

    // Stat file with CRC32
    let file_meta = vfs.stat(&file_path).unwrap();
    assert_eq!(file_meta.len, 26);
    assert!(file_meta.crc32.is_some());

    // File too large
    let err = vfs.read_all(&file_path, 10).unwrap_err();
    assert!(matches!(err, VfsError::FileTooLarge { .. }));

    // Read only checks
    assert!(matches!(
        vfs.write_atomic(&file_path, b"edit"),
        Err(VfsError::ReadOnly)
    ));
    assert!(matches!(vfs.delete(&file_path), Err(VfsError::ReadOnly)));
}

#[test]
fn test_zip_creation_and_roundtrip() {
    let temp_dir = std::env::temp_dir().join(format!("vfs_zip_rt_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("sub")).unwrap();

    let file_a = temp_dir.join("a.txt");
    let mut f = File::create(&file_a).unwrap();
    f.write_all(b"alpha content").unwrap();

    let file_b = temp_dir.join("sub").join("b.bin");
    let mut f2 = File::create(&file_b).unwrap();
    f2.write_all(b"beta nested content").unwrap();

    let local_vfs = LocalFs::new(&temp_dir).unwrap();
    let zip_bytes =
        create_zip_from_vfs(&local_vfs, &VfsPath::root(), &WalkOptions::unbounded()).unwrap();

    let zip_vfs = ZipFs::from_bytes(zip_bytes).unwrap();
    assert_eq!(zip_vfs.file_count(), 2);
    assert_eq!(zip_vfs.dir_count(), 1);
    assert_eq!(zip_vfs.total_uncompressed_bytes(), 13 + 19);

    // Read files back
    let path_a = VfsPath::new("a.txt").unwrap();
    assert_eq!(zip_vfs.read_all(&path_a, 1024).unwrap(), b"alpha content");

    let path_b = VfsPath::new("sub/b.bin").unwrap();
    assert_eq!(
        zip_vfs.read_all(&path_b, 1024).unwrap(),
        b"beta nested content"
    );

    // CRC32 check
    let meta_a = zip_vfs.stat(&path_a).unwrap();
    assert_eq!(meta_a.crc32, Some(crc32fast::hash(b"alpha content")));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_zip_archive_convenience_and_helpers() {
    let path1 = VfsPath::new("info/about.txt").unwrap();
    let path2 = VfsPath::new("data.csv").unwrap();
    let entries = [
        (&path1, b"Beyond Compare 5 Replica".as_slice()),
        (&path2, b"col1,col2\n1,2".as_slice()),
    ];

    let zip_bytes = create_zip_archive(&entries).unwrap();
    let zip_vfs = ZipFs::from_bytes(zip_bytes).unwrap();

    assert!(zip_vfs.contains(&path1));
    assert!(zip_vfs.contains(&path2));
    assert_eq!(zip_vfs.file_count(), 2);

    // Case-insensitive lookup
    let found = zip_vfs.find_case_insensitive("INFO/ABOUT.TXT");
    assert_eq!(found, Some(path1));

    let all = zip_vfs.all_paths();
    assert!(all.contains(&path2));

    // Raw bytes check
    assert!(!zip_vfs.raw_bytes().is_empty());
}

#[test]
fn test_zip_slip_protection() {
    let mut buffer = Vec::new();
    let mut writer = ZipWriter::new(Cursor::new(&mut buffer));
    let options = SimpleFileOptions::default();

    // Malicious entry with path traversal
    writer.start_file("../../evil.sh", options).unwrap();
    writer.write_all(b"rm -rf /").unwrap();
    writer.start_file("valid.txt", options).unwrap();
    writer.write_all(b"safe").unwrap();
    writer.finish().unwrap();

    let zip_vfs = ZipFs::from_bytes(buffer).unwrap();
    // evil.sh should be skipped due to VfsPath validation
    assert_eq!(zip_vfs.file_count(), 1);
    assert!(zip_vfs.contains(&VfsPath::new("valid.txt").unwrap()));
}

#[test]
fn test_zip_corrupted_archive() {
    let garbage = b"this is not a zip file at all";
    let res = ZipFs::from_bytes(garbage.to_vec());
    assert!(matches!(res, Err(VfsError::CorruptedArchive(_))));
}

#[test]
fn test_archive_fs_zip_polymorphic() {
    let zip_bytes = create_sample_zip();
    let archive = ArchiveFs::from_bytes(&zip_bytes).unwrap();

    assert!(archive.is_zip());
    assert!(!archive.is_tar());
    assert!(archive.is_read_only());
    assert_eq!(archive.file_count(), 2);
    assert_eq!(archive.dir_count(), 1);
    assert!(!archive.is_empty());

    let file_path = VfsPath::new("docs/readme.txt").unwrap();
    assert!(archive.contains(&file_path));

    let content = archive.read_all(&file_path, 1024).unwrap();
    assert_eq!(content, b"Beyond Compare Zip Replica");

    let stat = archive.stat(&file_path).unwrap();
    assert_eq!(stat.len, 26);

    let entries = archive.list_dir(&VfsPath::new("docs").unwrap()).unwrap();
    assert_eq!(entries.len(), 1);

    let found = archive.find_case_insensitive("DOCS/README.TXT");
    assert_eq!(found, Some(file_path));

    assert!(archive.as_zip().is_some());
    assert!(archive.as_tar_gz().is_none());
}

#[test]
fn test_zip_extract_to() {
    let p1 = VfsPath::new("src/main.rs").unwrap();
    let p2 = VfsPath::new("README.md").unwrap();
    let entries = [
        (&p1, b"fn main() {}".as_slice()),
        (&p2, b"# Project".as_slice()),
    ];

    let zip_bytes = create_zip_archive(&entries).unwrap();
    let archive = ArchiveFs::from_bytes(&zip_bytes).unwrap();

    let temp_dir = std::env::temp_dir().join(format!("vfs_zip_extract_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let local_dest = LocalFs::new(&temp_dir).unwrap();
    let count = archive.extract_to(&local_dest, &VfsPath::root()).unwrap();
    assert_eq!(count, 2);

    assert_eq!(local_dest.read_all(&p1, 1024).unwrap(), b"fn main() {}");
    assert_eq!(local_dest.read_all(&p2, 1024).unwrap(), b"# Project");

    let _ = fs::remove_dir_all(&temp_dir);
}
