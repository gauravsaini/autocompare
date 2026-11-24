//! Integration tests for the three-way folder merge: plan rules,
//! symlink policy and applying into real output trees. Fixtures are
//! content-based; mtimes are never pinned because the merge never
//! reads them.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use autocompare_compare::{
    Error, TreeMergeAction, TreeMergePlan, TreeMergeStep, apply_tree_merge, plan_tree_merge,
};
use common::{cleanup, temp_root, write, write_bytes};
use std::path::{Path, PathBuf};

/// Creates the three roots under `base`.
fn make_roots(base: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let b = base.join("base");
    let l = base.join("left");
    let r = base.join("right");
    std::fs::create_dir_all(&b).expect("failed to create the base root");
    std::fs::create_dir_all(&l).expect("failed to create the left root");
    std::fs::create_dir_all(&r).expect("failed to create the right root");
    (b, l, r)
}

/// The planned step for `rel`, if any.
fn step<'a>(plan: &'a TreeMergePlan, rel: &str) -> Option<&'a TreeMergeStep> {
    plan.steps.iter().find(|s| s.rel_path == rel)
}

/// The planned action for `rel`, if any.
fn action(plan: &TreeMergePlan, rel: &str) -> Option<TreeMergeAction> {
    step(plan, rel).map(|s| s.action)
}

/// The merged lines of the step for `rel`, asserted to exist.
fn merged_lines<'a>(plan: &'a TreeMergePlan, rel: &str) -> &'a [String] {
    step(plan, rel).unwrap().merged.as_deref().unwrap_or(&[])
}

/// The skip reason recorded for `rel`, if any.
fn skipped<'a>(plan: &'a TreeMergePlan, rel: &str) -> Option<&'a str> {
    plan.skipped
        .iter()
        .find(|(path, _)| path == rel)
        .map(|(_, reason)| reason.as_str())
}

/// Whether `rel` is recorded as a conflict.
fn is_conflict(plan: &TreeMergePlan, rel: &str) -> bool {
    plan.conflicts.iter().any(|path| path == rel)
}

#[test]
fn one_sided_changes_take_the_changed_side() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    // Only the left changed: TakeLeft.
    write(&b.join("left_edit.txt"), "a\n");
    write(&l.join("left_edit.txt"), "b\n");
    write(&r.join("left_edit.txt"), "a\n");

    // Only the right changed: TakeRight.
    write(&b.join("right_edit.txt"), "a\n");
    write(&l.join("right_edit.txt"), "a\n");
    write(&r.join("right_edit.txt"), "c\n");

    // Both made the same change: the left copy is taken.
    write(&b.join("same_change.txt"), "a\n");
    write(&l.join("same_change.txt"), "z\n");
    write(&r.join("same_change.txt"), "z\n");

    // Untouched: identical on both sides, so TakeLeft.
    write(&b.join("untouched.txt"), "a\n");
    write(&l.join("untouched.txt"), "a\n");
    write(&r.join("untouched.txt"), "a\n");

    let plan = plan_tree_merge(&b, &l, &r).unwrap();
    assert_eq!(
        action(&plan, "left_edit.txt"),
        Some(TreeMergeAction::TakeLeft)
    );
    assert_eq!(
        action(&plan, "right_edit.txt"),
        Some(TreeMergeAction::TakeRight)
    );
    assert_eq!(
        action(&plan, "same_change.txt"),
        Some(TreeMergeAction::TakeLeft)
    );
    assert_eq!(
        action(&plan, "untouched.txt"),
        Some(TreeMergeAction::TakeLeft)
    );
    assert!(plan.conflicts.is_empty());
    assert!(plan.skipped.is_empty());

    cleanup(&base);
}

