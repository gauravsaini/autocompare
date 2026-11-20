//! Folder sync: plan building and applying.

#![forbid(unsafe_code)]
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
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    Error, FolderComparison, FolderEntry, FolderOptions, FolderStatus, Result, SyncAction,
    SyncDirection, SyncOptions, SyncPlan, SyncReport, SyncStep, compare_folders,
    compare_folders_cancellable,
};

/// Builds a sync plan from a folder comparison. See the crate
/// documentation for the rules.
pub(crate) fn plan_sync(left: &Path, right: &Path, opts: &SyncOptions) -> Result<SyncPlan> {
    plan_sync_cancellable(left, right, opts, None)
}

/// Builds a sync plan with explicit folder comparison rules (depth
/// limit, content compare, case folding, include/exclude filters).
/// Read-only like [`plan_sync`]: the comparison walk is the only
/// filesystem access. Truncation and filtering flow into the plan:
/// entries below the depth cut are not planned (`truncated` is
/// set) and filtered-out names appear in neither steps nor skips.
pub fn plan_sync_with_compare(
    left: &Path,
    right: &Path,
    opts: &SyncOptions,
    compare: &FolderOptions,
) -> Result<SyncPlan> {
    plan_sync_with_compare_cancellable(left, right, opts, compare, None)
}

/// Progress callback for sync application: `(current_index, total_steps, current_rel_path)`.
pub type SyncProgressCallback<'a> = &'a dyn Fn(usize, usize, &str);

/// Builds a sync plan with cancellation support.
pub fn plan_sync_cancellable(
    left: &Path,
    right: &Path,
    opts: &SyncOptions,
    cancel: Option<&AtomicBool>,
) -> Result<SyncPlan> {
    plan_sync_with_compare_cancellable(left, right, opts, &FolderOptions::default(), cancel)
}

/// Builds a sync plan with explicit comparison rules and
/// cancellation support.
pub fn plan_sync_with_compare_cancellable(
    left: &Path,
    right: &Path,
    opts: &SyncOptions,
    compare: &FolderOptions,
    cancel: Option<&AtomicBool>,
) -> Result<SyncPlan> {
    require_dir(left)?;
    require_dir(right)?;

    let comparison = match cancel {
        Some(c) => compare_folders_cancellable(left, right, compare, c)?,
        None => compare_folders(left, right, compare)?,
    };

    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }

    build_plan(&comparison, left, right, opts, cancel)
}

/// Turns a finished comparison into steps and skips, honouring
/// direction and prune. One-sided prune deletes suppress their
/// whole subtree. Cancellation is re-checked per entry.
fn build_plan(
    comparison: &FolderComparison,
    left: &Path,
    right: &Path,
    opts: &SyncOptions,
    cancel: Option<&AtomicBool>,
) -> Result<SyncPlan> {
    let mut plan = SyncPlan {
        steps: Vec::new(),
        skipped: Vec::new(),
        truncated: comparison.truncated,
    };
    // One-sided directories that get a delete step: their descendants
    // are removed with them and are not planned at all.
    let mut doomed: Vec<String> = Vec::new();

    for entry in &comparison.entries {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }
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
    apply_sync_cancellable(left, right, plan, None, None)
}

/// Applies a plan with cancellation and bounded progress reporting support.
pub fn apply_sync_cancellable(
    left: &Path,
    right: &Path,
    plan: &SyncPlan,
    cancel: Option<&AtomicBool>,
    progress: Option<SyncProgressCallback<'_>>,
) -> Result<SyncReport> {
    require_dir(left)?;
    require_dir(right)?;

    let mut report = SyncReport::default();
    let total = plan.steps.len();

    for (idx, step) in plan.steps.iter().enumerate() {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }

        if let Some(p) = progress {
            p(idx, total, &step.rel_path);
        }

        match step.action {
            SyncAction::CopyToRight => apply_copy(left, right, step, &mut report),
            SyncAction::CopyToLeft => apply_copy(right, left, step, &mut report),
            SyncAction::DeleteRight => apply_delete(right, step, &mut report),
            SyncAction::DeleteLeft => apply_delete(left, step, &mut report),
        }
    }

    if let Some(p) = progress {
        p(total, total, "complete");
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

/// Action to take for a file or directory under custom sync rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SyncRuleAction {
    /// Leave the item alone (skip).
    #[default]
    Skip,
    /// Copy from left to right.
    CopyLeftToRight,
    /// Copy from right to left.
    CopyRightToLeft,
    /// Delete the left copy.
    DeleteLeft,
    /// Delete the right copy.
    DeleteRight,
}

