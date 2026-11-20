//! Media, image, sync and folder merge report adapters.
//!
//! Converts media, picture, sync and folder merge outcomes to ReportInput.

use autocompare_compare::{
    AudioComparison, PictureComparison, SyncPlan, TreeMergeAction, TreeMergePlan,
};
use autocompare_reports::{DiffStats, Hunk, HunkLine, ReportInput};

/// Converts sync plan outcome to ReportInput.
pub fn sync_report_input(left: &str, right: &str, plan: &SyncPlan) -> ReportInput {
    let mut lines = Vec::new();
    let mut modifications = 0usize;
    let mut deletions = 0usize;
    let mut unchanged = 0usize;

    for (i, step) in plan.steps.iter().enumerate() {
        let no = i.saturating_add(1);
        match step.action {
            autocompare_compare::SyncAction::CopyToRight
            | autocompare_compare::SyncAction::CopyToLeft => {
                lines.push(HunkLine::modify(
                    format!("copy {:?} (dir={})", step.rel_path, step.is_dir),
                    no,
                    no,
                ));
                modifications = modifications.saturating_add(1);
            }
            autocompare_compare::SyncAction::DeleteRight
            | autocompare_compare::SyncAction::DeleteLeft => {
                lines.push(HunkLine::delete(
                    format!("delete {:?} (dir={})", step.rel_path, step.is_dir),
                    no,
                ));
                deletions = deletions.saturating_add(1);
            }
        }
    }

    for (path, reason) in &plan.skipped {
        lines.push(HunkLine::unchanged(
            format!("skip {path}: {reason}"),
            lines.len().saturating_add(1),
            lines.len().saturating_add(1),
        ));
        unchanged = unchanged.saturating_add(1);
    }

    let stats = DiffStats::new(0, deletions, modifications, unchanged);
    let hunk = Hunk::new(1, lines.len(), 1, lines.len(), lines);
    ReportInput::new(format!("sync: {left} -> {right}"), vec![hunk], stats)
}

/// Converts picture comparison outcome to ReportInput.
pub fn picture_report_input(left: &str, right: &str, cmp: &PictureComparison) -> ReportInput {
    let mut lines = Vec::new();
    let (modifications, unchanged) = if cmp.diff.identical {
        lines.push(HunkLine::unchanged("Images are identical", 1, 1));
        (0, 1)
    } else {
        lines.push(HunkLine::modify(
            format!(
                "Differing pixels: {} / {}",
                cmp.diff.differing, cmp.diff.total
            ),
            1,
            1,
        ));
        (1, 0)
    };
    let stats = DiffStats::new(0, 0, modifications, unchanged);
    let hunk = Hunk::new(1, 1, 1, 1, lines);
    ReportInput::new(format!("picture: {left} <-> {right}"), vec![hunk], stats)
}

/// Converts tree merge plan outcome to ReportInput.
pub fn foldermerge_report_input(
    base: &str,
    left: &str,
    right: &str,
    plan: &TreeMergePlan,
) -> ReportInput {
    let mut lines = Vec::new();
    let mut additions = 0usize;
    let mut modifications = 0usize;
    let mut unchanged = 0usize;

    for (i, step) in plan.steps.iter().enumerate() {
        let no = i.saturating_add(1);
        match step.action {
            TreeMergeAction::Conflict => {
                lines.push(HunkLine::modify(
                    format!("conflict: {}", step.rel_path),
                    no,
                    no,
                ));
                modifications = modifications.saturating_add(1);
            }
            TreeMergeAction::MergedText | TreeMergeAction::MakeDir => {
                lines.push(HunkLine::insert(format!("action: {}", step.rel_path), no));
                additions = additions.saturating_add(1);
            }
            TreeMergeAction::TakeLeft | TreeMergeAction::TakeRight => {
                lines.push(HunkLine::unchanged(
                    format!("take: {}", step.rel_path),
                    no,
                    no,
                ));
                unchanged = unchanged.saturating_add(1);
            }
        }
    }

    let stats = DiffStats::new(additions, 0, modifications, unchanged);
    let hunk = Hunk::new(1, lines.len(), 1, lines.len(), lines);
    ReportInput::new(
        format!("folder-merge: {base} + {left} + {right}"),
        vec![hunk],
        stats,
    )
}