#[test]
fn clean_text_merge_takes_edits_from_both_sides() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    write(&b.join("doc.txt"), "a\nb\nc\n");
    write(&l.join("doc.txt"), "A\nb\nc\n"); // left edits the first line
    write(&r.join("doc.txt"), "a\nb\nC\n"); // right edits the last line

    let plan = plan_tree_merge(&b, &l, &r).unwrap();
    assert_eq!(action(&plan, "doc.txt"), Some(TreeMergeAction::MergedText));
    assert_eq!(
        merged_lines(&plan, "doc.txt"),
        ["A".to_owned(), "b".to_owned(), "C".to_owned()].as_slice()
    );
    assert!(plan.conflicts.is_empty());
    assert!(plan.skipped.is_empty());

    // Apply: the merged lines land in the output, newline-terminated.
    let out = base.join("out");
    let report = apply_tree_merge(&l, &r, &plan, &out).unwrap();
    assert_eq!(report.written, 1);
    assert_eq!(report.dirs_created, 0);
    assert!(report.failed.is_empty());
    assert_eq!(
        std::fs::read_to_string(out.join("doc.txt")).unwrap(),
        "A\nb\nC\n"
    );

    cleanup(&base);
}

#[test]
fn overlapping_text_edits_conflict_with_markers() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    write(&b.join("c.txt"), "x\n");
    write(&l.join("c.txt"), "y\n");
    write(&r.join("c.txt"), "z\n");

    let plan = plan_tree_merge(&b, &l, &r).unwrap();
    assert_eq!(action(&plan, "c.txt"), Some(TreeMergeAction::Conflict));
    assert!(
        merged_lines(&plan, "c.txt")
            .iter()
            .any(|line| line.starts_with("<<<<<<<")),
        "conflict markers expected"
    );
    assert!(is_conflict(&plan, "c.txt"));
    assert!(plan.skipped.is_empty());

    // Apply writes the marker lines verbatim.
    let out = base.join("out");
    apply_tree_merge(&l, &r, &plan, &out).unwrap();
    let written = std::fs::read_to_string(out.join("c.txt")).unwrap();
    assert!(written.starts_with("<<<<<<<"));
    assert!(written.ends_with(">>>>>>> right\n"));

    cleanup(&base);
}

#[test]
fn both_changed_binaries_conflict_without_merged_content() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    write_bytes(&b.join("blob.bin"), &[0xFF, 0x01, 0xFE]);
    write_bytes(&l.join("blob.bin"), &[0xFF, 0x01, 0xFD]);
    write_bytes(&r.join("blob.bin"), &[0xFF, 0x02, 0xFE]);

    let plan = plan_tree_merge(&b, &l, &r).unwrap();
    assert_eq!(action(&plan, "blob.bin"), Some(TreeMergeAction::Conflict));
    assert!(step(&plan, "blob.bin").unwrap().merged.is_none());
    assert!(is_conflict(&plan, "blob.bin"));

    // Apply writes the LEFT copy.
    let out = base.join("out");
    apply_tree_merge(&l, &r, &plan, &out).unwrap();
    assert_eq!(
        std::fs::read(out.join("blob.bin")).unwrap(),
        vec![0xFF, 0x01, 0xFD]
    );

    cleanup(&base);
}

