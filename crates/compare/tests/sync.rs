//! Integration tests for folder sync: plan building and applying.
//!
//! Fixtures pin mtimes so statuses are deterministic without sleeps.
//! Applying runs against real trees in unique temp dirs; failures are
//! provoked by removing sources between planning and applying rather
//! than by permission tricks, which do not work when tests run as
//! root.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use autocompare_compare::{
    Error, SyncAction, SyncDirection, SyncOptions, SyncPlan, SyncStep, apply_sync, plan_sync,
};
use common::{cleanup, fixed_time, set_mtime, temp_root, write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// A left/right pair of roots under `base`; both directories exist,
/// as plan and apply require.
fn make_pair(base: &Path) -> (PathBuf, PathBuf) {
    let left = base.join("left");
    let right = base.join("right");
    std::fs::create_dir_all(&left).expect("failed to create the left root");
    std::fs::create_dir_all(&right).expect("failed to create the right root");
    (left, right)
}

/// Sync options for one direction, with or without pruning.
fn opts(direction: SyncDirection, prune: bool) -> SyncOptions {
    SyncOptions { direction, prune }
}

/// A time strictly after [`fixed_time`], for "newer" fixtures.
fn later() -> SystemTime {
    fixed_time()
        .checked_add(Duration::from_secs(3_600))
        .unwrap()
}

/// The planned step for `rel`, if any.
fn step<'a>(plan: &'a SyncPlan, rel: &str) -> Option<&'a SyncStep> {
    plan.steps.iter().find(|s| s.rel_path == rel)
}

/// The skip reason recorded for `rel`, if any.
fn skipped<'a>(plan: &'a SyncPlan, rel: &str) -> Option<&'a str> {
    plan.skipped
        .iter()
        .find(|(path, _)| path == rel)
        .map(|(_, reason)| reason.as_str())
}

/// Writes the same bytes on both sides and pins both mtimes.
fn pair(left: &Path, right: &Path, rel: &str, contents: &str, mtime: SystemTime) {
    write(&left.join(rel), contents);
    write(&right.join(rel), contents);
    set_mtime(&left.join(rel), mtime);
    set_mtime(&right.join(rel), mtime);
}

/// Writes a file on one side only.
fn one(side: &Path, rel: &str, contents: &str) {
    write(&side.join(rel), contents);
}

/// A tree exercising every file status: newer-left, newer-right,
/// left-only, right-only, same and different.
fn mixed_tree(left: &Path, right: &Path) {
    let t = fixed_time();

    // Same size, left mtime later: NewerLeft.
    one(left, "newer.txt", "aaa\n");
    one(right, "newer.txt", "bbb\n");
    set_mtime(&left.join("newer.txt"), later());
    set_mtime(&right.join("newer.txt"), t);

    // Same size, right mtime later: NewerRight.
    one(left, "older.txt", "ccc\n");
    one(right, "older.txt", "ddd\n");
    set_mtime(&left.join("older.txt"), t);
    set_mtime(&right.join("older.txt"), later());

    // One-sided files.
    one(left, "leftonly.txt", "L\n");
    one(right, "rightonly.txt", "R\n");

    // Identical bytes and mtimes: Same.
    pair(left, right, "same.txt", "same\n", t);

    // Same mtime, different sizes: Different.
    one(left, "differs.txt", "tiny");
    one(right, "differs.txt", "much longer content");
    set_mtime(&left.join("differs.txt"), t);
    set_mtime(&right.join("differs.txt"), t);
}