/// User-configurable rules mapping folder comparison statuses to synchronization actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct SyncCustomRules {
    /// Action when left file is newer than right.
    pub on_newer_left: SyncRuleAction,
    /// Action when right file is newer than left.
    pub on_newer_right: SyncRuleAction,
    /// Action when file exists only on left.
    pub on_left_only: SyncRuleAction,
    /// Action when file exists only on right.
    pub on_right_only: SyncRuleAction,
    /// Action when files differ in content or size.
    pub on_different: SyncRuleAction,
    /// Action when files are identical.
    pub on_same: SyncRuleAction,
}

impl SyncCustomRules {
    /// Creates a custom rule set mirroring left to right.
    pub fn mirror_left_to_right() -> Self {
        Self {
            on_newer_left: SyncRuleAction::CopyLeftToRight,
            on_newer_right: SyncRuleAction::Skip,
            on_left_only: SyncRuleAction::CopyLeftToRight,
            on_right_only: SyncRuleAction::DeleteRight,
            on_different: SyncRuleAction::Skip,
            on_same: SyncRuleAction::Skip,
        }
    }

    /// Creates a custom rule set mirroring right to left.
    pub fn mirror_right_to_left() -> Self {
        Self {
            on_newer_left: SyncRuleAction::Skip,
            on_newer_right: SyncRuleAction::CopyRightToLeft,
            on_left_only: SyncRuleAction::DeleteLeft,
            on_right_only: SyncRuleAction::CopyRightToLeft,
            on_different: SyncRuleAction::Skip,
            on_same: SyncRuleAction::Skip,
        }
    }

    /// Creates a custom rule set updating right from left without pruning.
    pub fn update_left_to_right() -> Self {
        Self {
            on_newer_left: SyncRuleAction::CopyLeftToRight,
            on_newer_right: SyncRuleAction::Skip,
            on_left_only: SyncRuleAction::CopyLeftToRight,
            on_right_only: SyncRuleAction::Skip,
            on_different: SyncRuleAction::Skip,
            on_same: SyncRuleAction::Skip,
        }
    }

    /// Creates a custom rule set updating left from right without pruning.
    pub fn update_right_to_left() -> Self {
        Self {
            on_newer_left: SyncRuleAction::Skip,
            on_newer_right: SyncRuleAction::CopyRightToLeft,
            on_left_only: SyncRuleAction::Skip,
            on_right_only: SyncRuleAction::CopyRightToLeft,
            on_different: SyncRuleAction::Skip,
            on_same: SyncRuleAction::Skip,
        }
    }

    /// Creates a custom rule set for bi-directional (two-way) sync.
    pub fn two_way_sync() -> Self {
        Self {
            on_newer_left: SyncRuleAction::CopyLeftToRight,
            on_newer_right: SyncRuleAction::CopyRightToLeft,
            on_left_only: SyncRuleAction::CopyLeftToRight,
            on_right_only: SyncRuleAction::CopyRightToLeft,
            on_different: SyncRuleAction::Skip,
            on_same: SyncRuleAction::Skip,
        }
    }
}

