//! Integration tests for TarGzFs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;

use flate2::Compression;
use flate2::write::GzEncoder;
use tar::{Builder, Header};

use autocompare_vfs::{TarGzFs, Vfs, VfsError, VfsPath};

fn create_sample_tar() -> Vec<u8> {
    let mut builder = Builder::new(Vec::new());

    let mut header = Header::new_gnu();
    header.set_size(14);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(
            &mut header,
            "archive/hello.txt",
            b"Hello from Tar".as_slice(),
        )
        .unwrap();

    let mut dir_header = Header::new_gnu();
    dir_header.set_entry_type(tar::EntryType::Directory);
    dir_header.set_size(0);
    dir_header.set_mode(0o755);
    dir_header.set_cksum();
    builder
        .append_data(&mut dir_header, "archive/", [].as_slice())
        .unwrap();

    builder.into_inner().unwrap()
}

fn create_sample_tar_gz() -> Vec<u8> {
    let tar_bytes = create_sample_tar();
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&tar_bytes).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn test_tar_plain_reading() {
    let tar_bytes = create_sample_tar();
    let vfs = TarGzFs::from_bytes(&tar_bytes).unwrap();

    assert!(vfs.is_read_only());

    let entries = vfs.list_dir(&VfsPath::root()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries.first().unwrap().name, "archive");

    let file_path = VfsPath::new("archive/hello.txt").unwrap();
    let data = vfs.read_all(&file_path, 1024).unwrap();
    assert_eq!(data, b"Hello from Tar");
}

#[test]
fn test_tar_gz_reading() {
    let gz_bytes = create_sample_tar_gz();
    let vfs = TarGzFs::from_bytes(&gz_bytes).unwrap();

    assert!(vfs.is_read_only());

    let file_path = VfsPath::new("archive/hello.txt").unwrap();
    let data = vfs.read_all(&file_path, 1024).unwrap();
    assert_eq!(data, b"Hello from Tar");

    // Stat verification
    let meta = vfs.stat(&file_path).unwrap();
    assert_eq!(meta.len, 14);
    assert!(!meta.is_dir);

    // Read only enforcement
    assert!(matches!(
        vfs.write_atomic(&file_path, b"test"),
        Err(VfsError::ReadOnly)
    ));
    assert!(matches!(vfs.delete(&file_path), Err(VfsError::ReadOnly)));
}
