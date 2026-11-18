//! WAV and MP3 decoding plus PCM comparison.
//!
//! Pure: bytes in, samples out; no I/O, no panics. Undecodable input
//! is `None`, never an error; decoding is capped by [`MAX_SAMPLES`]
//! so hostile headers cannot exhaust memory.

/// Decoded interleaved PCM audio, f32 samples nominally in [-1, 1].
#[derive(Debug, Clone, PartialEq)]
pub struct PcmAudio {
    /// Samples per second.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u16,
    /// Interleaved samples: all channels of frame 0, then frame 1.
    pub samples: Vec<f32>,
}

/// Honest decode cap: a file decoding to more samples is treated as
/// undecodable (roughly 12 hours of stereo at 44.1 kHz).
pub const MAX_SAMPLES: usize = 200_000_000;

/// Decodes RIFF/WAV bytes (PCM 8/16/24/32-bit integer and 32/64-bit
/// IEEE float samples; any channel count). `None` when the bytes are
/// not a decodable WAV.
pub fn decode_wav(bytes: &[u8]) -> Option<PcmAudio> {
    // STUB: replaced by the real implementation in crates/diff.
    let _ = bytes;
    None
}

/// Decodes MP3 bytes (MPEG-1/2/2.5 audio, layers I/II/III, via the
/// pure-Rust symphonia decoder). `None` when the bytes are not a
/// decodable MP3.
pub fn decode_mp3(bytes: &[u8]) -> Option<PcmAudio> {
    // STUB: replaced by the real implementation in crates/diff.
    let _ = bytes;
    None
}

/// Sniffs the container and decodes WAV or MP3. `None` for anything
/// else.
pub fn decode_audio(bytes: &[u8]) -> Option<PcmAudio> {
    // STUB: replaced by the real implementation in crates/diff.
    let _ = bytes;
    None
}

/// Statistics of a PCM comparison over the two streams.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioDiff {
    /// Left sample rate.
    pub sample_rate_l: u32,
    /// Right sample rate.
    pub sample_rate_r: u32,
    /// Left channel count.
    pub channels_l: u16,
    /// Right channel count.
    pub channels_r: u16,
    /// Left duration in seconds.
    pub duration_l: f64,
    /// Right duration in seconds.
    pub duration_r: f64,
    /// True when sample rates or channel counts differ. The
    /// comparison fields below are then None: comparing across
    /// configs is not meaningful, and the honest answer is "not
    /// comparable", not a guess.
    pub config_mismatch: bool,
    /// Seconds into the stream of the first differing frame.
    pub first_diff_at: Option<f64>,
    /// 10 ms comparison windows over the common duration.
    pub windows_total: u64,
    /// Windows containing at least one differing sample.
    pub windows_differing: u64,
    /// Same configuration and no differing sample anywhere.
    pub identical: bool,
}

/// Compares two decoded streams.
///
/// When the configs match, frames are compared sample by sample over
/// the shorter stream's duration and 10 ms windows summarize the
/// differences. When they differ, only the header statistics are
/// filled in.
pub fn diff_audio(a: &PcmAudio, b: &PcmAudio) -> AudioDiff {
    // STUB: replaced by the real implementation in crates/diff.
    let _ = (a, b);
    AudioDiff {
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
        identical: false,
    }
}
