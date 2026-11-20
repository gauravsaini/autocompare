//! Integration tests for TarGzFs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs::{self, File};
use std::io::Write;

use flate2::Compression;
use flate2::write::GzEncoder;
use tar::{Builder, Header};

use autocompare_vfs::{
    LocalFs, TarGzFs, Vfs, VfsError, VfsPath, WalkOptions,
    archive_tar::{
        ArchiveFs, create_tar_archive, create_tar_from_vfs, create_tar_gz_archive,
        create_tar_gz_from_vfs,
    },
};

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

    // Check pre-computed CRC32 on tar entry
    let meta = vfs.stat(&file_path).unwrap();
    assert_eq!(meta.crc32, Some(crc32fast::hash(b"Hello from Tar")));
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
    assert_eq!(meta.crc32, Some(crc32fast::hash(b"Hello from Tar")));

    // Read only enforcement
    assert!(matches!(
        vfs.write_atomic(&file_path, b"test"),
        Err(VfsError::ReadOnly)
    ));
    assert!(matches!(vfs.delete(&file_path), Err(VfsError::ReadOnly)));
}

#[test]
fn test_tar_vfs_pack_and_roundtrip() {
    let temp_dir = std::env::temp_dir().join(format!("vfs_tar_rt_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("nested")).unwrap();

    let f1 = temp_dir.join("root_file.txt");
    let mut file1 = File::create(&f1).unwrap();
    file1.write_all(b"root content").unwrap();

    let f2 = temp_dir.join("nested").join("child.dat");
    let mut file2 = File::create(&f2).unwrap();
    file2.write_all(b"child binary payload").unwrap();

    let local_vfs = LocalFs::new(&temp_dir).unwrap();

    // 1. Plain Tar roundtrip
    let tar_bytes =
        create_tar_from_vfs(&local_vfs, &VfsPath::root(), &WalkOptions::unbounded()).unwrap();
    let tar_vfs = TarGzFs::from_bytes(&tar_bytes).unwrap();
    assert_eq!(tar_vfs.file_count(), 2);
    assert_eq!(tar_vfs.dir_count(), 1);
    assert_eq!(tar_vfs.total_bytes(), 12 + 20);

    let path1 = VfsPath::new("root_file.txt").unwrap();
    assert_eq!(tar_vfs.read_all(&path1, 1024).unwrap(), b"root content");

    let path2 = VfsPath::new("nested/child.dat").unwrap();
    assert_eq!(
        tar_vfs.read_all(&path2, 1024).unwrap(),
        b"child binary payload"
    );

    // 2. Tar.gz roundtrip
    let tar_gz_bytes =
        create_tar_gz_from_vfs(&local_vfs, &VfsPath::root(), &WalkOptions::unbounded()).unwrap();
    let tar_gz_vfs = TarGzFs::from_bytes(&tar_gz_bytes).unwrap();
    assert_eq!(tar_gz_vfs.file_count(), 2);
    assert_eq!(tar_gz_vfs.read_all(&path1, 1024).unwrap(), b"root content");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_tar_convenience_and_helpers() {
    let p1 = VfsPath::new("config/app.toml").unwrap();
    let p2 = VfsPath::new("logo.svg").unwrap();
    let entries = [
        (&p1, b"key = 'value'".as_slice()),
        (&p2, b"<svg></svg>".as_slice()),
    ];

    let tar_bytes = create_tar_archive(&entries).unwrap();
    let vfs = TarGzFs::from_bytes(&tar_bytes).unwrap();

    assert!(vfs.contains(&p1));
    assert!(vfs.contains(&p2));
    assert_eq!(vfs.file_count(), 2);

    // Case-insensitive lookup
    assert_eq!(
        vfs.find_case_insensitive("CONFIG/APP.TOML"),
        Some(p1.clone())
    );

    // Tar.gz convenience
    let gz_bytes = create_tar_gz_archive(&entries).unwrap();
    let gz_vfs = TarGzFs::from_bytes(&gz_bytes).unwrap();
    assert_eq!(gz_vfs.file_count(), 2);
}

#[test]
fn test_tar_corrupted_rejection() {
    let garbage = b"\x1F\x8B\x08\x00corrupted gzip header";
    let res = TarGzFs::from_bytes(garbage);
    assert!(matches!(res, Err(VfsError::CorruptedArchive(_))));
}

#[test]
fn test_archive_fs_tar_polymorphic() {
    let tar_bytes = create_sample_tar();
    let archive = ArchiveFs::from_bytes(&tar_bytes).unwrap();

    assert!(!archive.is_zip());
    assert!(archive.is_tar());
    assert!(archive.is_read_only());
    assert_eq!(archive.file_count(), 1);
    assert_eq!(archive.dir_count(), 1);
    assert!(!archive.is_empty());

    let file_path = VfsPath::new("archive/hello.txt").unwrap();
    assert!(archive.contains(&file_path));

    let content = archive.read_all(&file_path, 1024).unwrap();
    assert_eq!(content, b"Hello from Tar");

    let entries = archive.list_dir(&VfsPath::new("archive").unwrap()).unwrap();
    assert_eq!(entries.len(), 1);

    // from_tar_bytes & from_tar_gz_bytes
    let from_tar = TarGzFs::from_tar_bytes(&tar_bytes).unwrap();
    assert_eq!(from_tar.file_count(), 1);

    let gz_bytes = create_sample_tar_gz();
    let from_gz = TarGzFs::from_tar_gz_bytes(&gz_bytes).unwrap();
    assert_eq!(from_gz.file_count(), 1);

    assert!(archive.as_tar_gz().is_some());
    assert!(archive.as_zip().is_none());
}
