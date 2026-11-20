//! Audio file comparison over the filesystem.
//!
//! Both files are read whole, decoded by the pure layer and compared
//! sample by sample over the shorter stream's duration. A file that
//! does not decode as WAV or MP3 is [`Error::Unsupported`], not a
//! crash; a sample-rate or channel mismatch is reported through the
//! diff (not comparable, honestly), not as an error.

use std::path::Path;

use autocompare_diff::media_tags::{MediaTags, MediaTagsDiff, diff_media_tags, read_media_tags};
use autocompare_diff::{PcmAudio, decode_audio, diff_audio};

use crate::{AudioComparison, Error, Result};

/// Compares two audio files. See the crate documentation.
pub(crate) fn compare_audio_files(left: &Path, right: &Path) -> Result<AudioComparison> {
    let left_bytes = read_bytes(left)?;
    let right_bytes = read_bytes(right)?;
    let left_audio = decode_side(left, &left_bytes)?;
    let right_audio = decode_side(right, &right_bytes)?;
    let diff = diff_audio(&left_audio, &right_audio);
    Ok(AudioComparison { diff })
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
#[allow(dead_code)]
pub struct AudioTolerance {
    /// Samples with `|l - r| <= amplitude_tolerance` count as equal.
    pub amplitude_tolerance: f32,
}

/// Sample-level totality over the common span plus stream energies.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct AudioTotality {
    /// Max absolute sample delta over compared samples.
    pub peak_delta: f32,
    /// RMS of sample deltas over compared samples.
    pub rms_delta: f64,
    /// Peak amplitude of left stream.
    pub left_peak: f32,
    /// Peak amplitude of right stream.
    pub right_peak: f32,
    /// RMS energy of left stream.
    pub left_rms: f64,
    /// RMS energy of right stream.
    pub right_rms: f64,
    /// Absolute duration difference in seconds.
    pub duration_diff: f64,
}

/// Compares two audio files with an amplitude tolerance.
///
/// Same decoding and config-mismatch honesty as
/// [`compare_audio_files`]; only the per-sample equality gains the
/// tolerance. Undecodable sides are [`Error::Unsupported`].
#[allow(dead_code)]
pub fn compare_audio_files_with_tolerance(
    left: &Path,
    right: &Path,
    tol: &AudioTolerance,
) -> Result<AudioComparison> {
    let left_bytes = read_bytes(left)?;
    let right_bytes = read_bytes(right)?;
    let left_audio = decode_side(left, &left_bytes)?;
    let right_audio = decode_side(right, &right_bytes)?;
    let diff = diff_tolerant(&left_audio, &right_audio, tol.amplitude_tolerance);
    Ok(AudioComparison { diff })
}

/// Compares two audio files with tolerance and full totality metrics.
///
/// Returns the window diff plus [`AudioTotality`]. Undecodable sides
/// are [`Error::Unsupported`]; config mismatch yields a totality of
/// stream energies with zero joint deltas (honest, not a guess).
#[allow(dead_code)]
pub fn compare_audio_files_total(
    left: &Path,
    right: &Path,
    tol: &AudioTolerance,
) -> Result<(AudioComparison, AudioTotality)> {
    let left_bytes = read_bytes(left)?;
    let right_bytes = read_bytes(right)?;
    let left_audio = decode_side(left, &left_bytes)?;
    let right_audio = decode_side(right, &right_bytes)?;
    let diff = diff_tolerant(&left_audio, &right_audio, tol.amplitude_tolerance);
    let totality = totality_of(&left_audio, &right_audio, &diff);
    Ok((AudioComparison { diff }, totality))
}

#[allow(dead_code)]
fn diff_tolerant(
    a: &PcmAudio,
    b: &PcmAudio,
    amplitude_tolerance: f32,
) -> autocompare_diff::AudioDiff {
    let tol = amplitude_tolerance.max(0.0);
    let frames_l = frame_count_of(a);
    let frames_r = frame_count_of(b);
    let duration_l = duration_of(frames_l, a);
    let duration_r = duration_of(frames_r, b);
    let config_mismatch = a.sample_rate != b.sample_rate || a.channels != b.channels;
    if config_mismatch {
        return autocompare_diff::AudioDiff {
            sample_rate_l: a.sample_rate,
            sample_rate_r: b.sample_rate,
            channels_l: a.channels,
            channels_r: b.channels,
            duration_l,
            duration_r,
            config_mismatch,
            first_diff_at: None,
            windows_total: 0,
            windows_differing: 0,
            identical: false,
        };
    }
    let channels = usize::from(a.channels);
    let common = a.samples.len().min(b.samples.len());
    let mut first_frame: Option<u64> = None;
    let frames_per_window = (u64::from(a.sample_rate) / 100).max(1);
    let mut windows_total = 0u64;
    let mut windows_differing = 0u64;
    if channels > 0 {
        let common_frames = u64::try_from(common.checked_div(channels).unwrap_or(0)).unwrap_or(0);
        windows_total = common_frames / frames_per_window;
        let mut last_window: Option<u64> = None;
        for (index, (l, r)) in a
            .samples
            .iter()
            .zip(b.samples.iter())
            .take(common)
            .enumerate()
        {
            if l == r || (*l - *r).abs() <= tol {
                continue;
            }
            let frame = u64::try_from(index.checked_div(channels).unwrap_or(usize::MAX))
                .unwrap_or(u64::MAX);
            if first_frame.is_none() {
                first_frame = Some(frame);
            }
            let window = frame / frames_per_window;
            if window < windows_total && last_window != Some(window) {
                windows_differing = windows_differing.saturating_add(1);
                last_window = Some(window);
            }
        }
    }
    let identical = first_frame.is_none() && a.samples.len() == b.samples.len();
    let first_diff_at = match (first_frame, a.sample_rate) {
        (Some(frame), rate) if rate > 0 => Some(frame as f64 / f64::from(rate)),
        _ => None,
    };
    autocompare_diff::AudioDiff {
        sample_rate_l: a.sample_rate,
        sample_rate_r: b.sample_rate,
        channels_l: a.channels,
        channels_r: b.channels,
        duration_l,
        duration_r,
        config_mismatch,
        first_diff_at,
        windows_total,
        windows_differing,
        identical,
    }
}