#[test]
fn deletions_follow_the_delete_rules() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    // Left deleted, right unchanged: omitted with a reason.
    write(&b.join("gone_left.txt"), "a\n");
    write(&r.join("gone_left.txt"), "a\n");

    // Left deleted while right changed: keep right, record conflict.
    write(&b.join("del_left_change_right.txt"), "a\n");
    write(&r.join("del_left_change_right.txt"), "b\n");

    // Right deleted while left changed: keep left, record conflict.
    write(&b.join("del_right_change_left.txt"), "a\n");
    write(&l.join("del_right_change_left.txt"), "b\n");

    // Right deleted, left unchanged: omitted with a reason.
    write(&b.join("gone_right.txt"), "a\n");
    write(&l.join("gone_right.txt"), "a\n");

    // Deleted on both sides: omitted with a reason.
    write(&b.join("gone_both.txt"), "a\n");

    let plan = plan_tree_merge(&b, &l, &r).unwrap();
    assert!(step(&plan, "gone_left.txt").is_none());
    assert_eq!(
        skipped(&plan, "gone_left.txt"),
        Some("deleted on the left; right unchanged from base")
    );
    assert!(step(&plan, "gone_right.txt").is_none());
    assert_eq!(
        skipped(&plan, "gone_right.txt"),
        Some("deleted on the right; left unchanged from base")
    );
    assert!(step(&plan, "gone_both.txt").is_none());
    assert_eq!(
        skipped(&plan, "gone_both.txt"),
        Some("deleted on both sides")
    );

    assert_eq!(
        action(&plan, "del_left_change_right.txt"),
        Some(TreeMergeAction::TakeRight)
    );
    assert!(is_conflict(&plan, "del_left_change_right.txt"));
    assert!(skipped(&plan, "del_left_change_right.txt").is_none());
    assert_eq!(
        action(&plan, "del_right_change_left.txt"),
        Some(TreeMergeAction::TakeLeft)
    );
    assert!(is_conflict(&plan, "del_right_change_left.txt"));
    assert!(skipped(&plan, "del_right_change_left.txt").is_none());
    assert_eq!(plan.conflicts.len(), 2);

    cleanup(&base);
}

#[test]
fn both_new_files_merge_over_an_empty_base() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    // Identical new files: the left copy is taken.
    write(&l.join("new_same.txt"), "n\n");
    write(&r.join("new_same.txt"), "n\n");

    // Different new text files: a merge over an empty base conflicts.
    write(&l.join("new_diff.txt"), "p\n");
    write(&r.join("new_diff.txt"), "q\n");

    // Different new binaries: conflict, the left copy is written.
    write_bytes(&l.join("new_diff.bin"), &[0xFF, 0x00]);
    write_bytes(&r.join("new_diff.bin"), &[0xFE, 0x00]);

    let plan = plan_tree_merge(&b, &l, &r).unwrap();
    assert_eq!(
        action(&plan, "new_same.txt"),
        Some(TreeMergeAction::TakeLeft)
    );
    assert!(!is_conflict(&plan, "new_same.txt"));

    assert_eq!(
        action(&plan, "new_diff.txt"),
        Some(TreeMergeAction::Conflict)
    );
    assert!(
        merged_lines(&plan, "new_diff.txt")
            .iter()
            .any(|line| line.starts_with("<<<<<<<"))
    );
    assert!(is_conflict(&plan, "new_diff.txt"));

    assert_eq!(
        action(&plan, "new_diff.bin"),
        Some(TreeMergeAction::Conflict)
    );
    assert!(step(&plan, "new_diff.bin").unwrap().merged.is_none());
    assert!(is_conflict(&plan, "new_diff.bin"));

    cleanup(&base);
}

#[cfg(unix)]
#[test]
fn symlink_sources_are_never_followed() {
    use std::os::unix::fs::symlink;

    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    // A symlink only on the left: TakeLeft becomes a skip.
    write(&l.join("target.txt"), "target\n");
    symlink("target.txt", l.join("only_link.txt")).expect("failed to create the symlink");

    // A symlink on the left against a differing right file: the
    // conflict's left copy would be the symlink, so it is skipped.
    write(&r.join("clash.txt"), "right\n");
    symlink("target.txt", l.join("clash.txt")).expect("failed to create the symlink");

    // A symlink in base only, absent from both sides: never read,
    // treated as deleted on both sides.
    symlink("nowhere", b.join("base_link.txt")).expect("failed to create the base symlink");

    let plan = plan_tree_merge(&b, &l, &r).unwrap();
    assert!(step(&plan, "only_link.txt").is_none());
    assert_eq!(
        skipped(&plan, "only_link.txt"),
        Some("symlink; not followed")
    );
    assert!(!is_conflict(&plan, "only_link.txt"));

    assert!(step(&plan, "clash.txt").is_none());
    assert_eq!(skipped(&plan, "clash.txt"), Some("symlink; not followed"));
    assert!(is_conflict(&plan, "clash.txt"));

    assert_eq!(
        skipped(&plan, "base_link.txt"),
        Some("deleted on both sides")
    );

    // Defense in depth: applying refuses a hand-built plan whose
    // source turned out to be a symlink, without reading the target.
    let hostile = TreeMergePlan {
        steps: vec![TreeMergeStep {
            rel_path: "only_link.txt".to_owned(),
            action: TreeMergeAction::TakeLeft,
            merged: None,
        }],
        conflicts: Vec::new(),
        skipped: Vec::new(),
    };
    let out = base.join("out");
    let report = apply_tree_merge(&l, &r, &hostile, &out).unwrap();
    assert_eq!(report.written, 0);
    assert_eq!(report.failed.len(), 1);
    assert_eq!(
        report
            .failed
            .first()
            .map(|(p, reason)| (p.as_str(), reason.as_str())),
        Some(("only_link.txt", "symlink; not followed"))
    );
    assert_eq!(
        std::fs::read_to_string(l.join("target.txt")).unwrap(),
        "target\n"
    );

    cleanup(&base);
}

