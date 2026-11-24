//! Vfs-backed folder orchestration: LocalFs parity with the path walk,
//! Zip and Snapshot trees, Sync Both/Mirror dry-run/apply, Folder Merge
//! and the cancellable VFS JobRunner job.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use autocompare_compare::{
    Error, FolderCompare, FolderMerge, FolderOptions, FolderStatus, JobRunner, SyncMode,
    SyncPlanner, compare_folders, compare_vfs_folders, compare_vfs_folders_cancellable,
};
use autocompare_vfs::{
    LocalFs, SnapshotFs, Vfs, VfsPath, ZipFs, create_snapshot, create_zip_archive,
};
use common::{cleanup, set_mtime, temp_root, write};

fn content_opts() -> FolderOptions {
    FolderOptions {
        content_compare: true,
        ..FolderOptions::default()
    }
}

fn masks_opts(masks: Option<&str>) -> FolderOptions {
    FolderOptions {
        masks: masks.map(|s| s.to_string()),
        ..content_opts()
    }
}

fn statuses_of(entries: &[autocompare_compare::FolderEntry]) -> HashMap<String, FolderStatus> {
    entries
        .iter()
        .map(|e| (e.rel_path.clone(), e.status))
        .collect()
}

#[test]
fn vfs_localfs_matches_path_walk() {
    let base = temp_root();
    let left = base.join("left");
    let right = base.join("right");
    write(&left.join("same.txt"), "same\n");
    write(&right.join("same.txt"), "same\n");
    write(&left.join("diff.txt"), "left side\n");
    write(&right.join("diff.txt"), "right side\n");
    write(&left.join("sub/only.txt"), "left only\n");

    let opts = content_opts();
    let path_cmp = compare_folders(&left, &right, &opts).expect("path walk compares");
    let lfs = LocalFs::new(&left).expect("left local fs");
    let rfs = LocalFs::new(&right).expect("right local fs");
    let vfs_cmp = compare_vfs_folders(&lfs, &rfs, &opts).expect("vfs walk compares");
    assert!(!vfs_cmp.truncated);
    assert_eq!(
        statuses_of(&vfs_cmp.entries),
        statuses_of(&path_cmp.entries)
    );

    let engine = FolderCompare::new(&left, &right, opts).expect("engine builds");
    let via_engine = engine
        .run_over_vfs(&lfs, &rfs)
        .expect("engine vfs run compares");
    assert_eq!(
        statuses_of(&via_engine.entries),
        statuses_of(&path_cmp.entries)
    );
    cleanup(&base);
}

#[test]
fn vfs_zip_trees_compare_against_each_other_and_local() {
    let a_path = VfsPath::new("same.txt").unwrap();
    let b_path = VfsPath::new("changed.txt").unwrap();
    let left_bytes = create_zip_archive(&[
        (&a_path, b"same\n".as_slice()),
        (&b_path, b"left bytes\n".as_slice()),
    ])
    .unwrap();
    let right_bytes = create_zip_archive(&[
        (&a_path, b"same\n".as_slice()),
        (&b_path, b"right bytes\n".as_slice()),
    ])
    .unwrap();
    let left_zip = ZipFs::from_bytes(left_bytes).unwrap();
    let right_zip = ZipFs::from_bytes(right_bytes).unwrap();

    let opts = content_opts();
    let cmp = compare_vfs_folders(&left_zip, &right_zip, &opts).expect("zip trees compare");
    assert!(!cmp.truncated);
    let got = statuses_of(&cmp.entries);
    assert_eq!(got.get("same.txt"), Some(&FolderStatus::Same));
    assert_eq!(got.get("changed.txt"), Some(&FolderStatus::Different));

    let self_cmp = compare_vfs_folders(&left_zip, &left_zip, &opts).expect("self compare");
    assert!(
        self_cmp
            .entries
            .iter()
            .all(|e| e.status == FolderStatus::Same)
    );

    // Zip against a LocalFs tree with identical bytes.
    let base = temp_root();
    let dir = base.join("plain");
    write(&dir.join("same.txt"), "same\n");
    write(&dir.join("changed.txt"), "left bytes\n");
    let lfs = LocalFs::new(&dir).expect("local fs");
    let mixed = compare_vfs_folders(&lfs, &left_zip, &opts).expect("local-vs-zip compares");
    let mixed_got = statuses_of(&mixed.entries);
    assert_eq!(mixed_got.get("same.txt"), Some(&FolderStatus::Same));
    assert_eq!(mixed_got.get("changed.txt"), Some(&FolderStatus::Same));
    cleanup(&base);
}

