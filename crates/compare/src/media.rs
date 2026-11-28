//! Audio file comparison over the filesystem.
//!
//! Both files are read whole, decoded by the pure layer and compared
//! sample by sample over the shorter stream's duration. A file that
//! does not decode as WAV or MP3 is [`Error::Unsupported`], not a
//! crash; a sample-rate or channel mismatch is reported through the
//! diff (not comparable, honestly), not as an error.

use std::path::Path;

use autocompare_diff::{PcmAudio, decode_audio, diff_audio_with_tolerance};

use crate::folder::{odd_enabled, odd_path_token};
use crate::{AudioComparison, Error, Result};

/// Compares two audio files. See the crate documentation.
pub(crate) fn compare_audio_files(left: &Path, right: &Path) -> Result<AudioComparison> {
    compare_audio_files_with_tolerance(left, right, &AudioTolerance::default())
}

/// Reads a file's bytes; a missing path is [`Error::Missing`], any
/// other failure is [`Error::Io`].
fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Err(Error::Missing {
            path: path.display().to_string(),
        }),
        Err(source) => Err(Error::Io {
            path: path.display().to_string(),
            source,
        }),
    }
}

/// Decodes one side's audio bytes; undecodable content is
/// [`Error::Unsupported`].
fn decode_side(path: &Path, bytes: &[u8]) -> Result<PcmAudio> {
    decode_audio(bytes).ok_or(Error::Unsupported {
        path: path.display().to_string(),
        what: "not a decodable WAV or MP3",
    })
}

/// Amplitude tolerance for PCM comparison (Beyond Compare parity).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AudioTolerance {
    /// Samples with `|l - r| <= amplitude_tolerance` count as equal.
    pub amplitude_tolerance: f32,
}

/// Compares two audio files with an amplitude tolerance.
///
/// Same decoding and config-mismatch honesty as
/// [`compare_audio_files`]; only the per-sample equality gains the
/// tolerance. Undecodable sides are [`Error::Unsupported`].
pub fn compare_audio_files_with_tolerance(
    left: &Path,
    right: &Path,
    tol: &AudioTolerance,
) -> Result<AudioComparison> {
    // ODD Phase 4 Leaf B: orchestration span (redacted summaries only).
    let odd_span = tracing::info_span!(
        "media.audio_compare",
        component = "rs.compare/media",
        left = %odd_path_token(left),
        right = %odd_path_token(right),
        tolerance = tol.amplitude_tolerance,
        identical = tracing::field::Empty,
        config_mismatch = tracing::field::Empty,
        windows_differing = tracing::field::Empty,
    );
    let _odd_guard = if odd_enabled() {
        Some(odd_span.enter())
    } else {
        None
    };
    let left_bytes = read_bytes(left)?;
    let right_bytes = read_bytes(right)?;
    let left_audio = decode_side(left, &left_bytes)?;
    let right_audio = decode_side(right, &right_bytes)?;
    let diff = diff_audio_with_tolerance(&left_audio, &right_audio, tol.amplitude_tolerance);
    odd_span.record("identical", diff.identical);
    odd_span.record("config_mismatch", diff.config_mismatch);
    odd_span.record("windows_differing", diff.windows_differing);
    Ok(AudioComparison { diff })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::compare_audio_files;

    fn mono_wav(rate: u32, samples: &[i16]) -> Vec<u8> {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        let data_len = samples.len().saturating_mul(2) as u32;
        wav.extend_from_slice(&(data_len.saturating_add(36)).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&rate.to_le_bytes());
        wav.extend_from_slice(&rate.saturating_mul(2).to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        wav
    }

    #[test]
    fn test_pcm_wav_kept_identical_through_files() {
        let dir = tempfile::tempdir().unwrap();
        let samples = [0i16, 1000, -1000, 8000];
        let left = dir.path().join("left.wav");
        let right = dir.path().join("right.wav");
        std::fs::write(&left, mono_wav(8000, &samples)).unwrap();
        std::fs::write(&right, mono_wav(8000, &samples)).unwrap();
        let same = compare_audio_files(&left, &right).unwrap();
        assert!(same.diff.identical);
        assert!(!same.diff.config_mismatch);
        // One LSB differs strictly but survives a small tolerance.
        let mut changed = samples;
        changed[1] = changed[1].saturating_add(1);
        let drifted = dir.path().join("drifted.wav");
        std::fs::write(&drifted, mono_wav(8000, &changed)).unwrap();
        let strict = compare_audio_files(&left, &drifted).unwrap();
        assert!(!strict.diff.identical);
        let tol = AudioTolerance {
            amplitude_tolerance: 0.001,
        };
        let relaxed = compare_audio_files_with_tolerance(&left, &drifted, &tol).unwrap();
        assert!(relaxed.diff.identical);
    }

    #[test]
    fn test_zero_tolerance_matches_exact_diff() {
        let a = PcmAudio {
            sample_rate: 44100,
            channels: 1,
            samples: vec![0.5, 0.5],
        };
        let b = PcmAudio {
            sample_rate: 44100,
            channels: 1,
            samples: vec![0.50005, 0.49995],
        };
        let strict =
            diff_audio_with_tolerance(&a, &b, AudioTolerance::default().amplitude_tolerance);
        assert!(!strict.identical);
        let relaxed = diff_audio_with_tolerance(&a, &b, 0.001);
        assert!(relaxed.identical);
    }

    #[test]
    fn test_config_mismatch_is_not_comparable() {
        let a = PcmAudio {
            sample_rate: 44100,
            channels: 1,
            samples: vec![0.0],
        };
        let b = PcmAudio {
            sample_rate: 48000,
            channels: 1,
            samples: vec![0.0],
        };
        let d = diff_audio_with_tolerance(&a, &b, 1.0);
        assert!(d.config_mismatch);
        assert!(!d.identical);
    }
}
