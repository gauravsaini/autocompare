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
    let options_no_follow = WalkOptions::standard();
    let entries = vfs.walk(&VfsPath::root(), &options_no_follow).unwrap();
    let sym_entry = entries
        .iter()
        .find(|e| e.name == "cycle_to_a")
        .expect("should find symlink entry");
    assert!(sym_entry.meta.is_symlink);
    assert!(!sym_entry.meta.is_dir);

    // 2. With follow_symlinks = true and refuse_cycles = true, must detect and refuse cycle!
    let options_follow_refuse = WalkOptions::standard().with_follow_symlinks(true);
    let err = vfs
        .walk(&VfsPath::root(), &options_follow_refuse)
        .unwrap_err();
    assert!(matches!(err, VfsError::SymlinkCycle(_)));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_local_fs_traversal_escaping_refusal() {
    let base_temp = std::env::temp_dir();
    let root_dir = base_temp.join(format!("vfs_test_escape_root_{}", std::process::id()));
    let outside_dir = base_temp.join(format!("vfs_test_escape_outside_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root_dir);
    let _ = fs::remove_dir_all(&outside_dir);

    fs::create_dir_all(&root_dir).unwrap();
    fs::create_dir_all(&outside_dir).unwrap();

    let outside_file = outside_dir.join("secret.txt");
    fs::write(&outside_file, b"outside secret content").unwrap();

    // 1. Create a symlink to outside file
    let file_symlink = root_dir.join("link_to_secret");
    symlink(&outside_file, &file_symlink).unwrap();

    // 2. Create a symlink to outside directory
    let dir_symlink = root_dir.join("link_to_outside_dir");
    symlink(&outside_dir, &dir_symlink).unwrap();

    let vfs = LocalFs::new(&root_dir).unwrap();
    assert!(vfs.canonical_root().is_some());

    // Stat the symlink itself succeeds and reports is_symlink
    let file_sym_path = VfsPath::new("link_to_secret").unwrap();
    let meta = vfs.stat(&file_sym_path).unwrap();
    assert!(meta.is_symlink);

    // Reading stream through the escaping symlink MUST be refused!
    assert!(matches!(
        vfs.read_stream(&file_sym_path, 1024),
        Err(VfsError::TraversalEscape(_))
    ));

    // Listing directory through the escaping symlink MUST be refused!
    let dir_sym_path = VfsPath::new("link_to_outside_dir").unwrap();
    let list_err = vfs.list_dir(&dir_sym_path).unwrap_err();
    assert!(matches!(list_err, VfsError::TraversalEscape(_)));

    // Writing atomic through the escaping directory symlink MUST be refused!
    let write_target = VfsPath::new("link_to_outside_dir/injected.txt").unwrap();
    let write_err = vfs.write_atomic(&write_target, b"evil").unwrap_err();
    assert!(matches!(write_err, VfsError::TraversalEscape(_)));

    // Stat on a path under the escaping directory symlink MUST be refused!
    let stat_err = vfs.stat(&write_target).unwrap_err();
    assert!(matches!(stat_err, VfsError::TraversalEscape(_)));

    // Walking with follow_symlinks: true and refuse_traversal_escape: true MUST refuse!
    let walk_opt = WalkOptions::standard().with_follow_symlinks(true);
    let walk_err = vfs.walk(&VfsPath::root(), &walk_opt).unwrap_err();
    assert!(matches!(walk_err, VfsError::TraversalEscape(_)));

    let _ = fs::remove_dir_all(&root_dir);
    let _ = fs::remove_dir_all(&outside_dir);
}

#[test]
fn test_local_fs_read_only_mode() {
    let temp_dir = std::env::temp_dir().join(format!("vfs_test_ro_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let mut vfs = LocalFs::new(&temp_dir).unwrap().with_readonly(true);
    assert!(vfs.is_read_only());

    let path = VfsPath::new("dummy.txt").unwrap();
    assert!(matches!(
        vfs.write_atomic(&path, b"test"),
        Err(VfsError::ReadOnly)
    ));
    assert!(matches!(vfs.delete(&path), Err(VfsError::ReadOnly)));

    // Mutate read-only mode in place
    vfs.set_readonly(false);
    assert!(!vfs.is_read_only());
    vfs.write_atomic(&path, b"writable now").unwrap();
    assert_eq!(vfs.read_all(&path, 1024).unwrap(), b"writable now");

    vfs.set_readonly(true);
    assert!(vfs.is_read_only());
    assert!(matches!(vfs.delete(&path), Err(VfsError::ReadOnly)));

    let _ = fs::remove_dir_all(&temp_dir);
}
