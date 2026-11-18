//! WAV and MP3 decoding plus PCM comparison.
//!
//! Pure: bytes in, samples out; no I/O, no panics. Undecodable input
//! is `None`, never an error; decoding is capped by [`MAX_SAMPLES`]
//! so hostile headers cannot exhaust memory.
//!
//! WAV is parsed by hand — a RIFF chunk walk over the raw bytes, no
//! third-party code — because the format is small and total handling
//! of lying headers is easier to prove in a parser we own. MP3 goes
//! through the pure-Rust symphonia bundle. Both paths are best-effort
//! but honest: a file that claims more content than it carries
//! decodes only the bytes actually present, and a file whose
//! structure cannot be trusted at all yields `None`.

use std::io::Cursor;

use symphonia_bundle_mp3::{MpaDecoder, MpaReader};
use symphonia_core::codecs::CodecParameters;
use symphonia_core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia_core::formats::{FormatOptions, FormatReader};
use symphonia_core::io::MediaSourceStream;

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
///
/// Total and lenient by design. The chunk walk trusts declared sizes
/// only as far as the bytes agree: any chunk except `data` whose
/// declared size walks past the end of the buffer makes the file
/// undecodable, while a `data` chunk that declares more than remains
/// contributes the bytes actually present (and one that declares
/// less contributes only what it claims). Multiple `data` chunks are
/// concatenated in order, so a frame may straddle a chunk boundary. A
/// final frame truncated mid-sample is dropped; trailing bytes too
/// short for another chunk header end the walk. The first `fmt`
/// chunk is the one that counts and must validate; later ones are
/// skipped like unknown chunks. Declared and actual data lengths are
/// both capped through [`MAX_SAMPLES`] — an absurd claim is rejected
/// even when the bytes are not there to match it. The RIFF header's
/// own size field is ignored (streaming writers leave it stale), and
/// `byte_rate` and `block_align` are ignored in favour of the layout
/// recomputed from channels and bits per sample.
pub fn decode_wav(bytes: &[u8]) -> Option<PcmAudio> {
    // Container magic: "RIFF" <u32 size> "WAVE".
    if bytes.get(0..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return None;
    }
    let mut format: Option<WavFormat> = None;
    let mut data_seen = false;
    let mut data_ranges: Vec<(usize, usize)> = Vec::new();
    let mut data_declarations: Vec<u64> = Vec::new();
    // Walk chunks: 4-byte id + 4-byte little-endian size + payload,
    // padded to an even byte offset.
    let mut pos = 12usize;
    while let Some(header_end) = pos.checked_add(8).filter(|end| *end <= bytes.len()) {
        let id: [u8; 4] = bytes.get(pos..header_end - 4)?.try_into().ok()?;
        let size_bytes: [u8; 4] = bytes.get(header_end - 4..header_end)?.try_into().ok()?;
        let declared = u64::from(u32::from_le_bytes(size_bytes));
        let payload = header_end;
        let declared_end = payload.checked_add(usize::try_from(declared).ok()?)?;
        if id == *b"data" {
            // Lenient data: contribute the bytes actually present when
            // the declaration overruns, or only the declaration when
            // it is the shorter one.
            data_seen = true;
            data_ranges.push((payload, declared_end.min(bytes.len())));
            data_declarations.push(declared);
        } else if declared_end > bytes.len() {
            // Any other chunk lying about its size breaks the walk:
            // the file's own structure cannot be trusted.
            return None;
        } else if id == *b"fmt " && format.is_none() {
            // First fmt chunk wins and must validate; a later one is
            // skipped like any unknown chunk.
            format = Some(parse_fmt(bytes.get(payload..declared_end)?)?);
        }
        // Advance past the payload, then to the next even offset. A
        // missing final pad byte, or a ragged tail, simply ends the
        // walk on the next turn.
        let pad = usize::try_from(declared & 1).ok()?;
        pos = declared_end
            .checked_add(pad)
            .filter(|next| *next <= bytes.len())
            .unwrap_or(bytes.len());
    }
    let format = format?;
    if !data_seen {
        return None;
    }
    // The cap applies to the declared data length and to the honest
    // (possibly clamped) one alike: a header claiming more than
    // [`MAX_SAMPLES`] samples is undecodable, never half-trusted.
    let frame_bytes =
        u64::from(format.channels).checked_mul(u64::from(format.kind.bytes_per_sample()))?;
    for declared in data_declarations.iter().copied() {
        if implied_samples(declared, frame_bytes, format.channels)? > MAX_SAMPLES as u64 {
            return None;
        }
    }
    let mut total = 0u64;
    for (start, end) in data_ranges.iter().copied() {
        total = total.checked_add(u64::try_from(end.saturating_sub(start)).ok()?)?;
    }
    if implied_samples(total, frame_bytes, format.channels)? > MAX_SAMPLES as u64 {
        return None;
    }
    // One data chunk (the common case) is read in place; several are
    // joined into one buffer so frames may straddle chunk borders.
    let joined: Option<Vec<u8>> = if data_ranges.len() > 1 {
        let len = usize::try_from(total).ok()?;
        let mut joined = Vec::new();
        joined.try_reserve_exact(len).ok()?;
        for (start, end) in data_ranges.iter().copied() {
            joined.extend_from_slice(bytes.get(start..end)?);
        }
        Some(joined)
    } else {
        None
    };
    let empty: &[u8] = &[];
    let data = match (joined.as_deref(), data_ranges.first()) {
        (Some(joined), _) => joined,
        (None, Some((start, end))) => bytes.get(*start..*end).unwrap_or(empty),
        (None, None) => empty,
    };
    // Frame-wise conversion; chunks_exact drops a truncated tail.
    let frame_bytes = usize::from(format.channels)
        .checked_mul(usize::from(format.kind.bytes_per_sample()))?
        .max(1);
    let frames = data.len() / frame_bytes;
    let sample_count = frames.checked_mul(usize::from(format.channels))?;
    let mut samples: Vec<f32> = Vec::new();
    samples.try_reserve_exact(sample_count).ok()?;
    for frame in data.chunks_exact(frame_bytes) {
        push_frame(&mut samples, frame, &format)?;
    }
    Some(PcmAudio {
        sample_rate: format.sample_rate,
        channels: format.channels,
        samples,
    })
}