/// Builds a sync plan using custom rules mapping comparison status to sync actions.
pub fn plan_sync_with_rules_cancellable(
    left: &Path,
    right: &Path,
    rules: &SyncCustomRules,
    compare: &FolderOptions,
    cancel: Option<&AtomicBool>,
) -> Result<SyncPlan> {
    require_dir(left)?;
    require_dir(right)?;

    let comparison = match cancel {
        Some(c) => compare_folders_cancellable(left, right, compare, c)?,
        None => compare_folders(left, right, compare)?,
    };

    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }

    let mut plan = SyncPlan {
        steps: Vec::new(),
        skipped: Vec::new(),
        truncated: comparison.truncated,
    };
    let mut doomed: Vec<String> = Vec::new();

    for entry in &comparison.entries {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(Error::Cancelled);
        }
        if doomed.iter().any(|dir| is_under(&entry.rel_path, dir)) {
            continue;
        }

        let rule_action = match entry.status {
            FolderStatus::Same => rules.on_same,
            FolderStatus::Different => rules.on_different,
            FolderStatus::NewerLeft => rules.on_newer_left,
            FolderStatus::NewerRight => rules.on_newer_right,
            FolderStatus::LeftOnly => rules.on_left_only,
            FolderStatus::RightOnly => rules.on_right_only,
        };

        if entry.is_dir {
            match rule_action {
                SyncRuleAction::DeleteLeft => {
                    plan.steps.push(SyncStep {
                        rel_path: entry.rel_path.clone(),
                        is_dir: true,
                        action: SyncAction::DeleteLeft,
                    });
                    doomed.push(entry.rel_path.clone());
                }
                SyncRuleAction::DeleteRight => {
                    plan.steps.push(SyncStep {
                        rel_path: entry.rel_path.clone(),
                        is_dir: true,
                        action: SyncAction::DeleteRight,
                    });
                    doomed.push(entry.rel_path.clone());
                }
                _ => {}
            }
        } else {
            match rule_action {
                SyncRuleAction::CopyLeftToRight => {
                    plan_copy(entry, left, SyncAction::CopyToRight, &mut plan);
                }
                SyncRuleAction::CopyRightToLeft => {
                    plan_copy(entry, right, SyncAction::CopyToLeft, &mut plan);
                }
                SyncRuleAction::DeleteLeft => {
                    plan.steps.push(SyncStep {
                        rel_path: entry.rel_path.clone(),
                        is_dir: false,
                        action: SyncAction::DeleteLeft,
                    });
                }
                SyncRuleAction::DeleteRight => {
                    plan.steps.push(SyncStep {
                        rel_path: entry.rel_path.clone(),
                        is_dir: false,
                        action: SyncAction::DeleteRight,
                    });
                }
                SyncRuleAction::Skip => {
                    plan.skipped
                        .push((entry.rel_path.clone(), "skipped by custom rule".to_owned()));
                }
            }
        }
    }

    Ok(plan)
}

/// Prunes empty subdirectories under `root`, walking bottom-up so nested
/// empty directory structures are pruned cleanly. Symlinks are never followed
/// and the `root` directory itself is never removed.
pub fn prune_empty_directories(root: &Path, cancel: Option<&AtomicBool>) -> Result<usize> {
    require_dir(root)?;
    let mut pruned_count = 0;
    prune_empty_dirs_recurse(root, root, cancel, &mut pruned_count)?;
    Ok(pruned_count)
}

