//! Folder sync: plan building and applying.

use std::path::Path;

use crate::{Result, SyncOptions, SyncPlan, SyncReport};

/// Builds a sync plan from a folder comparison. See the crate
/// documentation for the rules.
pub(crate) fn plan_sync(left: &Path, right: &Path, opts: &SyncOptions) -> Result<SyncPlan> {
    // STUB: replaced by the real implementation in crates/compare.
    let _ = (left, right, opts);
    Ok(SyncPlan::default())
}

/// Applies a plan. See the crate documentation for the guarantees.
pub(crate) fn apply_sync(left: &Path, right: &Path, plan: &SyncPlan) -> Result<SyncReport> {
    // STUB: replaced by the real implementation in crates/compare.
    let _ = (left, right, plan);
    Ok(SyncReport::default())
}
