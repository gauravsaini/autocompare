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