#[allow(dead_code)]
fn totality_of(a: &PcmAudio, b: &PcmAudio, diff: &autocompare_diff::AudioDiff) -> AudioTotality {
    let common = a.samples.len().min(b.samples.len());
    let mut peak_delta = 0.0f32;
    let mut sum_sq = 0.0f64;
    for (l, r) in a.samples.iter().zip(b.samples.iter()).take(common) {
        let d = (*l - *r).abs();
        if d > peak_delta {
            peak_delta = d;
        }
        sum_sq += f64::from(d) * f64::from(d);
    }
    let rms_delta = if common > 0 {
        (sum_sq / (common as f64)).sqrt()
    } else {
        0.0
    };
    AudioTotality {
        peak_delta,
        rms_delta,
        left_peak: peak_of(a),
        right_peak: peak_of(b),
        left_rms: rms_of(a),
        right_rms: rms_of(b),
        duration_diff: (diff.duration_l - diff.duration_r).abs(),
    }
}

#[allow(dead_code)]
fn frame_count_of(audio: &PcmAudio) -> u64 {
    if audio.channels == 0 {
        return 0;
    }
    u64::try_from(audio.samples.len() / usize::from(audio.channels)).unwrap_or(u64::MAX)
}

#[allow(dead_code)]
fn duration_of(frames: u64, audio: &PcmAudio) -> f64 {
    if audio.sample_rate == 0 || audio.channels == 0 {
        0.0
    } else {
        frames as f64 / f64::from(audio.sample_rate)
    }
}

#[allow(dead_code)]
fn peak_of(audio: &PcmAudio) -> f32 {
    audio.samples.iter().fold(0.0f32, |acc, s| acc.max(s.abs()))
}

#[allow(dead_code)]
fn rms_of(audio: &PcmAudio) -> f64 {
    if audio.samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = audio
        .samples
        .iter()
        .map(|s| f64::from(*s) * f64::from(*s))
        .sum();
    (sum / (audio.samples.len() as f64)).sqrt()
}

/// Loads media metadata tags (FLAC, MP3, MP4, or AAC) from a file.
#[allow(dead_code)]
pub fn load_media_tags(path: &Path) -> Result<MediaTags> {
    let bytes = read_bytes(path)?;
    read_media_tags(&bytes).ok_or_else(|| Error::Unsupported {
        path: path.display().to_string(),
        what: "no decodable media tags (FLAC/MP3/MP4/AAC)",
    })
}

/// The result of comparing media metadata tags across two audio files.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct MediaTagsComparison {
    /// Left audio file tags.
    pub left_tags: MediaTags,
    /// Right audio file tags.
    pub right_tags: MediaTags,
    /// Detailed tag diff.
    pub diff: MediaTagsDiff,
}

/// Compares metadata tags (FLAC, MP3, MP4/AAC) between two media files.
#[allow(dead_code)]
pub fn compare_media_tags_files(left: &Path, right: &Path) -> Result<MediaTagsComparison> {
    let left_tags = load_media_tags(left)?;
    let right_tags = load_media_tags(right)?;
    let diff = diff_media_tags(&left_tags, &right_tags);
    Ok(MediaTagsComparison {
        left_tags,
        right_tags,
        diff,
    })
}

/// Combined comparison of media tags and PCM audio between two files.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct FullMediaComparison {
    /// Tag difference if tags are present on either side.
    pub tags: Option<MediaTagsComparison>,
    /// PCM audio sample diff if audio is decodable on both sides.
    pub audio_diff: Option<autocompare_diff::AudioDiff>,
}

/// Compares both metadata tags and PCM audio streams between two media files.
#[allow(dead_code)]
pub fn compare_media_files_full(left: &Path, right: &Path) -> Result<FullMediaComparison> {
    let tags_result = compare_media_tags_files(left, right).ok();
    let audio_result = compare_audio_files(left, right).ok().map(|c| c.diff);

    if tags_result.is_none() && audio_result.is_none() {
        return Err(Error::Unsupported {
            path: format!("{} / {}", left.display(), right.display()),
            what: "neither media tags nor PCM audio could be decoded",
        });
    }

    Ok(FullMediaComparison {
        tags: tags_result,
        audio_diff: audio_result,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_tolerant_diff_absorbs_small_delta() {
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
        let strict = diff_tolerant(&a, &b, 0.0);
        assert!(!strict.identical);
        let relaxed = diff_tolerant(&a, &b, 0.001);
        assert!(relaxed.identical);
    }

    #[test]
    fn test_totality_metrics_honest() {
        let a = PcmAudio {
            sample_rate: 44100,
            channels: 1,
            samples: vec![0.0, 0.5, 1.0],
        };
        let b = PcmAudio {
            sample_rate: 44100,
            channels: 1,
            samples: vec![0.0, 0.6, 1.0],
        };
        let diff = diff_tolerant(&a, &b, 0.0);
        let tot = totality_of(&a, &b, &diff);
        assert!((tot.peak_delta - 0.1).abs() < 1e-6);
        assert_eq!(tot.left_peak, 1.0);
        assert!((tot.duration_diff - 0.0).abs() < 1e-12);
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
        let d = diff_tolerant(&a, &b, 1.0);
        assert!(d.config_mismatch);
        assert!(!d.identical);
    }
}