#[test]
fn directories_on_either_side_become_make_dir_steps() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    // A dir only on the left, empty: survives as MakeDir.
    std::fs::create_dir_all(l.join("newdir")).expect("failed to create the dir");
    // A dir on both sides: one MakeDir.
    std::fs::create_dir_all(l.join("bothdir")).unwrap();
    std::fs::create_dir_all(r.join("bothdir")).unwrap();
    // A dir only in base: no step, just an omission record.
    std::fs::create_dir_all(b.join("basedir")).unwrap();
    // Its file is deleted on both sides.
    write(&b.join("basedir/old.txt"), "old\n");
    // A file inside a left-only dir: the dir is planned before it.
    write(&l.join("sub/inner.txt"), "inner\n");

    let plan = plan_tree_merge(&b, &l, &r).unwrap();
    assert_eq!(action(&plan, "newdir"), Some(TreeMergeAction::MakeDir));
    assert_eq!(action(&plan, "bothdir"), Some(TreeMergeAction::MakeDir));
    assert!(step(&plan, "basedir").is_none());
    assert_eq!(skipped(&plan, "basedir"), Some("deleted on both sides"));
    assert_eq!(
        skipped(&plan, "basedir/old.txt"),
        Some("deleted on both sides")
    );
    assert_eq!(
        action(&plan, "sub/inner.txt"),
        Some(TreeMergeAction::TakeLeft)
    );

    // Apply: empty dirs exist in the output and stay empty.
    let out = base.join("out");
    let report = apply_tree_merge(&l, &r, &plan, &out).unwrap();
    assert!(out.join("newdir").is_dir());
    assert_eq!(std::fs::read_dir(out.join("newdir")).unwrap().count(), 0);
    assert!(out.join("bothdir").is_dir());
    assert!(out.join("sub/inner.txt").is_file());
    assert_eq!(report.dirs_created, 3); // newdir, bothdir, sub
    assert_eq!(report.written, 1);
    assert!(report.failed.is_empty());

    cleanup(&base);
}

#[test]
fn steps_are_sorted_with_directories_before_children() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    write(&l.join("b.txt"), "b\n");
    write(&l.join("a/x.txt"), "x\n");
    write(&r.join("a/y.txt"), "y\n");
    write(&r.join("z.txt"), "z\n");

    let plan = plan_tree_merge(&b, &l, &r).unwrap();
    let rels: Vec<&str> = plan.steps.iter().map(|s| s.rel_path.as_str()).collect();
    assert_eq!(rels, ["a", "a/x.txt", "a/y.txt", "b.txt", "z.txt"]);
    assert_eq!(action(&plan, "a"), Some(TreeMergeAction::MakeDir));
    assert_eq!(action(&plan, "a/x.txt"), Some(TreeMergeAction::TakeLeft));
    assert_eq!(action(&plan, "a/y.txt"), Some(TreeMergeAction::TakeRight));
    assert!(plan.conflicts.is_empty());
    assert!(plan.skipped.is_empty());

    cleanup(&base);
}

