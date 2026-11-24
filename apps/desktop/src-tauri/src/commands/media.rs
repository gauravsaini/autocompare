//! Media (picture + audio) compare IPC commands (Wave 1d leaf).
//!
//! # Ownership
//!
//! This module is owned by Wave 1d. Wave 1R owns `commands/mod.rs` and the
//! `lib.rs` handler registration, so this file is intentionally **not**
//! referenced from anywhere yet; that is fine and the crate still builds.
//!
//! # Wiring (Wave 1R handoff)
//!
//! 1. Add to `apps/desktop/src-tauri/Cargo.toml`:
//!    `autocompare-compare = { path = "../../../crates/compare" }`.
//! 2. Create `commands/mod.rs` with `pub mod media;` (and `pub mod session;`).
//! 3. Register `picture_compare` and `audio_compare` in the `lib.rs`
//!    `generate_handler![...]` table.
//!
//! # Engine swap note (Wave 1R / later wave)
//!
//! The bodies below are std-only on purpose: this wave may not touch
//! `Cargo.toml`, so the engine crates are not linked yet. The DTO shapes
//! mirror the engine diffs field-for-field, so the swap is body-only and
//! `ipc/media.ts` is unaffected:
//!
//! - `picture_compare`: replace the byte-level scan with
//!   `autocompare_compare::compare_picture_files(left, right,
//!   &PictureOptions::default())` and map `PictureComparison.diff`
//!   (`width_l/height_l/width_r/height_r`, `differing`, `total`,
//!   `bbox`, `identical`, `differ_ratio()`) onto [`PictureSummary`].
//!   Pixel-accurate diff samples then come from the decoded overlap
//!   instead of raw file bytes. The [`MAX_DIFF_SAMPLES`] cap stays.
//! - `audio_compare`: replace [`parse_wav`] + the inline PCM loop with
//!   `autocompare_compare::compare_audio_files_with_tolerance(left,
//!   right, &AudioTolerance { amplitude_tolerance: tolerance })` and map
//!   `AudioComparison.diff` (`sample_rate_l/r`, `channels_l/r`,
//!   `duration_l/r`, `config_mismatch`, `windows_total`,
//!   `windows_differing`, `first_diff_at`, `identical`) onto
//!   [`AudioSummary`]. This also adds MP3 support; the interim
//!   implementation below is WAV (8/16/24-bit integer PCM) only.
//!
//! # Bounded-payload rule (locked)
//!
//! No command returns full bitmaps or full sample buffers over IPC.
//! [`PictureSummary::samples`] and [`AudioSummary::diff_windows`] are
//! downsampled to at most [`MAX_DIFF_SAMPLES`] entries; longer runs set
//! `truncated` instead of growing the payload. Every handler returns
//! errors as [`String`] and never panics.

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Maximum diff-sample entries returned over IPC. Longer runs set the
/// DTO's `truncated` flag instead of growing the payload.
pub const MAX_DIFF_SAMPLES: usize = 256;

/// Counter for opaque compare receipts (process-unique, never reused).
static NEXT_RECEIPT: AtomicU64 = AtomicU64::new(1);

/// PNG file signature; used to sniff decodable pictures and read
/// dimensions without a decoder.
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// One downsampled differing byte. Mirrors `ipc/media.ts`
/// `ByteDiffSample`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ByteDiffSample {
    /// Byte offset into both files.
    pub offset: u64,
    /// Byte value on the left at `offset`.
    pub left: u8,
    /// Byte value on the right at `offset`.
    pub right: u8,
}

/// Bounded picture-comparison summary. Mirrors `ipc/media.ts`
/// `PictureSummary`. Scalar metrics plus downsampled diff stats only --
/// never full bitmaps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PictureSummary {
    /// Opaque receipt for this comparison.
    pub receipt: String,
    /// True when both files are byte-identical.
    pub identical: bool,
    /// Sniffed container: `"png"` or `"unknown"`.
    pub format: String,
    /// Left image width/height from the PNG `IHDR`, else 0.
    pub width_l: u32,
    /// Left image height from the PNG `IHDR`, else 0.
    pub height_l: u32,
    /// Right image width from the PNG `IHDR`, else 0.
    pub width_r: u32,
    /// Right image height from the PNG `IHDR`, else 0.
    pub height_r: u32,
    /// Left file size in bytes.
    pub size_l: u64,
    /// Right file size in bytes.
    pub size_r: u64,
    /// Fraction of equal bytes over the longer file, in `[0.0, 1.0]`.
    pub score: f64,
    /// Differing byte positions plus the length tail, if any.
    pub differing_bytes: u64,
    /// Longer file length; the denominator of [`score`](Self::score).
    pub total_bytes: u64,
    /// Downsampled differing offsets, at most [`MAX_DIFF_SAMPLES`].
    pub samples: Vec<ByteDiffSample>,
    /// True when differing positions exceeded the sample cap.
    pub truncated: bool,
}

