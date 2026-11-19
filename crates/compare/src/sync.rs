//! Folder sync: plan building and applying.
//!
//! Documented choices for rare situations:
//!
//! * Planning is read-only apart from the comparison walk and a
//!   `symlink_metadata` probe of copy sources; applying performs the
//!   real filesystem mutations.
//! * Directories are never copied: copies happen per file and parent
//!   directories are created on demand at apply time. The only step
//!   a directory can produce is a prune delete of a one-sided
//!   directory on the losing side; its descendants are then not
//!   planned at all because `remove_dir_all` removes them with it.
//!   Any other directory entry produces no step and no skipped line.
//! * Symlinks are never followed: a copy whose source is a symlink is
//!   skipped at plan time, and applying refuses to copy through a
//!   symlink on either end. Deletes use `remove_file`, which removes
//!   the link itself. No step therefore reads, writes or destroys
//!   anything outside the two roots through a symlink.
//! * Applying never panics and never aborts the run: every step's
//!   failure is recorded in the report and the run continues. Only
//!   roots that are missing or not directories are hard errors.
//! * Step paths come from the trusted internal walk, but a plan is
//!   just an argument: each path is re-checked against the roots and
//!   anything that could name a path outside them (empty, `.` or
//!   `..` components, Windows-style separators, absolute paths) is
//!   refused as "unsafe path".
//! * The comparison entries are documented as sorted by relative
//!   path and one entry yields at most one step, so the steps come
//!   out sorted with parents before children; no re-sort is needed.

use std::path::{Path, PathBuf};

use crate::{
    Error, FolderEntry, FolderOptions, FolderStatus, Result, SyncAction, SyncDirection,
    SyncOptions, SyncPlan, SyncReport, SyncStep, compare_folders,
};

/// Builds a sync plan from a folder comparison. See the crate
/// documentation for the rules.
pub(crate) fn plan_sync(left: &Path, right: &Path, opts: &SyncOptions) -> Result<SyncPlan> {
    require_dir(left)?;
    require_dir(right)?;

    let comparison = compare_folders(left, right, &FolderOptions::default())?;
    let mut plan = SyncPlan {
        steps: Vec::new(),
        skipped: Vec::new(),
        truncated: comparison.truncated,
    };
    // One-sided directories that get a delete step: their descendants
    // are removed with them and are not planned at all.
    let mut doomed: Vec<String> = Vec::new();

    for entry in &comparison.entries {
        if doomed.iter().any(|dir| is_under(&entry.rel_path, dir)) {
            continue;
        }
        if entry.is_dir {
            plan_dir(entry, opts, &mut plan, &mut doomed);
        } else {
            plan_file(entry, left, right, opts, &mut plan);
        }
    }

    Ok(plan)
}

/// Applies a plan. See the crate documentation for the guarantees.
pub(crate) fn apply_sync(left: &Path, right: &Path, plan: &SyncPlan) -> Result<SyncReport> {
    require_dir(left)?;
    require_dir(right)?;

    let mut report = SyncReport::default();
    for step in &plan.steps {
        match step.action {
            SyncAction::CopyToRight => apply_copy(left, right, step, &mut report),
            SyncAction::CopyToLeft => apply_copy(right, left, step, &mut report),
            SyncAction::DeleteRight => apply_delete(right, step, &mut report),
            SyncAction::DeleteLeft => apply_delete(left, step, &mut report),
        }
    }
    Ok(report)
}

/// Decides one directory entry. Directories are never copied; the
/// only step they produce is a prune delete when one-sided on the
/// losing side of a one-directional sync. Everything else (same,
/// newer, or a mixed-kind pair) produces no step and no skipped
/// line: the conflict, if any, shows up on the children instead.
fn plan_dir(
    entry: &FolderEntry,
    opts: &SyncOptions,
    plan: &mut SyncPlan,
    doomed: &mut Vec<String>,
) {
    if !opts.prune {
        return;
    }
    let action = match (entry.status, opts.direction) {
        (FolderStatus::LeftOnly, SyncDirection::RightToLeft) => Some(SyncAction::DeleteLeft),
        (FolderStatus::RightOnly, SyncDirection::LeftToRight) => Some(SyncAction::DeleteRight),
        _ => None,
    };
    if let Some(action) = action {
        plan.steps.push(SyncStep {
            rel_path: entry.rel_path.clone(),
            is_dir: true,
            action,
        });
        doomed.push(entry.rel_path.clone());
    }
}