#[test]
fn missing_or_non_directory_roots_are_hard_errors() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    let missing = base.join("nowhere");
    assert!(matches!(
        plan_tree_merge(&missing, &l, &r),
        Err(Error::Missing { .. })
    ));
    assert!(matches!(
        plan_tree_merge(&b, &missing, &r),
        Err(Error::Missing { .. })
    ));

    let file = base.join("file.txt");
    write(&file, "not a dir\n");
    assert!(matches!(
        plan_tree_merge(&b, &l, &file),
        Err(Error::Missing { .. })
    ));

    cleanup(&base);
}

#[test]
fn apply_writes_the_plan_and_the_output_is_a_fixed_point() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    // A mixed tree: a one-sided edit, a clean merge, a shared new
    // file and a new directory with a file.
    write(&b.join("left_edit.txt"), "a\n");
    write(&l.join("left_edit.txt"), "b\n");
    write(&r.join("left_edit.txt"), "a\n");

    write(&b.join("doc.txt"), "a\nb\nc\n");
    write(&l.join("doc.txt"), "A\nb\nc\n");
    write(&r.join("doc.txt"), "a\nb\nC\n");

    write(&l.join("shared_new.txt"), "new\n");
    write(&r.join("shared_new.txt"), "new\n");

    write(&l.join("sub/inner.txt"), "inner\n");

    let plan = plan_tree_merge(&b, &l, &r).unwrap();
    let out = base.join("out");
    let report = apply_tree_merge(&l, &r, &plan, &out).unwrap();
    assert!(report.failed.is_empty());
    assert_eq!(report.dirs_created, 1); // sub
    assert_eq!(report.written, 4);
    assert_eq!(
        std::fs::read_to_string(out.join("left_edit.txt")).unwrap(),
        "b\n"
    );
    assert_eq!(
        std::fs::read_to_string(out.join("doc.txt")).unwrap(),
        "A\nb\nC\n"
    );
    assert_eq!(
        std::fs::read_to_string(out.join("shared_new.txt")).unwrap(),
        "new\n"
    );
    assert_eq!(
        std::fs::read_to_string(out.join("sub/inner.txt")).unwrap(),
        "inner\n"
    );

    // Convergence: the merged output, planned against itself over the
    // same base, is only directory creations and TakeLeft copies —
    // no conflicts, no skips.
    let again = plan_tree_merge(&b, &out, &out).unwrap();
    assert!(again.conflicts.is_empty());
    assert!(again.skipped.is_empty());
    let mut copies = 0usize;
    for s in &again.steps {
        if s.action == TreeMergeAction::TakeLeft {
            assert!(s.merged.is_none());
            copies += 1;
        } else {
            assert_eq!(s.action, TreeMergeAction::MakeDir);
        }
    }
    assert_eq!(copies, 4);

    // And applying that plan reproduces the merged tree.
    let out2 = base.join("out2");
    let report2 = apply_tree_merge(&out, &out, &again, &out2).unwrap();
    assert!(report2.failed.is_empty());
    assert_eq!(report2.written, 4);
    assert_eq!(
        std::fs::read_to_string(out2.join("doc.txt")).unwrap(),
        "A\nb\nC\n"
    );

    cleanup(&base);
}

#[test]
fn apply_refuses_non_empty_output_and_accepts_empty() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);
    write(&l.join("f.txt"), "f\n");

    let plan = plan_tree_merge(&b, &l, &r).unwrap();

    // A non-empty directory is refused, and its content is untouched.
    let busy = base.join("busy");
    std::fs::create_dir_all(&busy).unwrap();
    write(&busy.join("keep.txt"), "keep\n");
    assert!(matches!(
        apply_tree_merge(&l, &r, &plan, &busy),
        Err(Error::Unsupported { .. })
    ));
    assert_eq!(
        std::fs::read_to_string(busy.join("keep.txt")).unwrap(),
        "keep\n"
    );

    // A plain file is refused too.
    let file_out = base.join("file_out");
    write(&file_out, "nope\n");
    assert!(matches!(
        apply_tree_merge(&l, &r, &plan, &file_out),
        Err(Error::Unsupported { .. })
    ));

    // An existing empty directory works.
    let empty = base.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let report = apply_tree_merge(&l, &r, &plan, &empty).unwrap();
    assert!(report.failed.is_empty());
    assert!(empty.join("f.txt").is_file());

    cleanup(&base);
}