fn prune_empty_dirs_recurse(
    root: &Path,
    current: &Path,
    cancel: Option<&AtomicBool>,
    pruned: &mut usize,
) -> Result<bool> {
    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Err(Error::Cancelled);
    }
    let reader = match std::fs::read_dir(current) {
        Ok(r) => r,
        Err(e) => {
            return Err(Error::Io {
                path: current.display().to_string(),
                source: e,
            });
        }
    };

    let mut is_empty = true;
    for entry in reader {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                return Err(Error::Io {
                    path: current.display().to_string(),
                    source: e,
                });
            }
        };
        let path = entry.path();
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                return Err(Error::Io {
                    path: path.display().to_string(),
                    source: e,
                });
            }
        };

        if meta.file_type().is_symlink() {
            // Symlinks count as non-empty so they are never followed or removed here
            is_empty = false;
        } else if meta.is_dir() {
            let child_empty = prune_empty_dirs_recurse(root, &path, cancel, pruned)?;
            if !child_empty {
                is_empty = false;
            }
        } else {
            is_empty = false;
        }
    }

    // Only delete subdirectories, never the root itself
    if is_empty && current != root {
        match std::fs::remove_dir(current) {
            Ok(()) => {
                *pruned = pruned.saturating_add(1);
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    } else {
        Ok(is_empty)
    }
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
    /// Mirror right to left: copy newer/right-only files and delete files on left that do not exist on right.
    MirrorLeft,
    /// Two-way sync: bi-directional synchronization (newer files win on both sides, one-sided copied to other side).
    TwoWay,
    /// Custom rule-based synchronization.
    Custom,
}

/// A high-level folder synchronization planner and executor.
#[derive(Debug, Clone)]
pub struct SyncPlanner {
    left: PathBuf,
    right: PathBuf,
    mode: SyncMode,
    compare: FolderOptions,
    custom_rules: Option<SyncCustomRules>,
    prune_empty_dirs: bool,
}

impl SyncPlanner {
    /// Creates a new `SyncPlanner` for the given roots and sync mode.
    /// Comparison rules default to [`FolderOptions::default`]; use
    /// [`Self::with_compare_options`] to mirror a folder view with
    /// depth limits, content compare or name filters.
    pub fn new(left: impl AsRef<Path>, right: impl AsRef<Path>, mode: SyncMode) -> Self {
        Self {
            left: left.as_ref().to_path_buf(),
            right: right.as_ref().to_path_buf(),
            mode,
            compare: FolderOptions::default(),
            custom_rules: None,
            prune_empty_dirs: false,
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
            SyncMode::Both | SyncMode::TwoWay => SyncOptions {
                direction: SyncDirection::NewerWins,
                prune: false,
            },
            SyncMode::Mirror => SyncOptions {
                direction: SyncDirection::LeftToRight,
                prune: true,
            },
            SyncMode::MirrorLeft => SyncOptions {
                direction: SyncDirection::RightToLeft,
                prune: true,
            },
            SyncMode::Custom => SyncOptions {
                direction: SyncDirection::LeftToRight,
                prune: false,
            },
        }
    }

    /// Generates the sync plan without modifying the filesystem (dry-run).
    pub fn dry_run(&self) -> Result<SyncPlan> {
        if let Some(ref rules) = self.custom_rules {
            plan_sync_with_rules_cancellable(&self.left, &self.right, rules, &self.compare, None)
        } else if self.mode == SyncMode::Custom {
            let rules = SyncCustomRules::default();
            plan_sync_with_rules_cancellable(&self.left, &self.right, &rules, &self.compare, None)
        } else {
            plan_sync_with_compare(&self.left, &self.right, &self.options(), &self.compare)
        }
    }

    /// Generates the sync plan. Equivalent to [`Self::dry_run`].
    pub fn plan(&self) -> Result<SyncPlan> {
        self.dry_run()
    }

    /// Applies the planned synchronization steps to the filesystem.
    pub fn apply(&self, plan: &SyncPlan) -> Result<SyncReport> {
        let mut report = apply_sync(&self.left, &self.right, plan)?;
        if self.prune_empty_dirs {
            let target_root = match self.mode {
                SyncMode::UpdateLeft | SyncMode::MirrorLeft => Some(&self.left),
                SyncMode::UpdateRight | SyncMode::Mirror => Some(&self.right),
                SyncMode::Both | SyncMode::TwoWay | SyncMode::Custom => None,
            };
            if let Some(target) = target_root {
                let pruned = prune_empty_directories(target, None)?;
                report.deleted = report.deleted.saturating_add(pruned);
            } else {
                let pruned_l = prune_empty_directories(&self.left, None)?;
                let pruned_r = prune_empty_directories(&self.right, None)?;
                report.deleted = report
                    .deleted
                    .saturating_add(pruned_l)
                    .saturating_add(pruned_r);
            }
        }
        Ok(report)
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

    /// Updates the sync mode.
    pub fn set_mode(&mut self, mode: SyncMode) {
        self.mode = mode;
    }

    /// Returns a new `SyncPlanner` with the given mode.
    pub fn with_mode(mut self, mode: SyncMode) -> Self {
        self.mode = mode;
        self
    }

    /// Configures custom rules for synchronization.
    pub fn with_custom_rules(mut self, rules: SyncCustomRules) -> Self {
        self.custom_rules = Some(rules);
        self.mode = SyncMode::Custom;
        self
    }

    /// Sets or removes custom rules.
    pub fn set_custom_rules(&mut self, rules: Option<SyncCustomRules>) {
        if rules.is_some() {
            self.mode = SyncMode::Custom;
        }
        self.custom_rules = rules;
    }

    /// Returns a reference to the active custom rules, if any.
    pub fn custom_rules(&self) -> Option<&SyncCustomRules> {
        self.custom_rules.as_ref()
    }

    /// Configures whether empty directories should be pruned upon sync application.
    pub fn with_prune_empty_dirs(mut self, prune: bool) -> Self {
        self.prune_empty_dirs = prune;
        self
    }

    /// Updates whether empty directories should be pruned.
    pub fn set_prune_empty_dirs(&mut self, prune: bool) {
        self.prune_empty_dirs = prune;
    }

    /// Returns whether empty directories are pruned.
    pub fn prune_empty_dirs(&self) -> bool {
        self.prune_empty_dirs
    }

    /// Comparison rules this planner plans with (depth limit,
    /// content compare, case folding, name filters).
    pub fn compare_options(&self) -> &FolderOptions {
        &self.compare
    }

    /// Updates the comparison rules this planner plans with.
    pub fn set_compare_options(&mut self, compare: FolderOptions) {
        self.compare = compare;
    }

    /// Returns a new `SyncPlanner` with the given comparison rules.
    pub fn with_compare_options(mut self, compare: FolderOptions) -> Self {
        self.compare = compare;
        self
    }

    /// Returns whether this planner is in mirror mode.
    pub fn is_mirror(&self) -> bool {
        matches!(self.mode, SyncMode::Mirror | SyncMode::MirrorLeft)
    }

    /// Returns whether this planner is in bi-directional mode (Both or TwoWay).
    pub fn is_bidirectional(&self) -> bool {
        matches!(self.mode, SyncMode::Both | SyncMode::TwoWay)
    }

    /// Plans and applies the sync operation in one call.
    pub fn run(&self) -> Result<SyncReport> {
        let plan = self.plan()?;
        self.apply(&plan)
    }

    /// Runs planning and application with cancellation support.
    pub fn run_cancellable(&self, cancel: &AtomicBool) -> Result<SyncReport> {
        let plan = if let Some(ref rules) = self.custom_rules {
            plan_sync_with_rules_cancellable(
                &self.left,
                &self.right,
                rules,
                &self.compare,
                Some(cancel),
            )?
        } else if self.mode == SyncMode::Custom {
            let rules = SyncCustomRules::default();
            plan_sync_with_rules_cancellable(
                &self.left,
                &self.right,
                &rules,
                &self.compare,
                Some(cancel),
            )?
        } else {
            plan_sync_with_compare_cancellable(
                &self.left,
                &self.right,
                &self.options(),
                &self.compare,
                Some(cancel),
            )?
        };
        let mut report =
            apply_sync_cancellable(&self.left, &self.right, &plan, Some(cancel), None)?;
        if self.prune_empty_dirs {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let target_root = match self.mode {
                SyncMode::UpdateLeft | SyncMode::MirrorLeft => Some(&self.left),
                SyncMode::UpdateRight | SyncMode::Mirror => Some(&self.right),
                SyncMode::Both | SyncMode::TwoWay | SyncMode::Custom => None,
            };
            if let Some(target) = target_root {
                let pruned = prune_empty_directories(target, Some(cancel))?;
                report.deleted = report.deleted.saturating_add(pruned);
            } else {
                let pruned_l = prune_empty_directories(&self.left, Some(cancel))?;
                let pruned_r = prune_empty_directories(&self.right, Some(cancel))?;
                report.deleted = report
                    .deleted
                    .saturating_add(pruned_l)
                    .saturating_add(pruned_r);
            }
        }
        Ok(report)
    }

    /// Returns a human-readable summary of the dry run.
    pub fn dry_run_summary(&self, plan: &SyncPlan) -> String {
        let mode_name = match self.mode {
            SyncMode::UpdateRight => "Update Right",
            SyncMode::UpdateLeft => "Update Left",
            SyncMode::Both => "Update Both",
            SyncMode::Mirror => "Mirror Left to Right",
            SyncMode::MirrorLeft => "Mirror Right to Left",
            SyncMode::TwoWay => "Two-Way Sync",
            SyncMode::Custom => "Custom Sync",
        };
        format!("Dry-run {}: {}", mode_name, plan.summary())
    }

    /// Collects conflict paths from a plan.
    pub fn conflict_paths(&self, plan: &SyncPlan) -> Vec<String> {
        plan.conflict_paths()
            .into_iter()
            .map(String::from)
            .collect()
    }

    /// Navigates to the next conflict in the plan.
    pub fn next_conflict<'a>(
        &self,
        plan: &'a SyncPlan,
        current: Option<&str>,
    ) -> Option<&'a (String, String)> {
        plan.next_conflict(current)
    }

    /// Navigates to the previous conflict in the plan.
    pub fn prev_conflict<'a>(
        &self,
        plan: &'a SyncPlan,
        current: Option<&str>,
    ) -> Option<&'a (String, String)> {
        plan.prev_conflict(current)
    }
}