/// Decodes MP3 bytes (MPEG-1/2/2.5 audio, layers I/II/III, via the
/// pure-Rust symphonia decoder). `None` when the bytes are not a
/// decodable MP3.
///
/// Every packet is decoded to the end of the stream and the frames
/// are concatenated. Corrupt packets are skipped and decoding
/// continues — honest best-effort — but a stream from which not a
/// single packet decodes is `None`. A demux failure ends the decode
/// with whatever decoded so far. Sample rate and channels come from
/// the track's codec parameters (filled from the first frame header
/// at probe time), or from the first decoded buffer when the
/// parameters are silent; a successfully decoded buffer's spec
/// always wins, since it describes the samples actually kept.
pub fn decode_mp3(bytes: &[u8]) -> Option<PcmAudio> {
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let mut reader = MpaReader::try_new(source, FormatOptions::default()).ok()?;
    let params = match reader
        .tracks()
        .first()
        .and_then(|track| track.codec_params.as_ref())
    {
        Some(CodecParameters::Audio(params)) => params.clone(),
        _ => return None,
    };
    let mut decoder = MpaDecoder::try_new(&params, &AudioDecoderOptions::default()).ok()?;
    let mut sample_rate = params.sample_rate;
    let mut channels = match params.channels {
        Some(channels) => u16::try_from(channels.count()).ok(),
        None => None,
    };
    let mut samples: Vec<f32> = Vec::new();
    let mut decoded_any = false;
    // A demux error ends the stream; a decode error skips the packet.
    while let Ok(Some(packet)) = reader.next_packet() {
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(_) => continue,
        };
        decoded_any = true;
        let spec = decoded.spec();
        if sample_rate.is_none() || channels.is_none() {
            sample_rate = Some(spec.rate());
            channels = Some(u16::try_from(spec.channels().count()).ok()?);
        }
        let count = decoded.samples_interleaved();
        let end = samples.len().checked_add(count)?;
        if end > MAX_SAMPLES {
            return None;
        }
        // Reserve before growing so a failed allocation is None, not
        // an abort; the resize then cannot reallocate.
        samples.try_reserve(count).ok()?;
        let start = samples.len();
        samples.resize(end, 0.0);
        let dst = samples.get_mut(start..end)?;
        decoded.copy_to_slice_interleaved(dst);
    }
    if !decoded_any {
        return None;
    }
    Some(PcmAudio {
        sample_rate: sample_rate?,
        channels: channels?,
        samples,
    })
}

