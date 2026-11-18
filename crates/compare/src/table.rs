//! Delimited table comparison over the filesystem.

use std::path::Path;

use autocompare_diff::DiffScript;

use crate::{Result, TableComparison, TableOptions};

/// Compares two delimited files. See the crate documentation.
pub(crate) fn compare_table_files(
    left: &Path,
    right: &Path,
    opts: &TableOptions,
) -> Result<TableComparison> {
    // STUB: replaced by the real implementation in crates/compare.
    let _ = (left, right, opts);
    Ok(TableComparison {
        header_l: Vec::new(),
        header_r: Vec::new(),
        rows_l: Vec::new(),
        rows_r: Vec::new(),
        script: DiffScript {
            ops: Vec::new(),
            exact: true,
        },
    })
}