fn is_conflict_reason(reason: &str) -> bool {
    reason.contains("differs") || reason.contains("newer") || reason.contains("conflict")
}

impl SyncPlan {
    /// Returns the number of conflict entries skipped during planning.
    pub fn conflict_count(&self) -> usize {
        self.conflicts().len()
    }

    /// Whether any conflicts were detected during planning.
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts().is_empty()
    }

    /// Returns references to all skipped entries that represent conflicts.
    pub fn conflicts(&self) -> Vec<&(String, String)> {
        self.skipped
            .iter()
            .filter(|(_, reason)| is_conflict_reason(reason))
            .collect()
    }

    /// Returns relative paths of all conflicting entries.
    pub fn conflict_paths(&self) -> Vec<&str> {
        self.conflicts()
            .iter()
            .map(|(path, _)| path.as_str())
            .collect()
    }

    /// Returns the first conflict, if any.
    pub fn first_conflict(&self) -> Option<&(String, String)> {
        self.conflicts().first().copied()
    }

    /// Returns the last conflict, if any.
    pub fn last_conflict(&self) -> Option<&(String, String)> {
        self.conflicts().last().copied()
    }

    /// Navigates to the next conflict relative to `current_rel_path`.
    pub fn next_conflict<'a>(
        &'a self,
        current_rel_path: Option<&str>,
    ) -> Option<&'a (String, String)> {
        let conflicts = self.conflicts();
        let Some(curr) = current_rel_path else {
            return conflicts.first().copied();
        };
        for (idx, item) in conflicts.iter().enumerate() {
            if item.0.as_str() == curr {
                return idx
                    .checked_add(1)
                    .and_then(|next_idx| conflicts.get(next_idx).copied());
            }
        }
        conflicts
            .iter()
            .find(|item| item.0.as_str() > curr)
            .copied()
    }

    /// Navigates to the previous conflict relative to `current_rel_path`.
    pub fn prev_conflict<'a>(
        &'a self,
        current_rel_path: Option<&str>,
    ) -> Option<&'a (String, String)> {
        let conflicts = self.conflicts();
        let Some(curr) = current_rel_path else {
            return conflicts.last().copied();
        };
        for (idx, item) in conflicts.iter().enumerate() {
            if item.0.as_str() == curr {
                return idx
                    .checked_sub(1)
                    .and_then(|prev_idx| conflicts.get(prev_idx).copied());
            }
        }
        conflicts
            .iter()
            .rev()
            .find(|item| item.0.as_str() < curr)
            .copied()
    }

    /// Finds the planned step for a relative path, if any.
    pub fn step_for(&self, rel_path: &str) -> Option<&SyncStep> {
        self.steps.iter().find(|s| s.rel_path == rel_path)
    }

    /// Number of CopyToRight steps.
    pub fn copy_to_right_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.action == SyncAction::CopyToRight)
            .count()
    }

    /// Number of CopyToLeft steps.
    pub fn copy_to_left_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.action == SyncAction::CopyToLeft)
            .count()
    }

    /// Number of DeleteRight steps.
    pub fn delete_right_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.action == SyncAction::DeleteRight)
            .count()
    }

    /// Number of DeleteLeft steps.
    pub fn delete_left_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.action == SyncAction::DeleteLeft)
            .count()
    }

    /// Sorts steps ensuring directory creations precede their children and
    /// deletions sort children before parents.
    pub fn sort_steps(&mut self) {
        self.steps.sort_by(|a, b| {
            let is_del_a = matches!(a.action, SyncAction::DeleteLeft | SyncAction::DeleteRight);
            let is_del_b = matches!(b.action, SyncAction::DeleteLeft | SyncAction::DeleteRight);
            match (is_del_a, is_del_b) {
                (false, false) => a.rel_path.cmp(&b.rel_path),
                (true, true) => {
                    let depth_a = a.rel_path.matches('/').count();
                    let depth_b = b.rel_path.matches('/').count();
                    depth_b
                        .cmp(&depth_a)
                        .then_with(|| a.rel_path.cmp(&b.rel_path))
                }
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
            }
        });
    }

    /// Returns all steps matching a specific sync action.
    pub fn filter_by_action(&self, action: SyncAction) -> Vec<&SyncStep> {
        self.steps.iter().filter(|s| s.action == action).collect()
    }

    /// Whether this plan has no steps to apply.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Total number of copy steps in the plan.
    pub fn copy_count(&self) -> usize {
        self.copy_to_right_count()
            .saturating_add(self.copy_to_left_count())
    }

    /// Total number of delete steps in the plan.
    pub fn delete_count(&self) -> usize {
        self.delete_right_count()
            .saturating_add(self.delete_left_count())
    }

    /// Human-readable summary of the plan.
    pub fn summary(&self) -> String {
        format!(
            "SyncPlan: {} steps ({} copy-R, {} copy-L, {} del-R, {} del-L), {} conflicts, {} skipped",
            self.steps.len(),
            self.copy_to_right_count(),
            self.copy_to_left_count(),
            self.delete_right_count(),
            self.delete_left_count(),
            self.conflict_count(),
            self.skipped.len(),
        )
    }
}

