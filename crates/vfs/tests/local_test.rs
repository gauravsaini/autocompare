//! Integration tests for LocalFs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::symlink;

use autocompare_vfs::{LocalFs, Vfs, VfsError, VfsPath, WalkOptions};

#[test]
fn test_local_fs_basic_operations() {
    let temp_dir = std::env::temp_dir().join(format!("vfs_test_basic_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("sub")).unwrap();

    let file_path = temp_dir.join("hello.txt");
    let mut f = File::create(&file_path).unwrap();
    f.write_all(b"Hello LocalFS").unwrap();

    let sub_file = temp_dir.join("sub").join("nested.txt");
    let mut f2 = File::create(&sub_file).unwrap();
    f2.write_all(b"Nested content").unwrap();

    let vfs = LocalFs::new(&temp_dir).unwrap();

    // Stat root
    let root_meta = vfs.stat(&VfsPath::root()).unwrap();
    assert!(root_meta.is_dir);

    // List root dir
    let entries = vfs.list_dir(&VfsPath::root()).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries.first().unwrap().name, "hello.txt");
    assert_eq!(entries.get(1).unwrap().name, "sub");

    // Read file
    let path = VfsPath::new("hello.txt").unwrap();
    let bytes = vfs.read_all(&path, 1024).unwrap();
    assert_eq!(bytes, b"Hello LocalFS");

    // File too large error
    let err = vfs.read_all(&path, 5).unwrap_err();
    assert!(matches!(
        err,
        VfsError::FileTooLarge {
            max_bytes: 5,
            actual: 13,
            ..
        }
    ));

    // Atomic write
    let new_path = VfsPath::new("sub/new_file.txt").unwrap();
    vfs.write_atomic(&new_path, b"atomic content").unwrap();
    assert_eq!(vfs.read_all(&new_path, 1024).unwrap(), b"atomic content");

    // Delete
    vfs.delete(&new_path).unwrap();
    assert!(!vfs.exists(&new_path));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_local_fs_symlink_cycle_refusal() {
    let temp_dir = std::env::temp_dir().join(format!("vfs_test_symlink_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    let dir_a = temp_dir.join("dir_a");
    let dir_b = dir_a.join("dir_b");
    fs::create_dir_all(&dir_b).unwrap();

    // Create a symlink cycle: dir_b/cycle_to_a -> dir_a
    let cycle_link = dir_b.join("cycle_to_a");
    symlink(&dir_a, &cycle_link).unwrap();

    let vfs = LocalFs::new(&temp_dir).unwrap();

    // 1. Without following symlinks (default), walk should succeed and NOT loop
    let options_no_follow = WalkOptions {
        follow_symlinks: false,
        max_depth: None,
        refuse_cycles: true,
    };
    let entries = vfs.walk(&VfsPath::root(), &options_no_follow).unwrap();
    let sym_entry = entries
        .iter()
        .find(|e| e.name == "cycle_to_a")
        .expect("should find symlink entry");
    assert!(sym_entry.meta.is_symlink);
    assert!(!sym_entry.meta.is_dir);

    // 2. With follow_symlinks = true and refuse_cycles = true, must detect and refuse cycle!
    let options_follow_refuse = WalkOptions {
        follow_symlinks: true,
        max_depth: None,
        refuse_cycles: true,
    };
    let err = vfs
        .walk(&VfsPath::root(), &options_follow_refuse)
        .unwrap_err();
    assert!(matches!(err, VfsError::SymlinkCycle(_)));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_local_fs_read_only_mode() {
    let temp_dir = std::env::temp_dir().join(format!("vfs_test_ro_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let vfs = LocalFs::new(&temp_dir).unwrap().with_readonly(true);
    assert!(vfs.is_read_only());

    let path = VfsPath::new("dummy.txt").unwrap();
    assert!(matches!(
        vfs.write_atomic(&path, b"test"),
        Err(VfsError::ReadOnly)
    ));
    assert!(matches!(vfs.delete(&path), Err(VfsError::ReadOnly)));

    let _ = fs::remove_dir_all(&temp_dir);
}