#[test]
fn plan_l2r_copies_newer_and_left_only_and_skips_the_rest() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    mixed_tree(&left, &right);

    let plan = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, false)).unwrap();
    assert!(!plan.truncated, "shallow trees are never truncated");

    assert_eq!(
        step(&plan, "newer.txt").map(|s| s.action),
        Some(SyncAction::CopyToRight)
    );
    assert!(!step(&plan, "newer.txt").unwrap().is_dir);
    assert_eq!(
        step(&plan, "leftonly.txt").map(|s| s.action),
        Some(SyncAction::CopyToRight)
    );

    // A newer destination is never overwritten by a one-directional
    // sync: skipped with a reason instead.
    assert_eq!(skipped(&plan, "older.txt"), Some("destination is newer"));
    assert!(step(&plan, "older.txt").is_none());

    assert_eq!(
        skipped(&plan, "rightonly.txt"),
        Some("only on the right side")
    );
    assert!(step(&plan, "rightonly.txt").is_none());

    assert_eq!(
        skipped(&plan, "differs.txt"),
        Some("content differs; no side wins")
    );
    assert!(step(&plan, "differs.txt").is_none());

    // Same entries appear in neither list.
    assert!(step(&plan, "same.txt").is_none());
    assert!(skipped(&plan, "same.txt").is_none());

    // No deletes without prune.
    assert!(
        plan.steps
            .iter()
            .all(|s| !matches!(s.action, SyncAction::DeleteLeft | SyncAction::DeleteRight))
    );
    cleanup(&base);
}

#[test]
fn plan_r2l_mirrors_l2r() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    mixed_tree(&left, &right);

    let plan = plan_sync(&left, &right, &opts(SyncDirection::RightToLeft, false)).unwrap();

    assert_eq!(
        step(&plan, "older.txt").map(|s| s.action),
        Some(SyncAction::CopyToLeft)
    );
    assert_eq!(
        step(&plan, "rightonly.txt").map(|s| s.action),
        Some(SyncAction::CopyToLeft)
    );

    assert_eq!(skipped(&plan, "newer.txt"), Some("destination is newer"));
    assert_eq!(
        skipped(&plan, "leftonly.txt"),
        Some("only on the left side")
    );
    assert_eq!(
        skipped(&plan, "differs.txt"),
        Some("content differs; no side wins")
    );

    assert!(step(&plan, "same.txt").is_none());
    assert!(skipped(&plan, "same.txt").is_none());
    cleanup(&base);
}

#[test]
fn plan_newer_wins_copies_both_directions() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    mixed_tree(&left, &right);

    let plan = plan_sync(&left, &right, &opts(SyncDirection::NewerWins, false)).unwrap();
    assert_eq!(
        step(&plan, "newer.txt").map(|s| s.action),
        Some(SyncAction::CopyToRight)
    );
    assert_eq!(
        step(&plan, "older.txt").map(|s| s.action),
        Some(SyncAction::CopyToLeft)
    );
    assert_eq!(
        step(&plan, "leftonly.txt").map(|s| s.action),
        Some(SyncAction::CopyToRight)
    );
    assert_eq!(
        step(&plan, "rightonly.txt").map(|s| s.action),
        Some(SyncAction::CopyToLeft)
    );
    // No side wins on content: still skipped, and the newer files
    // are not reported as skipped.
    assert_eq!(
        skipped(&plan, "differs.txt"),
        Some("content differs; no side wins")
    );
    assert!(skipped(&plan, "newer.txt").is_none());

    // Prune is meaningless without a single winning side: still no
    // deletes, one-sided entries are copied instead.
    let pruned = plan_sync(&left, &right, &opts(SyncDirection::NewerWins, true)).unwrap();
    assert!(
        pruned
            .steps
            .iter()
            .all(|s| !matches!(s.action, SyncAction::DeleteLeft | SyncAction::DeleteRight))
    );
    assert_eq!(
        step(&pruned, "leftonly.txt").map(|s| s.action),
        Some(SyncAction::CopyToRight)
    );
    cleanup(&base);
}