/// One downsampled differing 10 ms audio window. Mirrors
/// `ipc/media.ts` `AudioDiffWindow`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDiffWindow {
    /// 10 ms window index into the common duration.
    pub window: u64,
    /// Window start in seconds.
    pub at_seconds: f64,
}

/// Bounded audio-comparison summary. Mirrors `ipc/media.ts`
/// `AudioSummary` and (field-for-field) the engine `AudioDiff`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSummary {
    /// Opaque receipt for this comparison.
    pub receipt: String,
    /// True when streams match within `tolerance` over equal lengths.
    pub identical: bool,
    /// True when sample rates, channels, or bit depths differ. The
    /// window fields are then 0/`None`: cross-config comparison is not
    /// meaningful, honestly reported instead of guessed.
    pub config_mismatch: bool,
    /// Left sample rate in Hz.
    pub sample_rate_l: u32,
    /// Right sample rate in Hz.
    pub sample_rate_r: u32,
    /// Left channel count.
    pub channels_l: u16,
    /// Right channel count.
    pub channels_r: u16,
    /// Left duration in seconds.
    pub duration_l: f64,
    /// Right duration in seconds.
    pub duration_r: f64,
    /// Applied amplitude tolerance, echoed back.
    pub tolerance: f32,
    /// 10 ms windows over the common duration.
    pub windows_total: u64,
    /// Windows containing at least one differing sample.
    pub windows_differing: u64,
    /// Seconds into the stream of the first differing frame, if any.
    pub first_diff_at: Option<f64>,
    /// Fraction of equal samples over the common duration, in
    /// `[0.0, 1.0]` (`0.0` on config mismatch).
    pub score: f64,
    /// Downsampled differing windows, at most [`MAX_DIFF_SAMPLES`].
    pub diff_windows: Vec<AudioDiffWindow>,
    /// True when differing windows exceeded the sample cap.
    pub truncated: bool,
}

/// Reads a file's bytes; a missing path is a `Missing`-style string, any
/// other failure an `Io`-style string. Mirrors the engine error wording.
fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("path not found: {}", path.display()))
        }
        Err(source) => Err(format!("reading {} failed: {source}", path.display())),
    }
}

/// Minted receipt ids are process-unique (`pic-<n>` / `aud-<n>`).
fn next_receipt(prefix: &str) -> String {
    let n = NEXT_RECEIPT.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{n}")
}

/// Big-endian `u32` at `at`, or `None` when out of bounds. No slicing,
/// so the `indexing_slicing` lint stays quiet.
fn be_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let b0 = bytes.get(at).copied()?;
    let b1 = bytes.get(at.saturating_add(1)).copied()?;
    let b2 = bytes.get(at.saturating_add(2)).copied()?;
    let b3 = bytes.get(at.saturating_add(3)).copied()?;
    Some(u32::from_be_bytes([b0, b1, b2, b3]))
}

/// Little-endian `u32` at `at`, or `None` when out of bounds.
fn le_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let b0 = bytes.get(at).copied()?;
    let b1 = bytes.get(at.saturating_add(1)).copied()?;
    let b2 = bytes.get(at.saturating_add(2)).copied()?;
    let b3 = bytes.get(at.saturating_add(3)).copied()?;
    Some(u32::from_le_bytes([b0, b1, b2, b3]))
}

/// Little-endian `u16` at `at`, or `None` when out of bounds.
fn le_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let b0 = bytes.get(at).copied()?;
    let b1 = bytes.get(at.saturating_add(1)).copied()?;
    Some(u16::from_le_bytes([b0, b1]))
}