/// Converts audio comparison outcome to ReportInput.
pub fn audio_report_input(left: &str, right: &str, cmp: &AudioComparison) -> ReportInput {
    let mut lines = Vec::new();
    let (modifications, unchanged) = if cmp.diff.identical {
        lines.push(HunkLine::unchanged("Audio samples identical", 1, 1));
        (0, 1)
    } else {
        lines.push(HunkLine::modify(
            format!(
                "Audio difference: {} / {} windows differ",
                cmp.diff.windows_differing, cmp.diff.windows_total
            ),
            1,
            1,
        ));
        (1, 0)
    };
    let stats = DiffStats::new(0, 0, modifications, unchanged);
    let hunk = Hunk::new(1, 1, 1, 1, lines);
    ReportInput::new(format!("audio: {left} <-> {right}"), vec![hunk], stats)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use autocompare_compare::{SyncAction, SyncStep, TreeMergeAction, TreeMergeStep};
    use autocompare_diff::{AudioDiff, PictureDiff};

    #[test]
    fn test_sync_report_input_steps_and_skipped() {
        let plan = SyncPlan {
            steps: vec![
                SyncStep {
                    rel_path: "copied.txt".into(),
                    is_dir: false,
                    action: SyncAction::CopyToRight,
                },
                SyncStep {
                    rel_path: "deleted.txt".into(),
                    is_dir: false,
                    action: SyncAction::DeleteLeft,
                },
            ],
            skipped: vec![("skipped.bin".into(), "newer at dest".into())],
            truncated: false,
        };

        let rep = sync_report_input("left_sync", "right_sync", &plan);
        assert_eq!(rep.title, "sync: left_sync -> right_sync");
        assert_eq!(rep.stats.modifications, 1);
        assert_eq!(rep.stats.deletions, 1);
        assert_eq!(rep.stats.unchanged, 1);
    }

    #[test]
    fn test_picture_report_input_identical_and_differing() {
        let cmp_same = PictureComparison {
            diff: PictureDiff {
                identical: true,
                width_l: 100,
                height_l: 100,
                width_r: 100,
                height_r: 100,
                overlap_w: 100,
                overlap_h: 100,
                total: 10000,
                differing: 0,
                bbox: None,
            },
        };
        let rep_same = picture_report_input("p1.png", "p2.png", &cmp_same);
        assert_eq!(rep_same.stats.unchanged, 1);
        assert_eq!(rep_same.stats.modifications, 0);

        let cmp_diff = PictureComparison {
            diff: PictureDiff {
                identical: false,
                width_l: 100,
                height_l: 100,
                width_r: 100,
                height_r: 100,
                overlap_w: 100,
                overlap_h: 100,
                total: 10000,
                differing: 50,
                bbox: Some((10, 10, 20, 20)),
            },
        };
        let rep_diff = picture_report_input("p1.png", "p2.png", &cmp_diff);
        assert_eq!(rep_diff.stats.unchanged, 0);
        assert_eq!(rep_diff.stats.modifications, 1);
    }

    #[test]
    fn test_foldermerge_report_input_all_actions() {
        let plan = TreeMergePlan {
            steps: vec![
                TreeMergeStep {
                    rel_path: "conflict.txt".into(),
                    action: TreeMergeAction::Conflict,
                    merged: None,
                },
                TreeMergeStep {
                    rel_path: "merged.txt".into(),
                    action: TreeMergeAction::MergedText,
                    merged: Some(vec!["merged line".into()]),
                },
                TreeMergeStep {
                    rel_path: "take_left.txt".into(),
                    action: TreeMergeAction::TakeLeft,
                    merged: None,
                },
            ],
            conflicts: vec!["conflict.txt".into()],
            skipped: Vec::new(),
        };

        let rep = foldermerge_report_input("base", "left", "right", &plan);
        assert_eq!(rep.stats.modifications, 1);
        assert_eq!(rep.stats.additions, 1);
        assert_eq!(rep.stats.unchanged, 1);
    }

    #[test]
    fn test_audio_report_input_identical_and_differing() {
        let cmp_same = AudioComparison {
            diff: AudioDiff {
                identical: true,
                duration_l: 1.0,
                duration_r: 1.0,
                sample_rate_l: 44100,
                sample_rate_r: 44100,
                channels_l: 2,
                channels_r: 2,
                config_mismatch: false,
                windows_total: 100,
                windows_differing: 0,
                first_diff_at: None,
            },
        };
        let rep_same = audio_report_input("a1.wav", "a2.wav", &cmp_same);
        assert_eq!(rep_same.stats.unchanged, 1);
        assert_eq!(rep_same.stats.modifications, 0);

        let cmp_diff = AudioComparison {
            diff: AudioDiff {
                identical: false,
                duration_l: 1.0,
                duration_r: 1.0,
                sample_rate_l: 44100,
                sample_rate_r: 44100,
                channels_l: 2,
                channels_r: 2,
                config_mismatch: false,
                windows_total: 100,
                windows_differing: 10,
                first_diff_at: Some(0.5),
            },
        };
        let rep_diff = audio_report_input("a1.wav", "a2.wav", &cmp_diff);
        assert_eq!(rep_diff.stats.unchanged, 0);
        assert_eq!(rep_diff.stats.modifications, 1);
    }
}