#[test]
fn plan_prune_l2r_deletes_right_only_files_and_dirs() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    one(&left, "keep.txt", "k\n");
    // A left-only directory: copied per file, never a step itself.
    one(&left, "newdir/child.txt", "c\n");
    one(&right, "extra.txt", "e\n");
    one(&right, "extradir/inner.txt", "i\n");

    let plan = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, true)).unwrap();

    let extra = step(&plan, "extra.txt").unwrap();
    assert_eq!(extra.action, SyncAction::DeleteRight);
    assert!(!extra.is_dir);

    let dir = step(&plan, "extradir").unwrap();
    assert_eq!(dir.action, SyncAction::DeleteRight);
    assert!(dir.is_dir);
    // The doomed directory's children die with it: not planned, not
    // even reported as skipped.
    assert!(step(&plan, "extradir/inner.txt").is_none());
    assert!(skipped(&plan, "extradir/inner.txt").is_none());

    // The winning side is copied, not pruned.
    assert_eq!(
        step(&plan, "keep.txt").map(|s| s.action),
        Some(SyncAction::CopyToRight)
    );
    // A left-only directory produces no step of its own; its child
    // copies and the directory is created at apply time.
    assert!(step(&plan, "newdir").is_none());
    assert!(skipped(&plan, "newdir").is_none());
    assert_eq!(
        step(&plan, "newdir/child.txt").map(|s| s.action),
        Some(SyncAction::CopyToRight)
    );
    cleanup(&base);
}

#[test]
fn plan_prune_r2l_deletes_left_only_files_and_dirs() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    one(&right, "keep.txt", "k\n");
    one(&left, "extra.txt", "e\n");
    one(&left, "extradir/inner.txt", "i\n");

    let plan = plan_sync(&left, &right, &opts(SyncDirection::RightToLeft, true)).unwrap();

    let extra = step(&plan, "extra.txt").unwrap();
    assert_eq!(extra.action, SyncAction::DeleteLeft);
    assert!(!extra.is_dir);
    let dir = step(&plan, "extradir").unwrap();
    assert_eq!(dir.action, SyncAction::DeleteLeft);
    assert!(dir.is_dir);
    assert!(step(&plan, "extradir/inner.txt").is_none());
    assert!(skipped(&plan, "extradir/inner.txt").is_none());
    assert_eq!(
        step(&plan, "keep.txt").map(|s| s.action),
        Some(SyncAction::CopyToLeft)
    );
    cleanup(&base);
}

#[test]
fn plan_reports_truncated_false_on_shallow_trees() {
    // plan_sync always compares with FolderOptions::default()
    // (depth_limit 32), so a deep-tree truncation cannot be provoked
    // from the outside; plumbing folder options through SyncOptions
    // is future work. The flag is asserted where it is observable:
    // false on normal trees.
    let base = temp_root();
    let (left, right) = make_pair(&base);
    one(&left, "a.txt", "a\n");
    let plan = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, false)).unwrap();
    assert!(!plan.truncated);
    cleanup(&base);
}

#[test]
fn apply_l2r_copies_files_and_creates_missing_dirs() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    let t = fixed_time();

    // A newer-left file whose right copy holds different bytes.
    one(&left, "shared.txt", "new\n");
    one(&right, "shared.txt", "old\n");
    set_mtime(&left.join("shared.txt"), later());
    set_mtime(&right.join("shared.txt"), t);

    // A left-only file inside a directory the right side lacks.
    one(&left, "newdir/inner.txt", "inner\n");

    let plan = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, false)).unwrap();
    assert!(
        step(&plan, "newdir").is_none(),
        "directories are never copy steps"
    );
    assert_eq!(plan.steps.len(), 2);

    let report = apply_sync(&left, &right, &plan).unwrap();
    assert_eq!(report.copied, 2);
    assert_eq!(report.deleted, 0);
    assert!(report.failed.is_empty());

    // The destinations now hold the left bytes.
    assert_eq!(std::fs::read(right.join("shared.txt")).unwrap(), b"new\n");
    assert_eq!(
        std::fs::read(right.join("newdir/inner.txt")).unwrap(),
        b"inner\n"
    );

    // Re-planning the synced trees converges. Copying does not
    // preserve mtimes (the destination is written now), so both
    // sides are pinned first to keep the verdict deterministic.
    for rel in ["shared.txt", "newdir/inner.txt"] {
        set_mtime(&left.join(rel), t);
        set_mtime(&right.join(rel), t);
    }
    let again = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, false)).unwrap();
    assert!(again.steps.is_empty());
    assert!(again.skipped.is_empty());
    cleanup(&base);
}