/// PNG dimensions from the `IHDR` chunk, or `None` when the bytes are
/// not a PNG or the header is truncated. No CRC validation: dimensions
/// are advisory metadata for the summary, not a decode.
fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if !bytes.starts_with(PNG_MAGIC) {
        return None;
    }
    // IHDR must be the first chunk: length at 8, tag at 12, data at 16
    // (width 16..20, height 20..24, big-endian).
    let is_ihdr = bytes
        .get(12..16)
        .is_some_and(|tag| tag == b"IHDR".as_slice());
    if !is_ihdr {
        return None;
    }
    let width = be_u32(bytes, 16)?;
    let height = be_u32(bytes, 20)?;
    Some((width, height))
}

/// Container sniff for the picture summary: `"png"` or `"unknown"`.
fn sniff_format(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(PNG_MAGIC) {
        "png"
    } else {
        "unknown"
    }
}

/// Compares two picture files and returns a bounded summary: scalar
/// metrics plus downsampled diff stats, never full bitmaps. Interim
/// byte-level comparison; see the engine swap note above.
#[tauri::command]
pub fn picture_compare(left: String, right: String) -> Result<PictureSummary, String> {
    if left.is_empty() {
        return Err("left path must not be empty".to_string());
    }
    if right.is_empty() {
        return Err("right path must not be empty".to_string());
    }
    let left_bytes = read_bytes(Path::new(&left))?;
    let right_bytes = read_bytes(Path::new(&right))?;
    Ok(summarize_picture(&left_bytes, &right_bytes))
}

/// Bounded picture summary over two byte buffers.
fn summarize_picture(left: &[u8], right: &[u8]) -> PictureSummary {
    let (width_l, height_l) = png_dimensions(left).unwrap_or((0, 0));
    let (width_r, height_r) = png_dimensions(right).unwrap_or((0, 0));
    let format = if sniff_format(left) == "png" && sniff_format(right) == "png" {
        "png"
    } else {
        "unknown"
    };

    let mut differing_overlap: u64 = 0;
    for pair in left.iter().zip(right.iter()) {
        if pair.0 != pair.1 {
            differing_overlap = differing_overlap.saturating_add(1);
        }
    }
    let len_tail = left.len().abs_diff(right.len()) as u64;
    let differing_bytes = differing_overlap.saturating_add(len_tail);
    let total_bytes = left.len().max(right.len()) as u64;
    let score = if total_bytes == 0 {
        1.0
    } else {
        (total_bytes.saturating_sub(differing_bytes) as f64) / (total_bytes as f64)
    };

    // Downsample differing offsets uniformly to the IPC cap.
    let stride = differing_overlap.div_ceil(MAX_DIFF_SAMPLES as u64).max(1);
    let mut seen: u64 = 0;
    let mut samples = Vec::new();
    for (index, pair) in left.iter().zip(right.iter()).enumerate() {
        if pair.0 == pair.1 {
            continue;
        }
        if seen.is_multiple_of(stride)
            && let (Some(l), Some(r)) = (left.get(index), right.get(index))
        {
            samples.push(ByteDiffSample {
                offset: index as u64,
                left: *l,
                right: *r,
            });
        }
        seen = seen.saturating_add(1);
    }
    let truncated = (samples.len() as u64) < differing_overlap;

    PictureSummary {
        receipt: next_receipt("pic"),
        identical: differing_bytes == 0,
        format: format.to_string(),
        width_l,
        height_l,
        width_r,
        height_r,
        size_l: left.len() as u64,
        size_r: right.len() as u64,
        score,
        differing_bytes,
        total_bytes,
        samples,
        truncated,
    }
}

/// Decoded WAV stream metadata plus PCM data extent.
struct WavInfo {
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    data_start: usize,
    data_len: usize,
}

