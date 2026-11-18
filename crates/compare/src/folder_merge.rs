//! Three-way folder merge: plan building and applying.

use std::path::Path;

use crate::{Result, TreeMergePlan, TreeMergeReport};

/// Plans a three-way merge of two trees over their base. See the crate
/// documentation for the resolution rules.
pub(crate) fn plan_tree_merge(base: &Path, left: &Path, right: &Path) -> Result<TreeMergePlan> {
    // STUB: replaced by the real implementation in crates/compare.
    let _ = (base, left, right);
    Ok(TreeMergePlan::default())
}

/// Applies a merge plan. See the crate documentation for the
/// guarantees.
pub(crate) fn apply_tree_merge(plan: &TreeMergePlan, output: &Path) -> Result<TreeMergeReport> {
    // STUB: replaced by the real implementation in crates/compare.
    let _ = (plan, output);
    Ok(TreeMergeReport::default())
}