#[test]
fn apply_prune_deletes_right_only_entries() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    one(&left, "keep.txt", "k\n");
    one(&right, "extra.txt", "e\n");
    one(&right, "extradir/inner.txt", "i\n");

    let plan = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, true)).unwrap();
    let report = apply_sync(&left, &right, &plan).unwrap();
    assert_eq!(report.deleted, 2, "one file and one whole directory");
    assert_eq!(report.copied, 1);
    assert!(report.failed.is_empty());

    assert!(!right.join("extra.txt").exists());
    assert!(!right.join("extradir").exists());
    assert!(right.join("keep.txt").exists());
    assert!(left.join("keep.txt").exists());
    cleanup(&base);
}

#[test]
fn apply_records_vanished_sources_and_continues() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    let t = fixed_time();

    one(&left, "gone.txt", "aaa\n");
    one(&right, "gone.txt", "zzz\n");
    set_mtime(&left.join("gone.txt"), later());
    set_mtime(&right.join("gone.txt"), t);
    one(&left, "kept.txt", "kept\n");

    let plan = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, false)).unwrap();
    assert_eq!(plan.steps.len(), 2);

    // The source of the first step vanishes before applying.
    std::fs::remove_file(left.join("gone.txt")).unwrap();

    let report = apply_sync(&left, &right, &plan).unwrap();
    assert_eq!(report.copied, 1, "the run continued past the failure");
    assert_eq!(
        report.failed,
        vec![("gone.txt".to_owned(), "source vanished".to_owned())]
    );
    // The other step still applied.
    assert_eq!(std::fs::read(right.join("kept.txt")).unwrap(), b"kept\n");
    // The vanished file's old right copy is untouched.
    assert_eq!(std::fs::read(right.join("gone.txt")).unwrap(), b"zzz\n");
    cleanup(&base);
}

#[test]
fn apply_refuses_unsafe_rel_paths() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    one(&left, "real.txt", "real\n");
    // A canary just outside both roots, inside their shared parent.
    one(&base, "canary.txt", "canary\n");

    let plan = SyncPlan {
        steps: vec![
            SyncStep {
                rel_path: "../evil".into(),
                is_dir: false,
                action: SyncAction::CopyToRight,
            },
            SyncStep {
                rel_path: "/abs".into(),
                is_dir: false,
                action: SyncAction::DeleteRight,
            },
            SyncStep {
                rel_path: "real.txt".into(),
                is_dir: false,
                action: SyncAction::CopyToRight,
            },
        ],
        skipped: Vec::new(),
        truncated: false,
    };
    let report = apply_sync(&left, &right, &plan).unwrap();
    assert_eq!(report.copied, 1, "the safe step still ran");
    assert_eq!(
        report.failed,
        vec![
            ("../evil".to_owned(), "unsafe path".to_owned()),
            ("/abs".to_owned(), "unsafe path".to_owned()),
        ]
    );
    // Nothing outside the roots was touched.
    assert_eq!(std::fs::read(base.join("canary.txt")).unwrap(), b"canary\n");
    assert!(!base.join("evil").exists());
    assert!(right.join("real.txt").exists());
    cleanup(&base);
}