/// Parses a WAV header (`RIFF....WAVE` + `fmt ` + `data` chunks),
/// tolerating extra chunks (`LIST`, `fact`, ...). Only integer PCM
/// (`audio_format == 1`) of 8/16/24 bits is supported; anything else is
/// an honest `Unsupported`-style string, not a crash.
fn parse_wav(path: &Path, bytes: &[u8]) -> Result<WavInfo, String> {
    let unsupported = |what: &str| format!("{}: {what}", path.display());
    if !bytes.starts_with(b"RIFF") {
        return Err(unsupported("not a decodable WAV or MP3"));
    }
    let is_wave = bytes
        .get(8..12)
        .is_some_and(|tag| tag == b"WAVE".as_slice());
    if !is_wave {
        return Err(unsupported("not a decodable WAV or MP3"));
    }
    let mut offset: usize = 12;
    let mut format: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<(usize, usize)> = None;
    while offset < bytes.len() {
        let id = match bytes.get(offset..offset.saturating_add(4)) {
            Some(id) => id,
            None => break,
        };
        let size = match le_u32(bytes, offset.saturating_add(4)) {
            Some(size) => size,
            None => return Err(unsupported("truncated WAV chunk header")),
        };
        let body = offset.saturating_add(8);
        let size_usize = usize::try_from(size).unwrap_or(usize::MAX);
        if id == b"fmt ".as_slice() {
            let audio_format = le_u16(bytes, body).unwrap_or(0);
            let channels = le_u16(bytes, body.saturating_add(2)).unwrap_or(0);
            let rate = le_u32(bytes, body.saturating_add(4)).unwrap_or(0);
            let bits = le_u16(bytes, body.saturating_add(14)).unwrap_or(0);
            format = Some((audio_format, channels, rate, bits));
        } else if id == b"data".as_slice() && data.is_none() {
            let available = bytes.len().saturating_sub(body);
            data = Some((body, available.min(size_usize)));
        }
        offset = body
            .saturating_add(size_usize)
            .saturating_add(size_usize & 1);
    }
    let (audio_format, channels, rate, bits) = match format {
        Some(fmt) => fmt,
        None => return Err(unsupported("WAV is missing its fmt chunk")),
    };
    if audio_format != 1 {
        return Err(unsupported("only integer PCM WAV is supported"));
    }
    if channels == 0 || rate == 0 {
        return Err(unsupported("WAV has no channels or zero sample rate"));
    }
    if bits != 8 && bits != 16 && bits != 24 {
        return Err(unsupported("only 8/16/24-bit PCM WAV is supported"));
    }
    let (data_start, data_len) = match data {
        Some(data) => data,
        None => return Err(unsupported("WAV is missing its data chunk")),
    };
    Ok(WavInfo {
        sample_rate: rate,
        channels,
        bits_per_sample: bits,
        data_start,
        data_len,
    })
}

/// Decodes one integer-PCM sample to `[-1.0, 1.0]`, or `None` when the
/// bytes are truncated. `frame_bytes` is kept for clarity at call sites.
fn decode_sample(bytes: &[u8], at: usize, bits: u16, _frame_bytes: usize) -> Option<f32> {
    match bits {
        8 => bytes
            .get(at)
            .copied()
            .map(|b| (f32::from(b) - 128.0) / 128.0),
        16 => {
            let lo = bytes.get(at).copied()?;
            let hi = bytes.get(at.saturating_add(1)).copied()?;
            Some(f32::from(i16::from_le_bytes([lo, hi])) / 32768.0)
        }
        24 => {
            let b0 = bytes.get(at).copied()?;
            let b1 = bytes.get(at.saturating_add(1)).copied()?;
            let b2 = bytes.get(at.saturating_add(2)).copied()?;
            let raw = i32::from_le_bytes([b0, b1, b2, if b2 >= 0x80 { 0xFF } else { 0x00 }]);
            Some(raw as f32 / 8_388_608.0)
        }
        _ => None,
    }
}

/// Compares two audio files with an amplitude tolerance and returns a
/// bounded summary. Interim WAV-only implementation; MP3 arrives with
/// the engine swap (see module docs).
#[tauri::command]
pub fn audio_compare(
    left: String,
    right: String,
    tolerance: Option<f32>,
) -> Result<AudioSummary, String> {
    if left.is_empty() {
        return Err("left path must not be empty".to_string());
    }
    if right.is_empty() {
        return Err("right path must not be empty".to_string());
    }
    let tol = tolerance.unwrap_or(0.0);
    if tol < 0.0 || !tol.is_finite() {
        return Err("tolerance must be a finite number >= 0".to_string());
    }
    let left_path = Path::new(&left);
    let right_path = Path::new(&right);
    let left_bytes = read_bytes(left_path)?;
    let right_bytes = read_bytes(right_path)?;
    let left_wav = parse_wav(left_path, &left_bytes)?;
    let right_wav = parse_wav(right_path, &right_bytes)?;
    Ok(summarize_audio(
        &left_bytes,
        &left_wav,
        &right_bytes,
        &right_wav,
        tol,
    ))
}