#[test]
fn unsafe_relative_paths_fail_without_escaping() {
    let base = temp_root();
    let (_, l, r) = make_roots(&base);
    write(&l.join("ok.txt"), "ok\n");

    // The canary sits where "../evil.txt" from the output would land.
    let canary = base.join("evil.txt");
    write(&canary, "canary\n");

    let plan = TreeMergePlan {
        steps: vec![
            TreeMergeStep {
                rel_path: "ok.txt".to_owned(),
                action: TreeMergeAction::TakeLeft,
                merged: None,
            },
            TreeMergeStep {
                rel_path: "../evil.txt".to_owned(),
                action: TreeMergeAction::TakeLeft,
                merged: None,
            },
            TreeMergeStep {
                rel_path: "sub/../../sneak.txt".to_owned(),
                action: TreeMergeAction::MergedText,
                merged: Some(vec!["no".to_owned()]),
            },
        ],
        conflicts: Vec::new(),
        skipped: Vec::new(),
    };

    let out = base.join("out");
    let report = apply_tree_merge(&l, &r, &plan, &out).unwrap();
    assert_eq!(report.written, 1);
    assert_eq!(report.dirs_created, 0);
    assert_eq!(report.failed.len(), 2);
    assert_eq!(
        report
            .failed
            .first()
            .map(|(p, reason)| (p.as_str(), reason.as_str())),
        Some(("../evil.txt", "unsafe path"))
    );
    assert!(
        report
            .failed
            .iter()
            .any(|(p, reason)| p == "sub/../../sneak.txt" && reason == "unsafe path")
    );

    // The canary is untouched and nothing exists outside the output.
    assert_eq!(std::fs::read_to_string(&canary).unwrap(), "canary\n");
    assert!(!base.join("sneak.txt").exists());
    assert!(out.join("ok.txt").is_file());

    cleanup(&base);
}

#[test]
fn a_vanished_source_fails_one_step_and_the_rest_still_apply() {
    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    write(&b.join("a.txt"), "a\n");
    write(&l.join("a.txt"), "changed-left\n");
    write(&r.join("a.txt"), "a\n");
    write(&l.join("b.txt"), "b-left\n");
    write(&r.join("c.txt"), "c-right\n");

    let plan = plan_tree_merge(&b, &l, &r).unwrap();
    assert_eq!(action(&plan, "a.txt"), Some(TreeMergeAction::TakeLeft));
    assert_eq!(action(&plan, "b.txt"), Some(TreeMergeAction::TakeLeft));
    assert_eq!(action(&plan, "c.txt"), Some(TreeMergeAction::TakeRight));

    // The left copy of a.txt vanishes between planning and applying.
    std::fs::remove_file(l.join("a.txt")).expect("failed to remove the source");

    let out = base.join("out");
    let report = apply_tree_merge(&l, &r, &plan, &out).unwrap();
    assert_eq!(report.failed.len(), 1);
    assert_eq!(
        report
            .failed
            .first()
            .map(|(p, reason)| (p.as_str(), reason.as_str())),
        Some(("a.txt", "source vanished"))
    );
    assert_eq!(report.written, 2);
    assert!(out.join("b.txt").is_file());
    assert!(out.join("c.txt").is_file());
    assert!(!out.join("a.txt").exists());

    cleanup(&base);
}