#[test]
fn vfs_snapshot_matches_the_captured_tree() {
    let base = temp_root();
    let dir = base.join("plain");
    write(&dir.join("keep.txt"), "keep\n");
    write(&dir.join("edit.txt"), "before\n");
    // Whole-second mtimes: snapshots keep epoch seconds only, so the
    // metadata compare below is exact instead of sub-second noise.
    let then = common::fixed_time();
    set_mtime(&dir.join("keep.txt"), then);
    set_mtime(&dir.join("edit.txt"), then);
    let lfs = LocalFs::new(&dir).expect("local fs");

    let doc = create_snapshot(&lfs, &VfsPath::root(), "snap", None).expect("snapshot captures");
    let snap = SnapshotFs::new(doc).expect("snapshot fs loads");
    // Snapshots store metadata only, so this compare follows the
    // size/mtime rules instead of byte content.
    let opts = FolderOptions::default();
    let cmp = compare_vfs_folders(&snap, &lfs, &opts).expect("snapshot compares");
    assert!(!cmp.truncated);
    assert!(cmp.entries.iter().all(|e| e.status == FolderStatus::Same));

    write(&dir.join("edit.txt"), "afterwards, longer\n");
    let lfs2 = LocalFs::new(&dir).expect("local fs again");
    let cmp2 = compare_vfs_folders(&snap, &lfs2, &opts).expect("edited tree compares");
    let got = statuses_of(&cmp2.entries);
    assert_eq!(got.get("keep.txt"), Some(&FolderStatus::Same));
    assert_eq!(got.get("edit.txt"), Some(&FolderStatus::Different));
    cleanup(&base);
}

#[test]
fn sync_update_both_copies_newer_each_way() {
    let base = temp_root();
    let left = base.join("left");
    let right = base.join("right");
    // Same sizes, different bytes; mtimes decide the winner.
    write(&left.join("a.txt"), "aaa\n");
    write(&right.join("a.txt"), "bbb\n");
    write(&left.join("b.txt"), "ccc\n");
    write(&right.join("b.txt"), "ddd\n");
    let old = common::fixed_time();
    let new = old + std::time::Duration::from_secs(60);
    set_mtime(&left.join("a.txt"), new);
    set_mtime(&right.join("a.txt"), old);
    set_mtime(&left.join("b.txt"), old);
    set_mtime(&right.join("b.txt"), new);

    let planner = SyncPlanner::new(&left, &right, SyncMode::Both);
    let plan = planner.dry_run().expect("both dry run plans");
    assert!(right.join("a.txt").exists());
    assert_eq!(plan.copy_to_right_count(), 1);
    assert_eq!(plan.copy_to_left_count(), 1);
    let report = planner.apply(&plan).expect("both apply runs");
    assert!(report.failed.is_empty());
    assert_eq!(
        std::fs::read_to_string(left.join("b.txt")).unwrap(),
        "ddd\n"
    );
    assert_eq!(
        std::fs::read_to_string(right.join("a.txt")).unwrap(),
        "aaa\n"
    );
    cleanup(&base);
}