/// Bounded audio summary over two decoded WAV buffers.
fn summarize_audio(
    left_bytes: &[u8],
    left_wav: &WavInfo,
    right_bytes: &[u8],
    right_wav: &WavInfo,
    tolerance: f32,
) -> AudioSummary {
    let bytes_per_sample_l = u64::from(left_wav.bits_per_sample).saturating_div(8).max(1);
    let bytes_per_sample_r = u64::from(right_wav.bits_per_sample)
        .saturating_div(8)
        .max(1);
    let duration_l = audio_duration(left_wav, bytes_per_sample_l);
    let duration_r = audio_duration(right_wav, bytes_per_sample_r);
    let config_mismatch = left_wav.sample_rate != right_wav.sample_rate
        || left_wav.channels != right_wav.channels
        || left_wav.bits_per_sample != right_wav.bits_per_sample;
    if config_mismatch {
        return AudioSummary {
            receipt: next_receipt("aud"),
            identical: false,
            config_mismatch: true,
            sample_rate_l: left_wav.sample_rate,
            sample_rate_r: right_wav.sample_rate,
            channels_l: left_wav.channels,
            channels_r: right_wav.channels,
            duration_l,
            duration_r,
            tolerance,
            windows_total: 0,
            windows_differing: 0,
            first_diff_at: None,
            score: 0.0,
            diff_windows: Vec::new(),
            truncated: false,
        };
    }

    let channels = u64::from(left_wav.channels).max(1);
    let frame_bytes = bytes_per_sample_l.saturating_mul(channels).max(1);
    let frames_l = (left_wav.data_len as u64).saturating_div(frame_bytes);
    let frames_r = (right_wav.data_len as u64).saturating_div(frame_bytes);
    let common_frames = frames_l.min(frames_r);
    let frames_per_window = u64::from(left_wav.sample_rate).saturating_div(100).max(1);
    let windows_total = common_frames.saturating_div(frames_per_window);

    let mut first_frame: Option<u64> = None;
    let mut differing_samples: u64 = 0;
    let mut differing_windows: Vec<u64> = Vec::new();
    let mut frame: u64 = 0;
    while frame < common_frames {
        let mut frame_differs = false;
        let mut channel: u64 = 0;
        while channel < channels {
            let at_l = left_wav
                .data_start
                .saturating_add((frame.saturating_mul(frame_bytes)) as usize)
                .saturating_add((channel.saturating_mul(bytes_per_sample_l)) as usize);
            let at_r = right_wav
                .data_start
                .saturating_add((frame.saturating_mul(frame_bytes)) as usize)
                .saturating_add((channel.saturating_mul(bytes_per_sample_r)) as usize);
            let sample_l = decode_sample(
                left_bytes,
                at_l,
                left_wav.bits_per_sample,
                bytes_per_sample_l as usize,
            );
            let sample_r = decode_sample(
                right_bytes,
                at_r,
                right_wav.bits_per_sample,
                bytes_per_sample_r as usize,
            );
            match (sample_l, sample_r) {
                (Some(l), Some(r)) => {
                    if l != r && (l - r).abs() > tolerance {
                        frame_differs = true;
                    }
                }
                _ => {
                    frame_differs = true;
                }
            }
            channel = channel.saturating_add(1);
        }
        if frame_differs {
            differing_samples = differing_samples.saturating_add(1);
            if first_frame.is_none() {
                first_frame = Some(frame);
            }
            let window = frame.saturating_div(frames_per_window);
            let is_new_window = differing_windows
                .last()
                .copied()
                .is_none_or(|last| last != window);
            if is_new_window {
                differing_windows.push(window);
            }
        }
        frame = frame.saturating_add(1);
    }

    let rate_f = f64::from(left_wav.sample_rate);
    let first_diff_at = match first_frame {
        Some(f) if rate_f > 0.0 => Some((f as f64) / rate_f),
        _ => None,
    };
    let identical = first_frame.is_none() && frames_l == frames_r;
    let total_samples = common_frames.saturating_mul(channels);
    let score = if total_samples == 0 {
        if identical { 1.0 } else { 0.0 }
    } else {
        ((total_samples.saturating_sub(differing_samples)) as f64) / (total_samples as f64)
    };
    let stride = (differing_windows.len() as u64)
        .div_ceil(MAX_DIFF_SAMPLES as u64)
        .max(1);
    let mut diff_windows = Vec::new();
    for (index, window) in differing_windows.iter().enumerate() {
        if (index as u64).is_multiple_of(stride) {
            let at_seconds = if rate_f > 0.0 {
                (window.saturating_mul(frames_per_window) as f64) / rate_f
            } else {
                0.0
            };
            diff_windows.push(AudioDiffWindow {
                window: *window,
                at_seconds,
            });
        }
    }
    let truncated = (diff_windows.len() as u64) < (differing_windows.len() as u64);

    AudioSummary {
        receipt: next_receipt("aud"),
        identical,
        config_mismatch: false,
        sample_rate_l: left_wav.sample_rate,
        sample_rate_r: right_wav.sample_rate,
        channels_l: left_wav.channels,
        channels_r: right_wav.channels,
        duration_l,
        duration_r,
        tolerance,
        windows_total,
        windows_differing: differing_windows.len() as u64,
        first_diff_at,
        score,
        diff_windows,
        truncated,
    }
}

