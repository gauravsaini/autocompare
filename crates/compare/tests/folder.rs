//! Tests for folder-tree comparison. Assertions target folder
//! structure and classification rules; nothing here depends on the
//! diff crate.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use autocompare_compare::{Error, FolderOptions, FolderStatus, NodeInfo, compare_folders};
use common::{cleanup, entry, fixed_time, rel_paths, set_mtime, temp_root, write};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

fn default_opts() -> FolderOptions {
    FolderOptions::default()
}

fn make_pair(base: &Path) -> (PathBuf, PathBuf) {
    (base.join("left"), base.join("right"))
}

#[test]
fn compare_folders_missing_roots_report_missing() {
    let base = temp_root();
    let err = compare_folders(
        &base.join("left-missing"),
        &base.join("right-missing"),
        &default_opts(),
    )
    .expect_err("missing roots must fail");
    assert!(matches!(err, Error::Missing { .. }));
    assert!(err.to_string().contains("left-missing"));
    cleanup(&base);
}

#[test]
fn compare_folders_file_root_reports_missing() {
    let base = temp_root();
    let file_root = base.join("plain.txt");
    write(&file_root, "not a dir");
    let dir = base.join("adir");
    std::fs::create_dir_all(&dir).expect("failed to create the dir fixture");
    let err =
        compare_folders(&file_root, &dir, &default_opts()).expect_err("a file as root must fail");
    assert!(matches!(err, Error::Missing { .. }));
    assert!(err.to_string().contains("plain.txt"));
    cleanup(&base);
}

#[test]
fn compare_folders_same_tree_with_content_compare_is_all_same() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    for name in ["a.txt", "b.txt"] {
        write(&left.join(name), "shared content\n");
        write(&right.join(name), "shared content\n");
    }
    write(&left.join("empty.txt"), "");
    write(&right.join("empty.txt"), "");
    let opts = FolderOptions {
        content_compare: true,
        ..default_opts()
    };
    let result = compare_folders(&left, &right, &opts).expect("same tree must compare");
    assert!(!result.truncated);
    assert_eq!(
        rel_paths(&result.entries),
        vec!["a.txt", "b.txt", "empty.txt"]
    );
    for e in &result.entries {
        assert_eq!(e.status, FolderStatus::Same, "entry {}", e.rel_path);
        assert!(
            e.left.is_some() && e.right.is_some(),
            "entry {}",
            e.rel_path
        );
    }
    cleanup(&base);
}

#[test]
fn compare_folders_same_tree_equal_mtimes_is_all_same() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    let t = fixed_time();
    for name in ["x.txt", "y.txt"] {
        write(&left.join(name), "same bytes\n");
        write(&right.join(name), "same bytes\n");
        set_mtime(&left.join(name), t);
        set_mtime(&right.join(name), t);
    }
    let result = compare_folders(&left, &right, &default_opts()).expect("same tree must compare");
    assert!(!result.truncated);
    assert_eq!(rel_paths(&result.entries), vec!["x.txt", "y.txt"]);
    for e in &result.entries {
        assert_eq!(e.status, FolderStatus::Same, "entry {}", e.rel_path);
    }
    let x = entry(&result.entries, "x.txt");
    assert_eq!(
        x.left,
        Some(NodeInfo {
            size: 11,
            modified: t
        })
    );
    assert_eq!(
        x.right,
        Some(NodeInfo {
            size: 11,
            modified: t
        })
    );
    assert!(!x.is_dir);
    cleanup(&base);
}

#[test]
fn compare_folders_one_sided_file_is_left_or_right_only() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    write(&left.join("only-left.txt"), "L\n");
    write(&right.join("only-right.txt"), "RR\n");
    let result =
        compare_folders(&left, &right, &default_opts()).expect("walkable tree must compare");
    assert_eq!(
        rel_paths(&result.entries),
        vec!["only-left.txt", "only-right.txt"]
    );
    let l = entry(&result.entries, "only-left.txt");
    assert_eq!(l.status, FolderStatus::LeftOnly);
    assert_eq!(l.left.map(|info| info.size), Some(2));
    assert!(l.right.is_none());
    assert!(!l.is_dir);
    let r = entry(&result.entries, "only-right.txt");
    assert_eq!(r.status, FolderStatus::RightOnly);
    assert_eq!(r.right.map(|info| info.size), Some(3));
    assert!(r.left.is_none());
    assert!(!r.is_dir);
    cleanup(&base);
}