#[test]
fn folder_merge_struct_api() {
    use autocompare_compare::FolderMerge;

    let base = temp_root();
    let (b, l, r) = make_roots(&base);
    write(&b.join("merged.txt"), "line1\nline2\nline3\n");
    write(&l.join("merged.txt"), "line1_left\nline2\nline3\n");
    write(&r.join("merged.txt"), "line1\nline2\nline3_right\n");

    let merger = FolderMerge::new(&b, &l, &r);
    assert_eq!(merger.base(), b);
    assert_eq!(merger.left(), l);
    assert_eq!(merger.right(), r);

    let plan = merger.plan().unwrap();
    assert_eq!(plan.conflicts.len(), 0);
    assert_eq!(
        action(&plan, "merged.txt"),
        Some(TreeMergeAction::MergedText)
    );

    let out = base.join("out");
    let report = merger.apply(&plan, &out).unwrap();
    assert_eq!(report.failed.len(), 0);
    assert_eq!(report.written, 1);

    let content = std::fs::read_to_string(out.join("merged.txt")).unwrap();
    assert_eq!(content, "line1_left\nline2\nline3_right\n");

    cleanup(&base);
}

#[test]
fn folder_merge_conflict_navigation_and_cancellation() {
    use autocompare_compare::FolderMerge;
    use std::sync::atomic::AtomicBool;

    let base = temp_root();
    let (b, l, r) = make_roots(&base);

    // Conflict 1: overlapping text edits
    write(&b.join("conflict1.txt"), "common\n");
    write(&l.join("conflict1.txt"), "left edit\n");
    write(&r.join("conflict1.txt"), "right edit\n");

    // Conflict 2: binary conflict (bytes differ, neither matches base)
    write_bytes(&b.join("conflict2.bin"), b"\x00\x01\x02");
    write_bytes(&l.join("conflict2.bin"), b"\x10\x11\x12");
    write_bytes(&r.join("conflict2.bin"), b"\x20\x21\x22");

    // Clean merge file
    write(&b.join("clean.txt"), "start\nmiddle\nend\n");
    write(&l.join("clean.txt"), "start_L\nmiddle\nend\n");
    write(&r.join("clean.txt"), "start\nmiddle\nend_R\n");

    let merger = FolderMerge::new(&b, &l, &r);
    let plan = merger.dry_run().unwrap();

    // Conflict navigation
    assert!(plan.has_conflicts());
    assert_eq!(plan.conflict_count(), 2);
    assert!(plan.is_conflict("conflict1.txt"));
    assert!(plan.is_conflict("conflict2.bin"));
    assert!(!plan.is_conflict("clean.txt"));

    let first = plan.first_conflict();
    assert_eq!(first, Some("conflict1.txt"));
    let last = plan.last_conflict();
    assert_eq!(last, Some("conflict2.bin"));

    assert_eq!(plan.next_conflict(None), Some("conflict1.txt"));
    assert_eq!(
        plan.next_conflict(Some("conflict1.txt")),
        Some("conflict2.bin")
    );
    assert_eq!(plan.next_conflict(Some("conflict2.bin")), None);

    assert_eq!(plan.prev_conflict(None), Some("conflict2.bin"));
    assert_eq!(
        plan.prev_conflict(Some("conflict2.bin")),
        Some("conflict1.txt")
    );
    assert_eq!(plan.prev_conflict(Some("conflict1.txt")), None);

    // Summary and counters
    assert_eq!(plan.conflict_step_count(), 2);
    assert_eq!(plan.merged_text_count(), 1);
    assert!(plan.summary().contains("TreeMergePlan:"));

    // Apply with merger.run
    let out = base.join("out_run");
    let report = merger.run(&out).unwrap();
    assert!(report.is_success());
    assert_eq!(report.failure_count(), 0);
    assert_eq!(report.total_processed(), 3);
    assert!(report.summary().contains("TreeMergeReport:"));

    // Cancellation
    let out2 = base.join("out_cancel");
    let cancel = AtomicBool::new(true);
    let cancel_res = merger.run_cancellable(&out2, &cancel);
    assert!(matches!(cancel_res, Err(Error::Cancelled)));

    cleanup(&base);
}