/// Stream duration in seconds from data length and format.
fn audio_duration(wav: &WavInfo, bytes_per_sample: u64) -> f64 {
    let channels = u64::from(wav.channels).max(1);
    let bytes_per_second = u64::from(wav.sample_rate)
        .saturating_mul(channels)
        .saturating_mul(bytes_per_sample);
    if bytes_per_second == 0 {
        0.0
    } else {
        (wav.data_len as f64) / (bytes_per_second as f64)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    fn write_temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("wave1d-media-tests");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let path = dir.join(name);
        std::fs::write(&path, bytes).expect("write");
        path
    }

    /// Minimal PNG-ish file: real magic + IHDR (width/height) + payload.
    /// The interim comparison only sniffs these bytes, so no CRC needed.
    fn fake_png(width: u32, height: u32, payload_byte: u8, payload_len: usize) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(PNG_MAGIC);
        out.extend_from_slice(&13u32.to_be_bytes());
        out.extend_from_slice(b"IHDR");
        out.extend_from_slice(&width.to_be_bytes());
        out.extend_from_slice(&height.to_be_bytes());
        out.extend_from_slice(&[8, 2, 0, 0, 0]);
        out.extend_from_slice(&[0u8; 4]);
        out.extend(std::iter::repeat_n(payload_byte, payload_len));
        out
    }

    /// Minimal 16-bit mono WAV.
    fn mono_wav(rate: u32, samples: &[i16]) -> Vec<u8> {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        let data_len = samples.len().saturating_mul(2) as u32;
        wav.extend_from_slice(&data_len.saturating_add(36).to_le_bytes());
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
    fn picture_missing_files_error() {
        let err = picture_compare(
            "/definitely/missing/left-1d.png".to_string(),
            "/definitely/missing/right-1d.png".to_string(),
        )
        .expect_err("missing files must fail");
        assert!(err.contains("path not found"), "unexpected: {err}");
    }

    #[test]
    fn picture_empty_paths_rejected() {
        assert!(picture_compare(String::new(), "b".to_string()).is_err());
        assert!(picture_compare("a".to_string(), String::new()).is_err());
    }

    #[test]
    fn picture_identical_and_round_trip() {
        let bytes = fake_png(4, 4, 0xAB, 64);
        let left = write_temp("pic-a.png", &bytes);
        let right = write_temp("pic-b.png", &bytes);
        let summary = picture_compare(
            left.to_string_lossy().to_string(),
            right.to_string_lossy().to_string(),
        )
        .expect("identical compare");
        assert!(summary.identical);
        assert_eq!(summary.score, 1.0);
        assert_eq!(summary.differing_bytes, 0);
        assert!(!summary.truncated);
        assert!(summary.samples.is_empty());
        assert_eq!(summary.format, "png");
        assert_eq!((summary.width_l, summary.height_l), (4, 4));
        let json = serde_json::to_string(&summary).expect("serialize");
        let back: PictureSummary = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(summary, back);
    }

    #[test]
    fn picture_differs_and_caps_samples() {
        // 512 differing bytes -> stride 2 -> exactly MAX_DIFF_SAMPLES.
        let left_bytes = fake_png(8, 8, 0x00, 512);
        let right_bytes = fake_png(8, 8, 0xFF, 512);
        let left = write_temp("pic-c.png", &left_bytes);
        let right = write_temp("pic-d.png", &right_bytes);
        let summary = picture_compare(
            left.to_string_lossy().to_string(),
            right.to_string_lossy().to_string(),
        )
        .expect("differing compare");
        assert!(!summary.identical);
        assert!(summary.score < 1.0 && summary.score >= 0.0);
        assert!(summary.differing_bytes > 0);
        assert!(summary.samples.len() <= MAX_DIFF_SAMPLES);
        assert!(summary.truncated);
        assert_eq!(summary.samples.len(), MAX_DIFF_SAMPLES);
        let json = serde_json::to_string(&summary).expect("serialize");
        let back: PictureSummary = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(summary, back);
    }

    #[test]
    fn png_dimensions_sniff() {
        let bytes = fake_png(16, 32, 0, 8);
        assert_eq!(png_dimensions(&bytes), Some((16, 32)));
        assert_eq!(png_dimensions(b"not a png"), None);
        assert_eq!(png_dimensions(&bytes[..10]), None);
    }

    #[test]
    fn audio_missing_files_error() {
        let err = audio_compare(
            "/definitely/missing/left-1d.wav".to_string(),
            "/definitely/missing/right-1d.wav".to_string(),
            None,
        )
        .expect_err("missing files must fail");
        assert!(err.contains("path not found"), "unexpected: {err}");
    }

    #[test]
    fn audio_rejects_bad_tolerance() {
        let left = write_temp("tol-l.wav", &mono_wav(8000, &[0, 0]));
        let l = left.to_string_lossy().to_string();
        assert!(audio_compare(l.clone(), l.clone(), Some(-1.0)).is_err());
        assert!(audio_compare(l.clone(), l.clone(), Some(f32::NAN)).is_err());
        assert!(audio_compare(l.clone(), l, Some(f32::INFINITY)).is_err());
    }

    #[test]
    fn audio_rejects_non_wav() {
        let left = write_temp("notaudio.bin", b"this is not audio");
        let right = write_temp("notaudio2.bin", b"neither is this");
        let err = audio_compare(
            left.to_string_lossy().to_string(),
            right.to_string_lossy().to_string(),
            None,
        )
        .expect_err("non-WAV must fail");
        assert!(err.contains("not a decodable WAV"), "unexpected: {err}");
    }

    #[test]
    fn audio_identical_and_tolerance() {
        let samples = [0i16, 1000, -1000, 8000];
        let left = write_temp("aud-l.wav", &mono_wav(8000, &samples));
        let right = write_temp("aud-r.wav", &mono_wav(8000, &samples));
        let same = audio_compare(
            left.to_string_lossy().to_string(),
            right.to_string_lossy().to_string(),
            None,
        )
        .expect("identical compare");
        assert!(same.identical);
        assert!(!same.config_mismatch);
        assert_eq!(same.score, 1.0);
        assert_eq!(same.first_diff_at, None);
        let json = serde_json::to_string(&same).expect("serialize");
        let back: AudioSummary = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(same, back);

        let mut drifted = samples;
        drifted[1] = drifted[1].saturating_add(1);
        let drifted_path = write_temp("aud-d.wav", &mono_wav(8000, &drifted));
        let strict = audio_compare(
            left.to_string_lossy().to_string(),
            drifted_path.to_string_lossy().to_string(),
            None,
        )
        .expect("strict compare");
        assert!(!strict.identical);
        assert!(strict.first_diff_at.is_some());
        let relaxed = audio_compare(
            left.to_string_lossy().to_string(),
            drifted_path.to_string_lossy().to_string(),
            Some(0.001),
        )
        .expect("relaxed compare");
        assert!(relaxed.identical);
    }

    #[test]
    fn audio_config_mismatch_is_honest() {
        let left = write_temp("aud-8k.wav", &mono_wav(8000, &[0, 1, 2, 3]));
        let right = write_temp("aud-16k.wav", &mono_wav(16000, &[0, 1, 2, 3]));
        let summary = audio_compare(
            left.to_string_lossy().to_string(),
            right.to_string_lossy().to_string(),
            None,
        )
        .expect("mismatch compare");
        assert!(summary.config_mismatch);
        assert!(!summary.identical);
        assert_eq!(summary.windows_total, 0);
        assert_eq!(summary.first_diff_at, None);
    }
}