#[test]
fn sync_mirror_dry_run_prunes_then_applies() {
    let base = temp_root();
    let left = base.join("left");
    let right = base.join("right");
    write(&left.join("keep.txt"), "keep\n");
    write(&left.join("new.txt"), "new\n");
    write(&right.join("keep.txt"), "keep\n");
    write(&right.join("stale.txt"), "stale\n");

    let planner = SyncPlanner::new(&left, &right, SyncMode::Mirror);
    let plan = planner.dry_run().expect("mirror dry run plans");
    assert!(!right.join("new.txt").exists());
    assert!(right.join("stale.txt").exists());
    assert!(plan.copy_to_right_count() >= 1);
    assert!(plan.delete_right_count() >= 1);
    let report = planner.apply(&plan).expect("mirror apply runs");
    assert!(report.failed.is_empty());
    assert!(right.join("new.txt").exists());
    assert!(!right.join("stale.txt").exists());
    cleanup(&base);
}

#[test]
fn folder_merge_clean_sides_apply_to_output() {
    let base = temp_root();
    let b = base.join("base");
    let l = base.join("left");
    let r = base.join("right");
    let out = base.join("out");
    write(&b.join("f.txt"), "one\n");
    write(&l.join("f.txt"), "one changed by left\n");
    write(&r.join("f.txt"), "one\n");
    write(&r.join("g.txt"), "right adds\n");

    let merge = FolderMerge::new(&b, &l, &r);
    let plan = merge.plan().expect("merge plans");
    assert!(!plan.has_conflicts());
    let report = merge.apply(&plan, &out).expect("merge applies");
    assert!(report.failed.is_empty());
    assert_eq!(
        std::fs::read_to_string(out.join("f.txt")).unwrap(),
        "one changed by left\n"
    );
    assert_eq!(
        std::fs::read_to_string(out.join("g.txt")).unwrap(),
        "right adds\n"
    );
    cleanup(&base);
}

#[test]
fn vfs_masks_match_path_walk_include_and_exclude() {
    let base = temp_root();
    let left = base.join("left");
    let right = base.join("right");
    for side in [&left, &right] {
        write(&side.join("a.ts"), "ts\n");
        write(&side.join("b.js"), "js\n");
        write(&side.join("sub/c.ts"), "cts\n");
        write(&side.join("sub/d.js"), "djs\n");
        write(&side.join("app.min.js"), "min\n");
    }
    // One differing file proves verdicts survive masking.
    write(&right.join("sub/c.ts"), "cts changed\n");

    for masks in [
        Some("*.ts;*.tsx"),
        Some("-*.min.js"),
        Some("*.js;-*.min.js"),
        None,
        Some(""),
        Some("  ; "),
    ] {
        let opts = masks_opts(masks);
        let path_cmp = compare_folders(&left, &right, &opts).expect("path walk compares");
        let lfs = LocalFs::new(&left).expect("left local fs");
        let rfs = LocalFs::new(&right).expect("right local fs");
        let vfs_cmp = compare_vfs_folders(&lfs, &rfs, &opts).expect("vfs walk compares");
        assert_eq!(
            statuses_of(&vfs_cmp.entries),
            statuses_of(&path_cmp.entries),
            "masks {masks:?} must keep vfs/path parity"
        );
    }

    // Spot-check include-only shape: files drop out, dirs stay walkable.
    let opts = masks_opts(Some("*.ts"));
    let lfs = LocalFs::new(&left).expect("left local fs");
    let rfs = LocalFs::new(&right).expect("right local fs");
    let cmp = compare_vfs_folders(&lfs, &rfs, &opts).expect("masked vfs compares");
    let paths: Vec<&str> = cmp.entries.iter().map(|e| e.rel_path.as_str()).collect();
    assert!(paths.contains(&"a.ts"), "include keeps a.ts: {paths:?}");
    assert!(paths.contains(&"sub"), "dirs always pass: {paths:?}");
    assert!(paths.contains(&"sub/c.ts"), "nested match walks: {paths:?}");
    assert!(!paths.contains(&"b.js"), "non-match drops: {paths:?}");
    assert!(
        !paths.contains(&"sub/d.js"),
        "nested non-match drops: {paths:?}"
    );
    assert!(
        !paths.contains(&"app.min.js"),
        "unmatched root file drops: {paths:?}"
    );
    cleanup(&base);
}