#[test]
fn missing_or_file_roots_are_hard_errors() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    one(&base, "plain.txt", "file\n");
    let plan = SyncPlan::default();

    let err = plan_sync(
        &base.join("nope"),
        &right,
        &opts(SyncDirection::LeftToRight, false),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Missing { .. }));

    let err = plan_sync(
        &base.join("plain.txt"),
        &right,
        &opts(SyncDirection::LeftToRight, false),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Missing { .. }));

    // apply_sync validates roots even for an empty plan.
    let err = apply_sync(&base.join("nope"), &right, &plan).unwrap_err();
    assert!(matches!(err, Error::Missing { .. }));
    assert!(err.to_string().contains("nope"));

    let err = apply_sync(&left, &base.join("plain.txt"), &plan).unwrap_err();
    assert!(matches!(err, Error::Missing { .. }));
    cleanup(&base);
}

#[test]
fn plan_steps_are_sorted_by_rel_path() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    for rel in ["z.txt", "a.txt", "m.txt", "sub/nested.txt"] {
        one(&left, rel, "x\n");
    }
    let plan = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, false)).unwrap();
    let rels: Vec<&str> = plan.steps.iter().map(|s| s.rel_path.as_str()).collect();
    // "sub" itself is a one-sided directory: no step, children only.
    assert_eq!(rels, vec!["a.txt", "m.txt", "sub/nested.txt", "z.txt"]);
    cleanup(&base);
}

#[test]
#[cfg(unix)]
fn plan_refuses_to_copy_symlink_sources() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    let outside = base.join("outside");
    one(&outside, "secret.txt", "secret\n");
    std::os::unix::fs::symlink(outside.join("secret.txt"), left.join("lnk")).unwrap();
    one(&left, "plain.txt", "p\n");

    let plan = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, false)).unwrap();
    assert_eq!(skipped(&plan, "lnk"), Some("symlink; not followed"));
    assert!(step(&plan, "lnk").is_none());
    assert_eq!(
        step(&plan, "plain.txt").map(|s| s.action),
        Some(SyncAction::CopyToRight)
    );
    cleanup(&base);
}

#[test]
#[cfg(unix)]
fn apply_refuses_to_copy_through_symlinks() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    let outside = base.join("outside");
    one(&outside, "victim.txt", "victim\n");
    one(&left, "x.txt", "new\n");
    // The destination is a symlink pointing outside the roots.
    std::os::unix::fs::symlink(outside.join("victim.txt"), right.join("x.txt")).unwrap();

    let plan = SyncPlan {
        steps: vec![SyncStep {
            rel_path: "x.txt".into(),
            is_dir: false,
            action: SyncAction::CopyToRight,
        }],
        skipped: Vec::new(),
        truncated: false,
    };
    let report = apply_sync(&left, &right, &plan).unwrap();
    assert_eq!(report.copied, 0);
    assert_eq!(
        report.failed,
        vec![("x.txt".to_owned(), "symlink; not followed".to_owned())]
    );
    // The symlink's target outside the roots was not overwritten.
    assert_eq!(
        std::fs::read(outside.join("victim.txt")).unwrap(),
        b"victim\n"
    );
    cleanup(&base);
}

#[test]
#[cfg(unix)]
fn apply_prune_removes_a_symlink_without_following_it() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    let outside = base.join("outside");
    one(&outside, "secret.txt", "secret\n");
    // A symlink to a directory, one-sided on the right.
    std::os::unix::fs::symlink(&outside, right.join("lnkdir")).unwrap();

    let plan = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, true)).unwrap();
    let lnk = step(&plan, "lnkdir").unwrap();
    assert_eq!(lnk.action, SyncAction::DeleteRight);
    assert!(
        !lnk.is_dir,
        "the walk never classifies a symlink as a directory"
    );

    let report = apply_sync(&left, &right, &plan).unwrap();
    assert_eq!(report.deleted, 1);
    assert!(report.failed.is_empty());
    assert!(!right.join("lnkdir").exists());
    // The link's target survived.
    assert!(outside.join("secret.txt").exists());
    cleanup(&base);
}
