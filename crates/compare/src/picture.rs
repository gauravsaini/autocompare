//! PNG picture comparison over the filesystem.

use std::path::Path;

use crate::{PictureComparison, PictureOptions, Result};

/// Compares two PNG files. See the crate documentation.
pub(crate) fn compare_picture_files(
    left: &Path,
    right: &Path,
    opts: &PictureOptions,
) -> Result<PictureComparison> {
    // STUB: replaced by the real implementation in crates/compare.
    let _ = (left, right, opts);
    Ok(PictureComparison {
        diff: autocompare_diff::PictureDiff {
            width_l: 0,
            height_l: 0,
            width_r: 0,
            height_r: 0,
            overlap_w: 0,
            overlap_h: 0,
            differing: 0,
            total: 0,
            bbox: None,
            identical: true,
        },
    })
}