#[test]
fn compare_folders_one_sided_directory_is_walked() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    write(&left.join("solo/inner.txt"), "inner\n");
    write(&right.join("keep.txt"), "k\n");
    let result =
        compare_folders(&left, &right, &default_opts()).expect("walkable tree must compare");
    assert_eq!(
        rel_paths(&result.entries),
        vec!["keep.txt", "solo", "solo/inner.txt"]
    );
    let solo = entry(&result.entries, "solo");
    assert_eq!(solo.status, FolderStatus::LeftOnly);
    assert!(solo.is_dir);
    assert_eq!(solo.left.map(|info| info.size), Some(0));
    assert!(solo.right.is_none());
    let inner = entry(&result.entries, "solo/inner.txt");
    assert_eq!(inner.status, FolderStatus::LeftOnly);
    assert!(inner.left.is_some());
    assert!(inner.right.is_none());
    assert!(!inner.is_dir);
    cleanup(&base);
}

#[test]
fn compare_folders_size_differ_is_different() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    write(&left.join("x.txt"), "tiny");
    write(&right.join("x.txt"), "much longer content");
    let result =
        compare_folders(&left, &right, &default_opts()).expect("walkable tree must compare");
    let x = entry(&result.entries, "x.txt");
    assert_eq!(x.status, FolderStatus::Different);
    assert_eq!(x.left.map(|info| info.size), Some(4));
    assert_eq!(x.right.map(|info| info.size), Some(19));
    assert!(!x.is_dir);
    cleanup(&base);
}

#[test]
fn compare_folders_newer_mtime_wins() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    // Equal sizes; the left file is written strictly after the right one.
    write(&right.join("t.txt"), "data\n");
    sleep(Duration::from_millis(20));
    write(&left.join("t.txt"), "data\n");
    let result =
        compare_folders(&left, &right, &default_opts()).expect("walkable tree must compare");
    assert_eq!(
        entry(&result.entries, "t.txt").status,
        FolderStatus::NewerLeft
    );
    // Swapped arguments report the other side as newer.
    let swapped =
        compare_folders(&right, &left, &default_opts()).expect("walkable tree must compare");
    assert_eq!(
        entry(&swapped.entries, "t.txt").status,
        FolderStatus::NewerRight
    );
    cleanup(&base);
}

#[test]
fn compare_folders_content_compare_classifies_by_bytes() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    write(&left.join("c.bin"), "aaaa");
    write(&right.join("c.bin"), "aaab");
    write(&left.join("e.bin"), "same");
    write(&right.join("e.bin"), "same");
    let opts = FolderOptions {
        content_compare: true,
        ..default_opts()
    };
    let result = compare_folders(&left, &right, &opts).expect("walkable tree must compare");
    assert_eq!(
        entry(&result.entries, "c.bin").status,
        FolderStatus::Different
    );
    assert_eq!(entry(&result.entries, "e.bin").status, FolderStatus::Same);
    cleanup(&base);
}

