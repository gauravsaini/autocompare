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
    Error, FolderOptions, SyncAction, SyncDirection, SyncOptions, SyncPlan, SyncStep, apply_sync,
    plan_sync, plan_sync_with_compare,
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
    // (depth_limit 32): shallow trees never truncate. Deeper
    // truncation is plumbed through plan_sync_with_compare (see the
    // depth test below).
    let base = temp_root();
    let (left, right) = make_pair(&base);
    one(&left, "a.txt", "a\n");
    let plan = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, false)).unwrap();
    assert!(!plan.truncated);
    cleanup(&base);
}

#[test]
fn plan_with_compare_depth_limit_truncates_and_skips_deeper() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    let t = fixed_time();
    pair(&left, &right, "top.txt", "top\n", t);
    pair(&left, &right, "sub/mid.txt", "mid\n", t);
    one(&left, "sub/deep/leaf.txt", "leaf\n");

    let compare = FolderOptions {
        depth_limit: 1,
        ..FolderOptions::default()
    };
    let sync_opts = opts(SyncDirection::LeftToRight, false);
    let plan = plan_sync_with_compare(&left, &right, &sync_opts, &compare).unwrap();
    assert!(plan.truncated, "depth cut must be reported");
    let rels: Vec<&str> = plan.steps.iter().map(|s| s.rel_path.as_str()).collect();
    assert!(
        !rels.iter().any(|r| r.starts_with("sub/deep")),
        "deeper entries are not planned: {rels:?}"
    );
    assert!(
        plan.skipped
            .iter()
            .all(|(path, _)| !path.starts_with("sub/deep")),
        "deeper entries are not even reported as skipped"
    );

    // The same tree with default comparison walks everything.
    let full = plan_sync(&left, &right, &sync_opts).unwrap();
    assert!(!full.truncated);
    assert!(
        full.steps.iter().any(|s| s.rel_path == "sub/deep/leaf.txt"),
        "default walk plans the deep leaf"
    );
    cleanup(&base);
}

#[test]
fn plan_with_compare_include_filter_limits_mirror_prune() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    let t = fixed_time();
    pair(&left, &right, "keep.txt", "k\n", t);
    one(&left, "new.txt", "n\n");
    one(&right, "extra.txt", "e\n");
    one(&right, "extra.log", "logged\n");

    let compare = FolderOptions {
        include: Some("*.txt".into()),
        ..FolderOptions::default()
    };
    let mirror = opts(SyncDirection::LeftToRight, true);
    let plan = plan_sync_with_compare(&left, &right, &mirror, &compare).unwrap();
    assert!(!plan.truncated);
    let rels: Vec<&str> = plan.steps.iter().map(|s| s.rel_path.as_str()).collect();
    assert!(rels.contains(&"new.txt"), "matched left-only is copied");
    assert!(rels.contains(&"extra.txt"), "matched right-only is pruned");
    assert!(
        !rels.iter().any(|r| r.ends_with(".log")),
        "filtered-out names are never planned: {rels:?}"
    );
    assert!(
        plan.skipped.iter().all(|(path, _)| !path.ends_with(".log")),
        "filtered-out names are not reported either"
    );
    assert_eq!(
        step(&plan, "extra.txt").map(|s| s.action),
        Some(SyncAction::DeleteRight)
    );
    cleanup(&base);
}

#[test]
fn plan_with_compare_content_compare_flags_same_size_bytes() {
    let base = temp_root();
    let (left, right) = make_pair(&base);
    let t = fixed_time();
    // Same size, different bytes, pinned mtimes: the default
    // size+mtime rule calls this Same.
    one(&left, "c.bin", "aaaa");
    one(&right, "c.bin", "aaab");
    set_mtime(&left.join("c.bin"), t);
    set_mtime(&right.join("c.bin"), t);

    let sync_opts = opts(SyncDirection::LeftToRight, false);
    let plain = plan_sync(&left, &right, &sync_opts).unwrap();
    assert!(step(&plain, "c.bin").is_none());
    assert!(skipped(&plain, "c.bin").is_none());

    let compare = FolderOptions {
        content_compare: true,
        ..FolderOptions::default()
    };
    let compared = plan_sync_with_compare(&left, &right, &sync_opts, &compare).unwrap();
    assert!(step(&compared, "c.bin").is_none());
    assert_eq!(
        skipped(&compared, "c.bin"),
        Some("content differs; no side wins")
    );
    cleanup(&base);
}