#[test]
fn vfs_masks_filter_zip_trees() {
    let a_path = VfsPath::new("same.txt").unwrap();
    let b_path = VfsPath::new("changed.txt").unwrap();
    let mk = || {
        let left_bytes = create_zip_archive(&[
            (&a_path, b"same\n".as_slice()),
            (&b_path, b"left bytes\n".as_slice()),
        ])
        .unwrap();
        let right_bytes = create_zip_archive(&[
            (&a_path, b"same\n".as_slice()),
            (&b_path, b"right bytes\n".as_slice()),
        ])
        .unwrap();
        (
            ZipFs::from_bytes(left_bytes).unwrap(),
            ZipFs::from_bytes(right_bytes).unwrap(),
        )
    };

    let (left_zip, right_zip) = mk();
    let full =
        compare_vfs_folders(&left_zip, &right_zip, &content_opts()).expect("unmasked zip compares");
    assert_eq!(full.entries.len(), 2);

    let (left_zip, right_zip) = mk();
    let opts = masks_opts(Some("*.txt;-changed.*"));
    let cmp = compare_vfs_folders(&left_zip, &right_zip, &opts).expect("masked zip compares");
    assert_eq!(cmp.entries.len(), 1);
    let only = cmp.entries.first().expect("one masked entry");
    assert_eq!(only.rel_path, "same.txt");
    assert_eq!(only.status, FolderStatus::Same);
}

#[test]
fn vfs_masks_flow_through_spawn_vfs_job() {
    let base = temp_root();
    let left = base.join("left");
    let right = base.join("right");
    write(&left.join("a.ts"), "ts\n");
    write(&right.join("a.ts"), "ts\n");
    write(&left.join("b.js"), "js\n");
    write(&right.join("b.js"), "js\n");
    let lfs: Arc<dyn Vfs> = Arc::new(LocalFs::new(&left).expect("left local fs"));
    let rfs: Arc<dyn Vfs> = Arc::new(LocalFs::new(&right).expect("right local fs"));

    let handle = JobRunner::spawn_vfs_folder_compare(lfs, rfs, masks_opts(Some("*.ts")));
    let compared = handle.join().expect("vfs job completes");
    let paths: Vec<&str> = compared
        .entries
        .iter()
        .map(|e| e.rel_path.as_str())
        .collect();
    assert_eq!(paths, vec!["a.ts"], "job carries masks: {paths:?}");
    cleanup(&base);
}

#[test]
fn vfs_jobrunner_completes_and_precancelled_fails() {
    let base = temp_root();
    let left = base.join("left");
    let right = base.join("right");
    write(&left.join("a.txt"), "a\n");
    write(&right.join("a.txt"), "a\n");
    let lfs: Arc<dyn Vfs> = Arc::new(LocalFs::new(&left).expect("left local fs"));
    let rfs: Arc<dyn Vfs> = Arc::new(LocalFs::new(&right).expect("right local fs"));

    let handle = JobRunner::spawn_vfs_folder_compare(lfs, rfs, content_opts());
    let compared = handle.join().expect("vfs job completes");
    assert!(
        compared
            .entries
            .iter()
            .all(|e| e.status == FolderStatus::Same)
    );

    let lfs2 = LocalFs::new(&left).expect("left local fs again");
    let rfs2 = LocalFs::new(&right).expect("right local fs again");
    let cancelled = AtomicBool::new(true);
    let err = compare_vfs_folders_cancellable(&lfs2, &rfs2, &content_opts(), Some(&cancelled))
        .expect_err("precancelled walk must fail");
    assert!(matches!(err, Error::Cancelled));
    assert!(cancelled.load(Ordering::Relaxed));
    cleanup(&base);
}