#[test]
fn compare_folders_case_insensitive_names_pair_up() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    write(&left.join("A.txt"), "same\n");
    write(&right.join("a.txt"), "same\n");
    let t = fixed_time();
    set_mtime(&left.join("A.txt"), t);
    set_mtime(&right.join("a.txt"), t);
    let opts = FolderOptions {
        case_insensitive: true,
        ..default_opts()
    };
    let result = compare_folders(&left, &right, &opts).expect("walkable tree must compare");
    assert_eq!(rel_paths(&result.entries), vec!["A.txt"]);
    let a = entry(&result.entries, "A.txt");
    assert_eq!(a.status, FolderStatus::Same);
    assert!(a.left.is_some() && a.right.is_some());
    assert!(!a.is_dir);
    // Without case-insensitive matching both are one-sided.
    let strict =
        compare_folders(&left, &right, &default_opts()).expect("walkable tree must compare");
    assert_eq!(rel_paths(&strict.entries), vec!["A.txt", "a.txt"]);
    assert_eq!(
        entry(&strict.entries, "A.txt").status,
        FolderStatus::LeftOnly
    );
    assert_eq!(
        entry(&strict.entries, "a.txt").status,
        FolderStatus::RightOnly
    );
    cleanup(&base);
}

#[test]
fn compare_folders_include_glob_filters_by_name() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    for (side, content) in [(&left, "a\n"), (&right, "a\n")] {
        write(&side.join("a.txt"), content);
        write(&side.join("b.log"), content);
        write(&side.join("sub/c.txt"), content);
    }
    let opts = FolderOptions {
        include: Some("*.txt".into()),
        ..default_opts()
    };
    let result = compare_folders(&left, &right, &opts).expect("walkable tree must compare");
    // "b.log" and the directory "sub" do not match, so the whole
    // "sub" subtree is pruned even though sub/c.txt matches.
    assert_eq!(rel_paths(&result.entries), vec!["a.txt"]);
    assert!(!result.truncated);
    cleanup(&base);
}

#[test]
fn compare_folders_exclude_glob_prunes_subtrees() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    for side in [&left, &right] {
        write(&side.join("a.txt"), "a\n");
        write(&side.join("b.log"), "b\n");
        write(&side.join("sub/c.txt"), "c\n");
    }
    let opts = FolderOptions {
        exclude: Some("sub".into()),
        ..default_opts()
    };
    let result = compare_folders(&left, &right, &opts).expect("walkable tree must compare");
    assert_eq!(rel_paths(&result.entries), vec!["a.txt", "b.log"]);
    assert!(!result.truncated);

    let opts = FolderOptions {
        exclude: Some("*.log".into()),
        ..default_opts()
    };
    let result = compare_folders(&left, &right, &opts).expect("walkable tree must compare");
    assert_eq!(
        rel_paths(&result.entries),
        vec!["a.txt", "sub", "sub/c.txt"]
    );
    assert!(!result.truncated);
    cleanup(&base);
}

#[test]
fn compare_folders_depth_limit_zero_skips_root_children() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    write(&left.join("a.txt"), "a\n");
    write(&left.join("sub/c.txt"), "c\n");
    write(&right.join("a.txt"), "a\n");
    write(&right.join("sub/c.txt"), "c\n");
    let opts = FolderOptions {
        depth_limit: 0,
        ..default_opts()
    };
    let result = compare_folders(&left, &right, &opts).expect("walkable tree must compare");
    assert!(result.entries.is_empty());
    assert!(result.truncated);

    // Empty roots at the depth cut are not truncated: nothing was
    // actually skipped.
    let empty_left = base.join("empty-left");
    let empty_right = base.join("empty-right");
    std::fs::create_dir_all(&empty_left).expect("failed to create the dir fixture");
    std::fs::create_dir_all(&empty_right).expect("failed to create the dir fixture");
    let empty = compare_folders(&empty_left, &empty_right, &opts).expect("empty dirs must compare");
    assert!(empty.entries.is_empty());
    assert!(!empty.truncated);
    cleanup(&base);
}

#[test]
fn compare_folders_depth_limit_one_skips_grandchildren() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    write(&left.join("a.txt"), "a\n");
    write(&left.join("sub/c.txt"), "c\n");
    write(&right.join("a.txt"), "a\n");
    write(&right.join("sub/c.txt"), "c\n");
    let opts = FolderOptions {
        depth_limit: 1,
        ..default_opts()
    };
    let result = compare_folders(&left, &right, &opts).expect("walkable tree must compare");
    assert_eq!(rel_paths(&result.entries), vec!["a.txt", "sub"]);
    assert!(result.truncated);

    // A deeper limit walks everything.
    let opts = FolderOptions {
        depth_limit: 2,
        ..default_opts()
    };
    let full = compare_folders(&left, &right, &opts).expect("walkable tree must compare");
    assert_eq!(rel_paths(&full.entries), vec!["a.txt", "sub", "sub/c.txt"]);
    assert!(!full.truncated);
    cleanup(&base);
}