impl SyncReport {
    /// Whether the sync operation succeeded with zero failed steps.
    pub fn is_success(&self) -> bool {
        self.failed.is_empty()
    }

    /// Total actions successfully applied (copied + deleted).
    pub fn total_applied(&self) -> usize {
        self.copied.saturating_add(self.deleted)
    }

    /// Number of failed steps.
    pub fn failure_count(&self) -> usize {
        self.failed.len()
    }

    /// Human-readable summary of the report.
    pub fn summary(&self) -> String {
        format!(
            "SyncReport: {} copied, {} deleted, {} failed",
            self.copied,
            self.deleted,
            self.failed.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "autocompare_sync_test_{name}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn test_sync_modes_and_planner_options() {
        let left = temp_dir("mode_left");
        let right = temp_dir("mode_right");

        let p_mirror_r2l = SyncPlanner::new(&left, &right, SyncMode::MirrorLeft);
        assert!(p_mirror_r2l.is_mirror());
        assert_eq!(p_mirror_r2l.options().direction, SyncDirection::RightToLeft);
        assert!(p_mirror_r2l.options().prune);

        let p_twoway = SyncPlanner::new(&left, &right, SyncMode::TwoWay);
        assert!(p_twoway.is_bidirectional());
        assert_eq!(p_twoway.options().direction, SyncDirection::NewerWins);
        assert!(!p_twoway.options().prune);

        let _ = fs::remove_dir_all(left);
        let _ = fs::remove_dir_all(right);
    }

    #[test]
    fn test_custom_rules_sync() {
        let left = temp_dir("custom_rules_l");
        let right = temp_dir("custom_rules_r");

        fs::write(left.join("a.txt"), "left a").unwrap();
        fs::write(right.join("b.txt"), "right b").unwrap();

        let rules = SyncCustomRules {
            on_left_only: SyncRuleAction::CopyLeftToRight,
            on_right_only: SyncRuleAction::DeleteRight,
            on_newer_left: SyncRuleAction::CopyLeftToRight,
            on_newer_right: SyncRuleAction::CopyRightToLeft,
            on_different: SyncRuleAction::Skip,
            on_same: SyncRuleAction::Skip,
        };

        let planner = SyncPlanner::new(&left, &right, SyncMode::Custom).with_custom_rules(rules);
        let plan = planner.dry_run().expect("dry run");
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.filter_by_action(SyncAction::CopyToRight).len(), 1);
        assert_eq!(plan.filter_by_action(SyncAction::DeleteRight).len(), 1);
        assert_eq!(plan.copy_count(), 1);
        assert_eq!(plan.delete_count(), 1);

        let _ = fs::remove_dir_all(left);
        let _ = fs::remove_dir_all(right);
    }