#[test]
fn sync_planner_compare_options_drive_dry_run_and_apply() {
    use autocompare_compare::{SyncMode, SyncPlanner};

    let base = temp_root();
    let (left, right) = make_pair(&base);
    let t = fixed_time();
    pair(&left, &right, "same.txt", "same\n", t);
    one(&left, "new.txt", "n\n");
    one(&right, "extra.txt", "e\n");
    one(&right, "junk.log", "j\n");

    let planner =
        SyncPlanner::new(&left, &right, SyncMode::Mirror).with_compare_options(FolderOptions {
            exclude: Some("*.log".into()),
            ..FolderOptions::default()
        });
    assert_eq!(planner.compare_options().exclude.as_deref(), Some("*.log"));
    let plan = planner.dry_run().unwrap();
    assert!(!plan.truncated);
    let rels: Vec<&str> = plan.steps.iter().map(|s| s.rel_path.as_str()).collect();
    assert!(rels.contains(&"new.txt"));
    assert!(rels.contains(&"extra.txt"));
    assert!(
        !rels.iter().any(|r| r.ends_with(".log")),
        "filtered junk is not pruned: {rels:?}"
    );

    let report = planner.apply(&plan).unwrap();
    assert!(report.is_success());
    assert!(right.join("new.txt").exists());
    assert!(!right.join("extra.txt").exists());
    assert!(
        right.join("junk.log").exists(),
        "filtered-out right-only file survives Mirror"
    );
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
                rel_path: String::new(),
                is_dir: false,
                action: SyncAction::CopyToRight,
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
            (String::new(), "unsafe path".to_owned()),
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

#[test]
fn sync_planner_all_modes_dry_run_and_apply() {
    use autocompare_compare::{SyncMode, SyncPlanner};

    let base = temp_root();
    let (left, right) = make_pair(&base);
    std::fs::create_dir_all(&left).unwrap();
    std::fs::create_dir_all(&right).unwrap();

    let t = fixed_time();
    one(&left, "left_new.txt", "left_new");
    set_mtime(&left.join("left_new.txt"), later());

    one(&right, "right_new.txt", "right_new");
    set_mtime(&right.join("right_new.txt"), later());

    one(&right, "extra_on_right.txt", "extra");
    set_mtime(&right.join("extra_on_right.txt"), t);

    // 1. UpdateRight dry run & plan
    let p_update_r = SyncPlanner::new(&left, &right, SyncMode::UpdateRight);
    let plan_r = p_update_r.dry_run().unwrap();
    assert_eq!(p_update_r.mode(), SyncMode::UpdateRight);
    assert!(step(&plan_r, "left_new.txt").is_some());
    assert!(step(&plan_r, "extra_on_right.txt").is_none());

    // 2. UpdateLeft dry run
    let p_update_l = SyncPlanner::new(&left, &right, SyncMode::UpdateLeft);
    let plan_l = p_update_l.plan().unwrap();
    assert!(step(&plan_l, "right_new.txt").is_some());
    assert!(step(&plan_l, "left_new.txt").is_none());

    // 3. Both (bi-directional)
    let p_both = SyncPlanner::new(&left, &right, SyncMode::Both);
    let plan_both = p_both.dry_run().unwrap();
    assert!(step(&plan_both, "left_new.txt").is_some());
    assert!(step(&plan_both, "right_new.txt").is_some());

    // 4. Mirror (copies to right and prunes extra on right)
    let p_mirror = SyncPlanner::new(&left, &right, SyncMode::Mirror);
    let plan_mirror = p_mirror.dry_run().unwrap();
    let prune_step = step(&plan_mirror, "extra_on_right.txt").unwrap();
    assert_eq!(prune_step.action, SyncAction::DeleteRight);

    // Apply Mirror
    let rep = p_mirror.apply(&plan_mirror).unwrap();
    assert_eq!(rep.failed.len(), 0);
    assert!(right.join("left_new.txt").exists());
    assert!(!right.join("extra_on_right.txt").exists());

    cleanup(&base);
}

#[test]
fn sync_plan_conflict_navigation_and_reports() {
    use autocompare_compare::{SyncMode, SyncPlanner};
    use std::sync::atomic::AtomicBool;

    let base = temp_root();
    let (left, right) = make_pair(&base);

    // Mixed tree with conflicts
    mixed_tree(&left, &right);

    let mut planner = SyncPlanner::new(&left, &right, SyncMode::UpdateRight);
    assert!(!planner.is_mirror());
    assert!(!planner.is_bidirectional());

    let plan = planner.plan().unwrap();
    assert!(plan.has_conflicts());
    assert_eq!(plan.conflict_count(), 2); // older.txt (destination is newer) + differs.txt (content differs)

    let conflicts = plan.conflicts();
    assert_eq!(conflicts.len(), 2);
    let c_paths = plan.conflict_paths();
    assert!(c_paths.contains(&"differs.txt"));
    assert!(c_paths.contains(&"older.txt"));

    let first = plan.first_conflict();
    assert!(first.is_some());
    let last = plan.last_conflict();
    assert!(last.is_some());

    // Navigation
    let next_from_none = plan.next_conflict(None);
    assert_eq!(next_from_none, first);

    let next_from_first = plan.next_conflict(Some(&first.unwrap().0));
    assert_eq!(next_from_first, last);

    let next_from_last = plan.next_conflict(Some(&last.unwrap().0));
    assert_eq!(next_from_last, None);

    let prev_from_none = plan.prev_conflict(None);
    assert_eq!(prev_from_none, last);

    let prev_from_last = plan.prev_conflict(Some(&last.unwrap().0));
    assert_eq!(prev_from_last, first);

    // Plan step counters & summary
    assert_eq!(plan.copy_to_right_count(), 2); // newer.txt, leftonly.txt
    assert_eq!(plan.copy_to_left_count(), 0);
    assert!(plan.summary().contains("SyncPlan:"));
    assert!(
        planner
            .dry_run_summary(&plan)
            .contains("Dry-run Update Right:")
    );

    // Planner helper methods
    planner.set_mode(SyncMode::Both);
    assert!(planner.is_bidirectional());
    let planner = planner.with_mode(SyncMode::Mirror);
    assert!(planner.is_mirror());

    // Report
    let report = planner.run().unwrap();
    assert!(report.is_success());
    assert_eq!(report.failure_count(), 0);
    assert!(report.total_applied() > 0);
    assert!(report.summary().contains("SyncReport:"));

    // Cancellation support
    let cancel = AtomicBool::new(true);
    let cancel_res = planner.run_cancellable(&cancel);
    assert!(matches!(cancel_res, Err(Error::Cancelled)));

    cleanup(&base);
}

#[test]
fn toolspec_gui_verdicts_match_engine_plans_on_orphan_newer_edge() {
    use autocompare_compare::{
        FolderStatus, SyncMode, gui_action_for, gui_skip_reason_for, gui_sync_modes,
        gui_sync_options,
    };

    // Edge fixture: two flat folders with one orphan per side, one
    // newer file per side, plus same + content-differ controls.
    let base = temp_root();
    let (left, right) = make_pair(&base);
    let t = fixed_time();
    one(&left, "orphan-left.txt", "L\n");
    one(&right, "orphan-right.txt", "R\n");
    one(&left, "newer.txt", "aaa\n");
    one(&right, "newer.txt", "bbb\n");
    set_mtime(&left.join("newer.txt"), later());
    set_mtime(&right.join("newer.txt"), t);
    one(&left, "older.txt", "ccc\n");
    one(&right, "older.txt", "ddd\n");
    set_mtime(&left.join("older.txt"), t);
    set_mtime(&right.join("older.txt"), later());
    pair(&left, &right, "same.txt", "same\n", t);
    one(&left, "differs.txt", "tiny");
    one(&right, "differs.txt", "much longer content");
    set_mtime(&left.join("differs.txt"), t);
    set_mtime(&right.join("differs.txt"), t);

    fn glyph(status: FolderStatus) -> char {
        match status {
            FolderStatus::Same => '=',
            FolderStatus::Different => '!',
            FolderStatus::NewerLeft => '>',
            FolderStatus::NewerRight => '<',
            FolderStatus::LeftOnly => 'L',
            FolderStatus::RightOnly => 'R',
        }
    }

    // Pin the classification the engine must produce for this fixture.
    let want = [
        ("orphan-left.txt", FolderStatus::LeftOnly),
        ("orphan-right.txt", FolderStatus::RightOnly),
        ("newer.txt", FolderStatus::NewerLeft),
        ("older.txt", FolderStatus::NewerRight),
        ("same.txt", FolderStatus::Same),
        ("differs.txt", FolderStatus::Different),
    ];

    let modes = gui_sync_modes();
    assert_eq!(
        modes,
        [
            SyncMode::UpdateLeft,
            SyncMode::UpdateRight,
            SyncMode::Both,
            SyncMode::Mirror
        ]
    );
    for mode in modes {
        let opts = gui_sync_options(mode);
        let plan = plan_sync(&left, &right, &opts).unwrap();
        for (rel, status) in want {
            let g = glyph(status);
            let tool_action = gui_action_for(g, false, &opts);
            let tool_skip = gui_skip_reason_for(g, false, &opts);
            let plan_action = step(&plan, rel).map(|s| s.action);
            let plan_skip = skipped(&plan, rel);
            assert_eq!(tool_action, plan_action, "action for {rel} in {mode:?}");
            assert_eq!(tool_skip, plan_skip, "skip reason for {rel} in {mode:?}");
        }
    }
    cleanup(&base);
}

#[test]
fn toolspec_conflict_nav_and_live_log_cover_plan_and_report() {
    use autocompare_compare::{
        SyncLiveLog, is_sync_conflict_reason, sync_conflict_next, sync_conflict_prev,
    };

    let base = temp_root();
    let (left, right) = make_pair(&base);
    mixed_tree(&left, &right);

    let plan = plan_sync(&left, &right, &opts(SyncDirection::LeftToRight, false)).unwrap();
    assert!(plan.has_conflicts());
    assert!(is_sync_conflict_reason("content differs; no side wins"));
    assert!(is_sync_conflict_reason("destination is newer"));
    assert!(!is_sync_conflict_reason("only on the right side"));

    // Headless conflict walk: first -> last -> end, then back.
    let first = sync_conflict_next(&plan.skipped, None).unwrap();
    let last = sync_conflict_prev(&plan.skipped, None).unwrap();
    assert!(sync_conflict_next(&plan.skipped, Some(&last.0)).is_none());
    assert_eq!(
        sync_conflict_next(&plan.skipped, Some(&first.0)).unwrap().0,
        last.0
    );
    assert_eq!(
        sync_conflict_prev(&plan.skipped, Some(&last.0)).unwrap().0,
        first.0
    );

    // Live log: plan summary + steps + skips, then report summary.
    let mut log = SyncLiveLog::new();
    assert!(log.is_empty());
    log.push_plan(&plan);
    assert!(!log.is_empty());
    assert!(log.lines().first().is_some_and(|l| l.contains("SyncPlan:")));
    assert!(log.lines().iter().any(|l| l.contains("copy L->R")));
    assert!(log.last().is_some());

    let report = apply_sync(
        &left,
        &right,
        &plan.select_subset(&plan.steps.iter().map(|_| true).collect::<Vec<_>>()),
    )
    .unwrap();
    let mut log2 = SyncLiveLog::new();
    log2.push_report(&report);
    assert!(
        log2.lines()
            .first()
            .is_some_and(|l| l.contains("SyncReport:"))
    );
    log.clear();
    assert!(log.is_empty());
    assert_eq!(log.len(), 0);

    cleanup(&base);
}