/// Sniffs the container and decodes WAV or MP3. `None` for anything
/// else.
///
/// The sniff is a heuristic: RIFF bytes go to the WAV parser (a RIFF
/// file that is not a decodable WAV stays `None` — it is not
/// silently re-tried as MP3), everything else is offered to the MP3
/// decoder. Symphonia finds MPEG frame sync by scanning, so an MP3
/// wearing an ID3v2 tag decodes without special handling.
pub fn decode_audio(bytes: &[u8]) -> Option<PcmAudio> {
    if bytes.get(0..4) == Some(b"RIFF") {
        decode_wav(bytes)
    } else {
        decode_mp3(bytes)
    }
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
///
/// Samples are compared for exact equality (decoded PCM of identical
/// content is exactly equal). `first_diff_at` is the first frame
/// holding a differing sample, in seconds; with a sample rate of
/// zero no seconds can be expressed and the field stays `None`.
/// With zero channels on both sides frames are undefined, so the
/// streams compare only by sample count (`identical` when equal)
/// and the window counts are zero. Windows hold `rate / 100` frames,
/// at least one; only complete 10 ms windows are counted, so a
/// difference in a trailing partial window shows in `first_diff_at`
/// but not in the window counts. `identical` requires matching
/// configs, equal sample counts and no differing sample in the
/// common span.
pub fn diff_audio(a: &PcmAudio, b: &PcmAudio) -> AudioDiff {
    let frames_l = frame_count(a);
    let frames_r = frame_count(b);
    let duration_l = duration(frames_l, a);
    let duration_r = duration(frames_r, b);
    let config_mismatch = a.sample_rate != b.sample_rate || a.channels != b.channels;
    if config_mismatch {
        return AudioDiff {
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
    let mut windows_total = 0u64;
    let mut windows_differing = 0u64;
    let frames_per_window = (u64::from(a.sample_rate) / 100).max(1);
    if channels > 0 {
        let common_frames =
            u64::try_from(common.checked_div(channels).unwrap_or(0)).unwrap_or(u64::MAX);
        windows_total = common_frames / frames_per_window;
        // Sample-by-sample over the common span, no allocation: the
        // first differing sample gives the first differing frame,
        // and differing windows are counted as the walk moves
        // forward (window indices are non-decreasing).
        let mut last_window: Option<u64> = None;
        for (index, (left, right)) in a
            .samples
            .iter()
            .zip(b.samples.iter())
            .take(common)
            .enumerate()
        {
            if left == right {
                continue;
            }
            let frame = u64::try_from(index.checked_div(channels).unwrap_or(usize::MAX))
                .unwrap_or(u64::MAX);
            if first_frame.is_none() {
                first_frame = Some(frame);
            }
            let window = frame / frames_per_window;
            if window < windows_total && last_window != Some(window) {
                windows_differing += 1;
                last_window = Some(window);
            }
        }
    }
    let identical = first_frame.is_none() && a.samples.len() == b.samples.len();
    let first_diff_at = match (first_frame, a.sample_rate) {
        (Some(frame), rate) if rate > 0 => Some(frame as f64 / f64::from(rate)),
        _ => None,
    };
    AudioDiff {
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

/// Frames of a decoded stream: samples grouped by channel count. Zero
/// channels mean no frames (only possible in a hand-built stream).
fn frame_count(audio: &PcmAudio) -> u64 {
    if audio.channels == 0 {
        return 0;
    }
    let frames = audio.samples.len() / usize::from(audio.channels);
    u64::try_from(frames).unwrap_or(u64::MAX)
}

/// Duration in seconds; a zero rate or channel count has no
/// meaningful duration.
fn duration(frames: u64, audio: &PcmAudio) -> f64 {
    if audio.sample_rate == 0 || audio.channels == 0 {
        0.0
    } else {
        frames as f64 / f64::from(audio.sample_rate)
    }
}

/// Samples implied by a data length, for cap checks: whole frames
/// only, a truncated final frame decodes to nothing.
fn implied_samples(data_len: u64, frame_bytes: u64, channels: u16) -> Option<u64> {
    if frame_bytes == 0 {
        return None;
    }
    (data_len / frame_bytes).checked_mul(u64::from(channels))
}

/// The accepted WAV sample layouts.
#[derive(Clone, Copy)]
enum SampleKind {
    /// 8-bit unsigned PCM, centred on 128.
    PcmU8,
    /// 16-bit little-endian signed PCM.
    PcmI16,
    /// 24-bit little-endian signed PCM.
    PcmI24,
    /// 32-bit little-endian signed PCM.
    PcmI32,
    /// 32-bit little-endian IEEE float.
    Float32,
    /// 64-bit little-endian IEEE float, narrowed.
    Float64,
}

impl SampleKind {
    /// Bytes one sample of this kind occupies.
    fn bytes_per_sample(self) -> u16 {
        match self {
            SampleKind::PcmU8 => 1,
            SampleKind::PcmI16 => 2,
            SampleKind::PcmI24 => 3,
            SampleKind::PcmI32 | SampleKind::Float32 => 4,
            SampleKind::Float64 => 8,
        }
    }
}

/// A validated WAV `fmt` chunk.
struct WavFormat {
    sample_rate: u32,
    channels: u16,
    kind: SampleKind,
}

/// Parses and validates a `fmt` payload: format 1 (integer PCM) with
/// 8/16/24/32 bits, format 3 (IEEE float) with 32/64 bits, channels
/// from 1 to 64. Anything else — WAVE_FORMAT_EXTENSIBLE included —
/// is `None`, never a guess. `byte_rate` and `block_align` are
/// ignored; the layout is recomputed from channels and bits.
fn parse_fmt(body: &[u8]) -> Option<WavFormat> {
    if body.len() < 16 {
        return None;
    }
    let format = u16::from_le_bytes(body.get(0..2)?.try_into().ok()?);
    let channels = u16::from_le_bytes(body.get(2..4)?.try_into().ok()?);
    let sample_rate = u32::from_le_bytes(body.get(4..8)?.try_into().ok()?);
    let bits = u16::from_le_bytes(body.get(14..16)?.try_into().ok()?);
    if channels == 0 || channels > 64 {
        return None;
    }
    let kind = match (format, bits) {
        (1, 8) => SampleKind::PcmU8,
        (1, 16) => SampleKind::PcmI16,
        (1, 24) => SampleKind::PcmI24,
        (1, 32) => SampleKind::PcmI32,
        (3, 32) => SampleKind::Float32,
        (3, 64) => SampleKind::Float64,
        _ => return None,
    };
    Some(WavFormat {
        sample_rate,
        channels,
        kind,
    })
}

/// Converts one frame of raw PCM bytes to interleaved f32 samples and
/// appends them to `out`. Values are normalised to roughly [-1, 1]:
/// integers divided by their full scale, floats as decoded (f64
/// narrowed to f32).
fn push_frame(out: &mut Vec<f32>, frame: &[u8], format: &WavFormat) -> Option<()> {
    let channels = usize::from(format.channels);
    let bytes = usize::from(format.kind.bytes_per_sample());
    for channel in 0..channels {
        let start = channel.checked_mul(bytes)?;
        let raw = frame.get(start..start.checked_add(bytes)?)?;
        let value = match format.kind {
            SampleKind::PcmU8 => (f32::from(raw.first().copied()?) - 128.0) / 128.0,
            SampleKind::PcmI16 => {
                f32::from(i16::from_le_bytes(raw.get(0..2)?.try_into().ok()?)) / 32768.0
            }
            SampleKind::PcmI24 => i24_from_le(raw)? as f32 / 8_388_608.0,
            SampleKind::PcmI32 => {
                i32::from_le_bytes(raw.get(0..4)?.try_into().ok()?) as f32 / 2_147_483_648.0
            }
            SampleKind::Float32 => f32::from_le_bytes(raw.get(0..4)?.try_into().ok()?),
            SampleKind::Float64 => f64::from_le_bytes(raw.get(0..8)?.try_into().ok()?) as f32,
        };
        out.push(value);
    }
    Some(())
}

/// A little-endian 24-bit signed integer, sign-extended from bit 23.
fn i24_from_le(raw: &[u8]) -> Option<i32> {
    let bits = u32::from(raw.first().copied()?)
        | u32::from(raw.get(1).copied()?) << 8
        | u32::from(raw.get(2).copied()?) << 16;
    if bits & 0x0080_0000 != 0 {
        Some((bits | 0xFF00_0000) as i32)
    } else {
        Some(bits as i32)
    }
}