/// Decides one non-directory entry by status, direction and prune,
/// following the rules in the crate documentation.
fn plan_file(
    entry: &FolderEntry,
    left: &Path,
    right: &Path,
    opts: &SyncOptions,
    plan: &mut SyncPlan,
) {
    match (entry.status, opts.direction) {
        (FolderStatus::Same, _) => {}
        (FolderStatus::Different, _) => {
            plan.skipped.push((
                entry.rel_path.clone(),
                "content differs; no side wins".to_owned(),
            ));
        }
        (FolderStatus::NewerLeft, SyncDirection::RightToLeft)
        | (FolderStatus::NewerRight, SyncDirection::LeftToRight) => {
            plan.skipped
                .push((entry.rel_path.clone(), "destination is newer".to_owned()));
        }
        (FolderStatus::LeftOnly, SyncDirection::RightToLeft) => {
            if opts.prune {
                plan.steps.push(SyncStep {
                    rel_path: entry.rel_path.clone(),
                    is_dir: false,
                    action: SyncAction::DeleteLeft,
                });
            } else {
                plan.skipped
                    .push((entry.rel_path.clone(), "only on the left side".to_owned()));
            }
        }
        (FolderStatus::RightOnly, SyncDirection::LeftToRight) => {
            if opts.prune {
                plan.steps.push(SyncStep {
                    rel_path: entry.rel_path.clone(),
                    is_dir: false,
                    action: SyncAction::DeleteRight,
                });
            } else {
                plan.skipped
                    .push((entry.rel_path.clone(), "only on the right side".to_owned()));
            }
        }
        (FolderStatus::NewerLeft, _) => plan_copy(entry, left, SyncAction::CopyToRight, plan),
        (FolderStatus::NewerRight, _) => plan_copy(entry, right, SyncAction::CopyToLeft, plan),
        (FolderStatus::LeftOnly, _) => plan_copy(entry, left, SyncAction::CopyToRight, plan),
        (FolderStatus::RightOnly, _) => plan_copy(entry, right, SyncAction::CopyToLeft, plan),
    }
}

/// Plans a copy step unless the source is a symlink: the walk never
/// follows symlinks, so the plan never copies through one. A source
/// that vanished since the walk keeps its step; apply reports it.
fn plan_copy(entry: &FolderEntry, source_root: &Path, action: SyncAction, plan: &mut SyncPlan) {
    let source = source_root.join(&entry.rel_path);
    let is_symlink = std::fs::symlink_metadata(&source)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false);
    if is_symlink {
        plan.skipped
            .push((entry.rel_path.clone(), "symlink; not followed".to_owned()));
    } else {
        plan.steps.push(SyncStep {
            rel_path: entry.rel_path.clone(),
            is_dir: false,
            action,
        });
    }
}

/// Applies one copy step: refuses unsafe paths and symlinks, creates
/// the destination's parent directories on demand, then copies the
/// bytes. Failures are recorded and the run continues.
fn apply_copy(source_root: &Path, dest_root: &Path, step: &SyncStep, report: &mut SyncReport) {
    let (Some(source), Some(dest)) = (
        under(source_root, &step.rel_path),
        under(dest_root, &step.rel_path),
    ) else {
        fail_step(report, step, "unsafe path");
        return;
    };
    match std::fs::symlink_metadata(&source) {
        Ok(meta) if meta.file_type().is_symlink() => {
            fail_step(report, step, "symlink; not followed");
            return;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fail_step(report, step, "source vanished");
            return;
        }
        Err(e) => {
            fail_step(report, step, &e.to_string());
            return;
        }
        Ok(_) => {}
    }
    // A symlink destination would make the copy write through to
    // wherever it points, possibly outside the roots.
    if let Ok(meta) = std::fs::symlink_metadata(&dest)
        && meta.file_type().is_symlink()
    {
        fail_step(report, step, "symlink; not followed");
        return;
    }
    let Some(parent) = dest.parent() else {
        fail_step(report, step, "unsafe path");
        return;
    };
    if let Err(e) = std::fs::create_dir_all(parent) {
        fail_step(report, step, &e.to_string());
        return;
    }
    match std::fs::copy(&source, &dest) {
        Ok(_) => report.copied = report.copied.saturating_add(1),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fail_step(report, step, "source vanished");
        }
        Err(e) => fail_step(report, step, &e.to_string()),
    }
}

