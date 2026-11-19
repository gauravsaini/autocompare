//! Integration tests for ZipFs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Write};

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use autocompare_vfs::{Vfs, VfsError, VfsPath, ZipFs};

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