    #[test]
    fn test_prune_empty_directories_nested() {
        let root = temp_dir("prune_nested");
        let empty_sub = root.join("parent/child/deep");
        fs::create_dir_all(&empty_sub).unwrap();

        let non_empty = root.join("keep/dir");
        fs::create_dir_all(&non_empty).unwrap();
        fs::write(non_empty.join("file.txt"), "content").unwrap();

        let pruned = prune_empty_directories(&root, None).expect("prune");
        assert_eq!(pruned, 3); // deep, child, parent
        assert!(!empty_sub.exists());
        assert!(!root.join("parent").exists());
        assert!(non_empty.join("file.txt").exists());
        assert!(root.exists()); // root itself is preserved

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_sort_steps_ordering() {
        let mut plan = SyncPlan {
            steps: vec![
                SyncStep {
                    rel_path: "a/b/c.txt".into(),
                    is_dir: false,
                    action: SyncAction::DeleteRight,
                },
                SyncStep {
                    rel_path: "x/y.txt".into(),
                    is_dir: false,
                    action: SyncAction::CopyToRight,
                },
                SyncStep {
                    rel_path: "a".into(),
                    is_dir: true,
                    action: SyncAction::DeleteRight,
                },
                SyncStep {
                    rel_path: "x".into(),
                    is_dir: true,
                    action: SyncAction::CopyToRight,
                },
            ],
            skipped: Vec::new(),
            truncated: false,
        };

        plan.sort_steps();
        // Copies come first: "x" before "x/y.txt"
        assert_eq!(plan.steps[0].rel_path, "x");
        assert_eq!(plan.steps[1].rel_path, "x/y.txt");
        // Deletes come after, children before parents: "a/b/c.txt" before "a"
        assert_eq!(plan.steps[2].rel_path, "a/b/c.txt");
        assert_eq!(plan.steps[3].rel_path, "a");
    }

    #[test]
    fn test_under_path_safety() {
        let root = PathBuf::from("/test/root");
        assert!(under(&root, "safe/path.txt").is_some());
        assert!(under(&root, "../escaped.txt").is_none());
        assert!(under(&root, "safe/../../escape.txt").is_none());
        assert!(under(&root, "/absolute.txt").is_none());
        assert!(under(&root, "").is_none());
        assert!(under(&root, ".").is_none());
    }
}