#[test]
fn compare_folders_mixed_kinds_are_different() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    write(&left.join("x"), "file\n");
    write(&right.join("x/inner.txt"), "dir contents\n");
    let result =
        compare_folders(&left, &right, &default_opts()).expect("walkable tree must compare");
    // The right-side directory is not walked because its pair is a file.
    assert_eq!(rel_paths(&result.entries), vec!["x"]);
    let x = entry(&result.entries, "x");
    assert_eq!(x.status, FolderStatus::Different);
    assert!(x.left.is_some() && x.right.is_some());
    assert!(!x.is_dir);
    cleanup(&base);
}

#[test]
fn compare_folders_entries_are_sorted_by_rel_path() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    for side in [&left, &right] {
        write(&side.join("z.txt"), "z\n");
        write(&side.join("a.txt"), "a\n");
        write(&side.join("m.txt"), "m\n");
        write(&side.join("sub/nested.txt"), "n\n");
    }
    let result =
        compare_folders(&left, &right, &default_opts()).expect("walkable tree must compare");
    assert_eq!(
        rel_paths(&result.entries),
        vec!["a.txt", "m.txt", "sub", "sub/nested.txt", "z.txt"]
    );
    assert!(entry(&result.entries, "sub").is_dir);
    cleanup(&base);
}

#[test]
#[cfg(unix)]
fn compare_folders_does_not_follow_symlinks() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    write(&left.join("real/inner.txt"), "inner\n");
    write(&right.join("keep.txt"), "k\n");
    // A directory and a file outside the compared trees.
    let outside = base.join("outside");
    std::fs::create_dir_all(&outside).expect("failed to create the dir fixture");
    write(&outside.join("secret.txt"), "secret\n");
    std::os::unix::fs::symlink(&outside, left.join("outdir"))
        .expect("failed to create the dir symlink");
    std::os::unix::fs::symlink(outside.join("secret.txt"), left.join("outlink"))
        .expect("failed to create the file symlink");

    let result = compare_folders(&left, &right, &default_opts()).expect("walk must complete");
    let rels = rel_paths(&result.entries);
    assert!(rels.contains(&"outlink".to_string()));
    assert!(rels.contains(&"outdir".to_string()));
    for rel in &rels {
        assert!(!rel.starts_with("outdir/"), "symlink must not be traversed");
        assert!(!rel.contains("secret"), "outside content must not appear");
    }
    let outdir = entry(&result.entries, "outdir");
    assert_eq!(outdir.status, FolderStatus::LeftOnly);
    assert!(!outdir.is_dir);
    let outlink = entry(&result.entries, "outlink");
    assert_eq!(outlink.status, FolderStatus::LeftOnly);
    assert!(!outlink.is_dir);
    cleanup(&base);
}

#[test]
#[cfg(unix)]
fn compare_folders_dir_symlink_loop_terminates() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    write(&left.join("leaf.txt"), "leaf\n");
    write(&right.join("keep.txt"), "k\n");
    // A symlink pointing at its own parent directory.
    std::os::unix::fs::symlink(&left, left.join("selfloop"))
        .expect("failed to create the loop symlink");

    let result = compare_folders(&left, &right, &default_opts()).expect("walk must terminate");
    let rels = rel_paths(&result.entries);
    assert_eq!(rels, vec!["keep.txt", "leaf.txt", "selfloop"]);
    let selfloop = entry(&result.entries, "selfloop");
    assert!(!selfloop.is_dir);
    assert_eq!(selfloop.status, FolderStatus::LeftOnly);
    cleanup(&base);
}
