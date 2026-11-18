//! Audio file comparison over the filesystem.

use std::path::Path;

use crate::{AudioComparison, Result};

/// Compares two audio files. See the crate documentation.
pub(crate) fn compare_audio_files(left: &Path, right: &Path) -> Result<AudioComparison> {
    // STUB: replaced by the real implementation in crates/compare.
    let _ = (left, right);
    Ok(AudioComparison {
        diff: autocompare_diff::AudioDiff {
            sample_rate_l: 0,
            sample_rate_r: 0,
            channels_l: 0,
            channels_r: 0,
            duration_l: 0.0,
            duration_r: 0.0,
            config_mismatch: true,
            first_diff_at: None,
            windows_total: 0,
            windows_differing: 0,
            identical: true,
        },
    })
}