/// Applies one delete step: `remove_dir_all` for directories,
/// `remove_file` otherwise (which also removes a symlink without
/// following it). Missing targets and io failures are recorded and
/// the run continues.
fn apply_delete(root: &Path, step: &SyncStep, report: &mut SyncReport) {
    let Some(target) = under(root, &step.rel_path) else {
        fail_step(report, step, "unsafe path");
        return;
    };
    let result = if step.is_dir {
        std::fs::remove_dir_all(&target)
    } else {
        std::fs::remove_file(&target)
    };
    match result {
        Ok(()) => report.deleted = report.deleted.saturating_add(1),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fail_step(report, step, "already gone");
        }
        Err(e) => fail_step(report, step, &e.to_string()),
    }
}

/// Records one failed step with a plain-English reason.
fn fail_step(report: &mut SyncReport, step: &SyncStep, reason: &str) {
    report
        .failed
        .push((step.rel_path.clone(), reason.to_owned()));
}

/// Verifies that `path` exists and is a directory, mirroring the
/// root check in [`crate::folder`] (which keeps its own copy
/// private).
fn require_dir(path: &Path) -> Result<()> {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(Error::Missing {
            path: path.display().to_string(),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::Missing {
            path: path.display().to_string(),
        }),
        Err(e) => Err(Error::Io {
            path: path.display().to_string(),
            source: e,
        }),
    }
}

/// Joins `rel` under `root`, refusing anything that could name a
/// path outside it: empty, `.` or `..` components, Windows-style
/// backslash separators, and paths whose join escapes the root (an
/// absolute `rel` replaces it). The walk only ever produces plain
/// '/'-separated names, so legitimate steps pass untouched.
fn under(root: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() {
        return None;
    }
    for component in rel.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return None;
        }
        if cfg!(windows) && component.contains('\\') {
            return None;
        }
    }
    let joined = root.join(rel);
    joined.starts_with(root).then_some(joined)
}

/// Whether `rel` names something strictly inside the directory
/// `dir_rel` (a child, never the directory itself).
fn is_under(rel: &str, dir_rel: &str) -> bool {
    let prefix = format!("{dir_rel}/");
    rel.starts_with(&prefix)
}

/// High-level folder synchronization modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SyncMode {
    /// Update right target with newer and left-only files from left (no pruning).
    UpdateRight,
    /// Update left target with newer and right-only files from right (no pruning).
    UpdateLeft,
    /// Bi-directional update: newer files win on both sides (no pruning).
    Both,
    /// Mirror left to right: copy newer/left-only files and delete files on right that do not exist on left.
    Mirror,
}

/// A high-level folder synchronization planner and executor.
#[derive(Debug, Clone)]
pub struct SyncPlanner {
    left: PathBuf,
    right: PathBuf,
    mode: SyncMode,
}

impl SyncPlanner {
    /// Creates a new `SyncPlanner` for the given roots and sync mode.
    pub fn new(left: impl AsRef<Path>, right: impl AsRef<Path>, mode: SyncMode) -> Self {
        Self {
            left: left.as_ref().to_path_buf(),
            right: right.as_ref().to_path_buf(),
            mode,
        }
    }

    /// Converts the `SyncMode` into underlying [`SyncOptions`].
    pub fn options(&self) -> SyncOptions {
        match self.mode {
            SyncMode::UpdateRight => SyncOptions {
                direction: SyncDirection::LeftToRight,
                prune: false,
            },
            SyncMode::UpdateLeft => SyncOptions {
                direction: SyncDirection::RightToLeft,
                prune: false,
            },
            SyncMode::Both => SyncOptions {
                direction: SyncDirection::NewerWins,
                prune: false,
            },
            SyncMode::Mirror => SyncOptions {
                direction: SyncDirection::LeftToRight,
                prune: true,
            },
        }
    }

    /// Generates the sync plan without modifying the filesystem (dry-run).
    pub fn dry_run(&self) -> Result<SyncPlan> {
        plan_sync(&self.left, &self.right, &self.options())
    }

    /// Generates the sync plan. Equivalent to [`Self::dry_run`].
    pub fn plan(&self) -> Result<SyncPlan> {
        self.dry_run()
    }

    /// Applies the planned synchronization steps to the filesystem.
    pub fn apply(&self, plan: &SyncPlan) -> Result<SyncReport> {
        apply_sync(&self.left, &self.right, plan)
    }

    /// Left root directory.
    pub fn left(&self) -> &Path {
        &self.left
    }

    /// Right root directory.
    pub fn right(&self) -> &Path {
        &self.right
    }

    /// Current sync mode.
    pub fn mode(&self) -> SyncMode {
        self.mode
    }
}
