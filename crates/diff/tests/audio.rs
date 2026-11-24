//! Tests for WAV/MP3 decoding and PCM comparison. WAV fixtures are
//! hand-built RIFF bytes (a small pure helper, no I/O), so every
//! hostile case — lying chunk sizes, hostile fmt chunks, truncated
//! data, declared sizes past the cap — is asserted exactly as
//! documented. MP3 bytes cannot be synthesized by hand; when a
//! local ffmpeg exists it generates one tiny silent fixture once,
//! otherwise those tests skip and the garbage-input guarantees
//! still hold.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

use std::process::Command;

use autocompare_diff::{MAX_SAMPLES, PcmAudio, decode_audio, decode_mp3, decode_wav, diff_audio};

/// Little-endian bytes of a u16.
fn le16(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

/// Little-endian bytes of a u32.
fn le32(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// A well-formed RIFF chunk: id, little-endian size, payload, even
/// padding.
fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + payload.len() + 1);
    out.extend_from_slice(id);
    out.extend_from_slice(&le32(u32::try_from(payload.len()).unwrap_or(u32::MAX)));
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
    out
}

/// A chunk whose declared size is whatever the caller says, with the
/// payload bytes as given; for hostile fixtures.
fn lying_chunk(id: &[u8; 4], declared: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(id);
    out.extend_from_slice(&le32(declared));
    out.extend_from_slice(payload);
    out
}

/// A 16-byte WAV `fmt` payload (byte rate and block align recomputed
/// by the decoder; only format, channels, rate and bits matter).
fn fmt_body(format: u16, channels: u16, rate: u32, bits: u16) -> Vec<u8> {
    let mut body = Vec::with_capacity(16);
    body.extend_from_slice(&le16(format));
    body.extend_from_slice(&le16(channels));
    body.extend_from_slice(&le32(rate));
    body.extend_from_slice(&le32(rate * u32::from(channels) * u32::from(bits) / 8));
    body.extend_from_slice(&le16(channels.wrapping_mul(bits) / 8));
    body.extend_from_slice(&le16(bits));
    body
}

/// RIFF/WAVE container bytes around the given chunks; the RIFF size
/// field is honest by default and a lie only when overridden.
fn riff(parts: &[Vec<u8>], size_override: Option<u32>) -> Vec<u8> {
    let body: Vec<u8> = parts.concat();
    let size = size_override.unwrap_or(u32::try_from(4 + body.len()).unwrap_or(u32::MAX));
    let mut out = Vec::with_capacity(12 + body.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&le32(size));
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&body);
    out
}

/// A minimal valid WAV: fmt (format, channels, rate, bits) plus data.
fn wav(format: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
    riff(
        &[
            chunk(b"fmt ", &fmt_body(format, channels, rate, bits)),
            chunk(b"data", data),
        ],
        None,
    )
}

/// A decoded WAV fixture, expected to be valid.
fn decoded(bytes: &[u8]) -> PcmAudio {
    decode_wav(bytes).expect("fixture WAV must decode")
}

/// A hand-built PCM stream.
fn pcm(rate: u32, channels: u16, samples: Vec<f32>) -> PcmAudio {
    PcmAudio {
        sample_rate: rate,
        channels,
        samples,
    }
}

// ------------------------------------------------------------------
// decode_wav: accepted layouts
// ------------------------------------------------------------------

#[test]
fn decodes_sixteen_bit_mono_wav() {
    // Two frames: LE 0x0100 = 256 -> 256/32768, and 0x7FFF = 32767
    // -> 32767/32768. Both exact in f32.
    let bytes = wav(1, 1, 8000, 16, &[0x00, 0x01, 0xFF, 0x7F]);
    let audio = decoded(&bytes);
    assert_eq!(audio.sample_rate, 8000);
    assert_eq!(audio.channels, 1);
    let expected = [256.0 / 32768.0, 32767.0 / 32768.0];
    assert_eq!(audio.samples, expected);
}

#[test]
fn decodes_eight_bit_unsigned_wav() {
    // 8-bit samples are unsigned, centred on 128: 0 -> -1.0,
    // 128 -> 0.0, 255 -> 127/128.
    let bytes = wav(1, 1, 8000, 8, &[0, 128, 255]);
    let audio = decoded(&bytes);
    assert_eq!(audio.samples, [-1.0, 0.0, 127.0 / 128.0]);
}

#[test]
fn decodes_twentyfour_bit_wav() {
    // LE 0x000001 -> 1/8388608; LE 0xFFFFFF -> -1/8388608 (24-bit
    // two's complement, sign-extended).
    let bytes = wav(1, 1, 8000, 24, &[0x01, 0x00, 0x00, 0xFF, 0xFF, 0xFF]);
    let audio = decoded(&bytes);
    let scale = 8_388_608.0;
    assert_eq!(audio.samples, [1.0 / scale, -1.0 / scale]);
}

#[test]
fn decodes_thirtytwo_bit_integer_wav() {
    // LE 1 -> 2^-31, LE -1 -> -2^-31; both exact in f32.
    let bytes = wav(
        1,
        1,
        8000,
        32,
        &[0x01, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF],
    );
    let audio = decoded(&bytes);
    let scale = 2_147_483_648.0;
    assert_eq!(audio.samples, [1.0 / scale, -1.0 / scale]);
}

#[test]
fn decodes_float32_wav() {
    // IEEE float data passes through unchanged.
    let mut data = Vec::new();
    data.extend_from_slice(&0.5f32.to_le_bytes());
    data.extend_from_slice(&(-0.25f32).to_le_bytes());
    let bytes = wav(3, 1, 48000, 32, &data);
    let audio = decoded(&bytes);
    assert_eq!(audio.sample_rate, 48000);
    assert_eq!(audio.samples, [0.5, -0.25]);
}

#[test]
fn decodes_float64_wav() {
    // 64-bit floats are narrowed to f32; 0.25 is exact either way.
    let bytes = wav(3, 1, 48000, 64, &0.25f64.to_le_bytes());
    let audio = decoded(&bytes);
    assert_eq!(audio.samples, [0.25]);
}

#[test]
fn stereo_interleave_order_is_preserved() {
    // Frames of two LE u16 samples: L0=0, R0=1, L1=2, R1=3. The
    // output keeps frame-then-channel order.
    let data: Vec<u8> = [0u16, 1, 2, 3]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    let bytes = wav(1, 2, 44100, 16, &data);
    let audio = decoded(&bytes);
    assert_eq!(audio.channels, 2);
    let scale = 32768.0;
    assert_eq!(audio.samples, [0.0, 1.0 / scale, 2.0 / scale, 3.0 / scale]);
}

#[test]
fn four_channel_wav_decodes() {
    // Any channel count up to the cap of 64 is accepted.
    let data: Vec<u8> = [10u16, 20, 30, 40, 50, 60, 70, 80]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    let bytes = wav(1, 4, 44100, 16, &data);
    let audio = decoded(&bytes);
    assert_eq!(audio.channels, 4);
    assert_eq!(audio.samples.len(), 8);
    assert_eq!(audio.samples.first(), Some(&(10.0 / 32768.0)));
}

// ------------------------------------------------------------------
// decode_wav: hostile and lenient cases
// ------------------------------------------------------------------

#[test]
fn garbage_bytes_are_none() {
    assert_eq!(decode_wav(b"not a wav at all, just some text"), None);
    assert_eq!(decode_wav(b""), None);
}

#[test]
fn riff_without_wave_magic_is_none() {
    // "RIFF" size "JUNK": the form is wrong.
    let bytes = riff(&[chunk(b"fmt ", &fmt_body(1, 1, 8000, 16))], None);
    assert_eq!(decode_wav(&bytes), None);
}

#[test]
fn riff_without_fmt_chunk_is_none() {
    let bytes = riff(&[chunk(b"data", &[0, 0, 0, 0])], None);
    assert_eq!(decode_wav(&bytes), None);
}

#[test]
fn riff_without_data_chunk_is_none() {
    let bytes = riff(&[chunk(b"fmt ", &fmt_body(1, 1, 8000, 16))], None);
    assert_eq!(decode_wav(&bytes), None);
}

#[test]
fn hostile_fmt_chunks_are_none() {
    // Bits not in the accepted set.
    assert_eq!(decode_wav(&wav(1, 1, 8000, 12, &[0, 0])), None);
    // PCM with 64-bit samples is not an accepted layout.
    assert_eq!(decode_wav(&wav(1, 1, 8000, 64, &[0; 8])), None);
    // Float with 16-bit samples is not either.
    assert_eq!(decode_wav(&wav(3, 1, 8000, 16, &[0, 0])), None);
    // Zero channels.
    assert_eq!(decode_wav(&wav(1, 0, 8000, 16, &[0, 0])), None);
    // Absurd channel count (the cap is 64).
    assert_eq!(decode_wav(&wav(1, 65, 8000, 16, &[0; 130])), None);
    // An unknown format tag (WAVE_FORMAT_EXTENSIBLE included).
    assert_eq!(decode_wav(&wav(0xFFFE, 1, 8000, 16, &[0, 0])), None);
    // A fmt chunk shorter than 16 bytes.
    let short = lying_chunk(b"fmt ", 10, &fmt_body(1, 1, 8000, 16)[..10]);
    assert_eq!(decode_wav(&riff(&[short], None)), None);
    // A fmt chunk whose declared size walks past the end.
    let lying = lying_chunk(b"fmt ", 16, &fmt_body(1, 1, 8000, 16)[..8]);
    assert_eq!(decode_wav(&riff(&[lying], None)), None);
}

#[test]
fn unknown_chunk_with_lying_size_is_none() {
    // Any chunk except data that walks past the end of the buffer
    // breaks the file's own structure.
    let parts = [
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 16)),
        lying_chunk(b"LIST", 1000, b"INFO"),
    ];
    assert_eq!(decode_wav(&riff(&parts, None)), None);
}

#[test]
fn unknown_chunks_are_skipped_by_declared_size() {
    // A well-formed LIST chunk between fmt and data is ignored.
    let parts = [
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 16)),
        chunk(b"LIST", b"INFOISFTautocompare test"),
        chunk(b"data", &[0x01, 0x00, 0x02, 0x00]),
    ];
    let audio = decoded(&riff(&parts, None));
    let scale = 32768.0;
    assert_eq!(audio.samples, [1.0 / scale, 2.0 / scale]);
}

#[test]
fn a_later_invalid_fmt_chunk_is_ignored() {
    // The first fmt chunk wins; a second, hostile one is skipped
    // like any unknown chunk.
    let parts = [
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 16)),
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 12)),
        chunk(b"data", &[0x01, 0x00]),
    ];
    let audio = decoded(&riff(&parts, None));
    assert_eq!(audio.samples, [1.0 / 32768.0]);
}

#[test]
fn data_declaring_more_than_present_decodes_leniently() {
    // Declared 8 bytes, 4 present: the bytes actually there decode.
    let parts = [
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 16)),
        lying_chunk(b"data", 8, &[0x01, 0x00, 0xFF, 0x7F]),
    ];
    let audio = decoded(&riff(&parts, None));
    let scale = 32768.0;
    assert_eq!(audio.samples, [1.0 / scale, 32767.0 / scale]);
}

#[test]
fn data_declaring_less_than_present_uses_the_declaration() {
    // Declared 2 bytes, 4 present: only the claimed frame decodes
    // and the walk continues after it.
    let parts = [
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 16)),
        lying_chunk(b"data", 2, &[0x01, 0x00, 0x02, 0x00]),
    ];
    let audio = decoded(&riff(&parts, None));
    assert_eq!(audio.samples, [1.0 / 32768.0]);
}

#[test]
fn truncated_final_frame_is_dropped() {
    // Stereo, 16-bit: a frame is 4 bytes; the 5th byte is a partial
    // frame and decodes to nothing.
    let data = [0x01, 0x00, 0x02, 0x00, 0x03];
    let audio = decoded(&wav(1, 2, 8000, 16, &data));
    let scale = 32768.0;
    assert_eq!(audio.samples, [1.0 / scale, 2.0 / scale]);
}

#[test]
fn empty_data_chunk_decodes_to_no_samples() {
    // A data chunk with no bytes is a valid stream of zero frames.
    let bytes = wav(1, 1, 8000, 16, &[]);
    let audio = decoded(&bytes);
    assert_eq!(audio.sample_rate, 8000);
    assert!(audio.samples.is_empty());
}

#[test]
fn multiple_data_chunks_are_concatenated() {
    // Two data chunks, each with one 16-bit sample.
    let parts = [
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 16)),
        chunk(b"data", &[0x01, 0x00]),
        chunk(b"data", &[0x02, 0x00]),
    ];
    let audio = decoded(&riff(&parts, None));
    let scale = 32768.0;
    assert_eq!(audio.samples, [1.0 / scale, 2.0 / scale]);
}

#[test]
fn a_frame_may_straddle_a_data_chunk_boundary() {
    // 1.5 frames then the last byte: concatenation joins them.
    let parts = [
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 16)),
        chunk(b"data", &[0x01, 0x00, 0x02]),
        chunk(b"data", &[0x00]),
    ];
    let audio = decoded(&riff(&parts, None));
    let scale = 32768.0;
    assert_eq!(audio.samples, [1.0 / scale, 2.0 / scale]);
}

#[test]
fn a_stale_riff_size_field_is_ignored() {
    // Streaming writers leave the RIFF size stale or zero; the walk
    // follows actual chunk structure instead.
    let parts = [
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 16)),
        chunk(b"data", &[0x01, 0x00]),
    ];
    let audio = decoded(&riff(&parts, Some(0)));
    assert_eq!(audio.samples, [1.0 / 32768.0]);
}

// ------------------------------------------------------------------
// MAX_SAMPLES cap
// ------------------------------------------------------------------

#[test]
fn a_header_declaring_past_the_cap_is_none() {
    // 8-bit mono: a declared data size of MAX+1 bytes claims more
    // samples than the cap, even though almost none are present.
    assert_eq!(MAX_SAMPLES, 200_000_000);
    let declared = u32::try_from(MAX_SAMPLES + 1).expect("cap fits in u32");
    let parts = [
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 8)),
        lying_chunk(b"data", declared, &[0x00]),
    ];
    assert_eq!(decode_wav(&riff(&parts, None)), None);
}

#[test]
fn a_declaration_exactly_at_the_cap_stays_lenient() {
    // Declaring exactly the cap is not absurd; a truncated file
    // still decodes the bytes honestly present.
    let declared = u32::try_from(MAX_SAMPLES).expect("cap fits in u32");
    let parts = [
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 8)),
        lying_chunk(b"data", declared, &[0x80, 0x40]),
    ];
    let audio = decoded(&riff(&parts, None));
    assert_eq!(audio.samples, [0.0, -0.5]);
}

#[test]
fn an_overrunning_data_chunk_consumes_the_remaining_bytes() {
    // A data chunk declaring far more than present (but under the
    // cap) takes all bytes actually remaining — including the header
    // of the chunk behind it, which the lying size has swallowed —
    // and the walk ends there.
    let declared = u32::try_from(MAX_SAMPLES / 2 + 10).expect("cap fits in u32");
    let parts = [
        chunk(b"fmt ", &fmt_body(1, 1, 8000, 8)),
        lying_chunk(b"data", declared, &[0x40]),
        lying_chunk(b"data", declared, &[0x80]),
    ];
    let audio = decoded(&riff(&parts, None));
    // The swallowed tail: the second chunk's header and payload,
    // decoded as 8-bit PCM just like the first payload byte.
    let mut tail = Vec::new();
    tail.extend_from_slice(b"data");
    tail.extend_from_slice(&declared.to_le_bytes());
    tail.push(0x80);
    let mut expected = vec![u8_sample(0x40)];
    expected.extend(tail.iter().copied().map(u8_sample));
    assert_eq!(audio.samples, expected);
}

/// The 8-bit unsigned PCM mapping the decoder applies.
fn u8_sample(byte: u8) -> f32 {
    (f32::from(byte) - 128.0) / 128.0
}

// ------------------------------------------------------------------
// decode_audio sniffing
// ------------------------------------------------------------------

#[test]
fn decode_audio_sniffs_riff_as_wav() {
    let bytes = wav(1, 1, 8000, 16, &[0x01, 0x00]);
    let audio = decode_audio(&bytes).expect("RIFF bytes go to the WAV parser");
    assert_eq!(audio.sample_rate, 8000);
    assert_eq!(audio.samples, [1.0 / 32768.0]);
}

#[test]
fn decode_audio_does_not_retry_riff_as_mp3() {
    // RIFF magic but not a decodable WAV: stays None, no silent MP3
    // retry.
    let bytes = riff(&[chunk(b"fmt ", &fmt_body(1, 1, 8000, 16))], None);
    assert_eq!(decode_audio(&bytes), None);
}

#[test]
fn decode_audio_garbage_is_none() {
    assert_eq!(decode_audio(b"plainly not audio at all"), None);
    assert_eq!(decode_audio(b""), None);
}

// ------------------------------------------------------------------
// decode_mp3
// ------------------------------------------------------------------

#[test]
fn mp3_garbage_is_none() {
    assert_eq!(
        decode_mp3(b"hello world, definitely not an mpeg stream"),
        None
    );
    assert_eq!(decode_mp3(&[]), None);
}

#[test]
fn mp3_id3_tag_only_is_none() {
    // An ID3v2 header with no frames behind it: nothing decodes.
    assert_eq!(decode_mp3(b"ID3\x04\x00\x00\x00\x00\x00\x00"), None);
}

#[test]
fn ffmpeg_silence_decodes_when_ffmpeg_exists() {
    // A guarded fixture: without a working ffmpeg on this machine the
    // test skips (command errors return early); with one, a tiny
    // silent mono 44100 Hz MP3 must fully decode.
    let out_path = std::env::temp_dir().join(format!("autocompare-mp3-{}.mp3", std::process::id()));
    let status = Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-y")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("anullsrc=r=44100:cl=mono")
        .arg("-t")
        .arg("0.1")
        .arg("-codec:a")
        .arg("libmp3lame")
        .arg("-b:a")
        .arg("32k")
        .arg(&out_path)
        .status();
    if !matches!(status, Ok(exit) if exit.success()) {
        // ffmpeg missing or failed: fixture unavailable, skip.
        return;
    }
    let bytes = match std::fs::read(&out_path) {
        Ok(bytes) => bytes,
        Err(_) => return,
    };
    let _ = std::fs::remove_file(&out_path);
    let audio = decode_mp3(&bytes).expect("an ffmpeg-encoded MP3 must decode");
    assert_eq!(audio.sample_rate, 44100);
    assert_eq!(audio.channels, 1);
    assert!(!audio.samples.is_empty());
    // Interleaved samples are whole frames.
    assert_eq!(audio.samples.len() % usize::from(audio.channels), 0);
    // Silence decodes to (numerically) zero, allowing encoder noise.
    assert!(audio.samples.iter().all(|sample| sample.abs() <= 0.001));
}

// ------------------------------------------------------------------
// diff_audio
// ------------------------------------------------------------------

#[test]
fn identical_streams_report_no_difference() {
    let a = pcm(8000, 1, vec![0.25; 1600]);
    let b = pcm(8000, 1, vec![0.25; 1600]);
    let diff = diff_audio(&a, &b);
    assert!(diff.identical);
    assert!(!diff.config_mismatch);
    assert_eq!(diff.first_diff_at, None);
    assert_eq!(diff.windows_total, 20);
    assert_eq!(diff.windows_differing, 0);
    assert_eq!(diff.duration_l, 0.2);
    assert_eq!(diff.duration_r, 0.2);
}

#[test]
fn first_difference_is_reported_in_seconds_and_windows() {
    // 1000 Hz mono, 1000 frames: frame 55 differs, so the first
    // difference is 55/1000 s in, one 10 ms window of 100.
    let a = pcm(1000, 1, vec![0.0; 1000]);
    let mut b_samples = vec![0.0; 1000];
    b_samples[55] = 0.5;
    let b = pcm(1000, 1, b_samples);
    let diff = diff_audio(&a, &b);
    assert!(!diff.identical);
    assert_eq!(diff.first_diff_at, Some(0.055));
    assert_eq!(diff.windows_total, 100);
    assert_eq!(diff.windows_differing, 1);
}

#[test]
fn a_frame_with_any_differing_channel_counts_once() {
    // One stereo frame with both channels differing is still one
    // differing frame and one differing window.
    let a = pcm(1000, 2, vec![0.0; 2000]);
    let mut b_samples = vec![0.0; 2000];
    b_samples[0] = 0.1;
    b_samples[1] = -0.1;
    let b = pcm(1000, 2, b_samples);
    let diff = diff_audio(&a, &b);
    assert_eq!(diff.first_diff_at, Some(0.0));
    assert_eq!(diff.windows_total, 100);
    assert_eq!(diff.windows_differing, 1);
}

#[test]
fn config_mismatch_reports_header_stats_only() {
    let a = pcm(8000, 1, vec![0.5; 2]);
    let b = pcm(44100, 2, vec![0.5; 44100]);
    let diff = diff_audio(&a, &b);
    assert!(diff.config_mismatch);
    assert!(!diff.identical);
    assert_eq!(diff.first_diff_at, None);
    assert_eq!(diff.windows_total, 0);
    assert_eq!(diff.windows_differing, 0);
    assert_eq!(diff.sample_rate_l, 8000);
    assert_eq!(diff.sample_rate_r, 44100);
    assert_eq!(diff.channels_l, 1);
    assert_eq!(diff.channels_r, 2);
    assert_eq!(diff.duration_l, 0.00025);
    assert_eq!(diff.duration_r, 0.5);
}

#[test]
fn different_lengths_compare_over_the_common_span() {
    // Equal prefix, right stream shorter: no differing sample, but
    // the streams are not identical, and the windows cover only the
    // common span.
    let a = pcm(1000, 1, vec![0.25; 1000]);
    let b = pcm(1000, 1, vec![0.25; 500]);
    let diff = diff_audio(&a, &b);
    assert!(!diff.identical);
    assert_eq!(diff.first_diff_at, None);
    assert_eq!(diff.windows_total, 50);
    assert_eq!(diff.windows_differing, 0);
    assert_eq!(diff.duration_l, 1.0);
    assert_eq!(diff.duration_r, 0.5);
}

#[test]
fn scattered_differences_count_their_windows() {
    // Frames 5 and 6 share window 0; 25 is window 2; 999 is window
    // 99: three differing windows of a hundred, first at 5 ms.
    let a = pcm(1000, 1, vec![0.0; 1000]);
    let mut b_samples = vec![0.0; 1000];
    b_samples[5] = 1.0;
    b_samples[6] = 1.0;
    b_samples[25] = 1.0;
    b_samples[999] = 1.0;
    let b = pcm(1000, 1, b_samples);
    let diff = diff_audio(&a, &b);
    assert_eq!(diff.first_diff_at, Some(0.005));
    assert_eq!(diff.windows_differing, 3);
    assert_eq!(diff.windows_total, 100);
}

#[test]
fn a_difference_in_a_trailing_partial_window_shows_only_in_first_diff() {
    // 15 frames at 1000 Hz: one full 10-frame window, a trailing
    // partial one. A difference at frame 14 cannot be attributed to
    // a counted window, but the first-difference time is honest.
    let a = pcm(1000, 1, vec![0.0; 15]);
    let mut b_samples = vec![0.0; 15];
    b_samples[14] = 0.5;
    let b = pcm(1000, 1, b_samples);
    let diff = diff_audio(&a, &b);
    assert!(!diff.identical);
    assert_eq!(diff.windows_total, 1);
    assert_eq!(diff.windows_differing, 0);
    assert_eq!(diff.first_diff_at, Some(0.014));
}

#[test]
fn zero_sample_rate_reports_zero_duration_and_no_seconds() {
    // A zero rate on both sides is not a mismatch, but no duration or
    // first-difference seconds can be expressed. Windows fall back
    // to one frame each.
    let a = pcm(0, 1, vec![0.0, 0.0]);
    let b = pcm(0, 1, vec![0.0, 0.5]);
    let diff = diff_audio(&a, &b);
    assert!(!diff.config_mismatch);
    assert!(!diff.identical);
    assert_eq!(diff.duration_l, 0.0);
    assert_eq!(diff.duration_r, 0.0);
    assert_eq!(diff.first_diff_at, None);
    assert_eq!(diff.windows_total, 2);
    assert_eq!(diff.windows_differing, 1);
}

#[test]
fn zero_channels_compare_by_sample_count() {
    // Frames are undefined without channels: only the sample counts
    // decide, and no windows exist.
    let a = pcm(8000, 0, vec![0.0]);
    let b = pcm(8000, 0, vec![0.0]);
    let diff = diff_audio(&a, &b);
    assert!(diff.identical);
    assert_eq!(diff.windows_total, 0);

    let c = pcm(8000, 0, vec![0.0, 0.0]);
    let diff = diff_audio(&a, &c);
    assert!(!diff.identical);
    assert_eq!(diff.first_diff_at, None);
    assert_eq!(diff.windows_total, 0);
    assert_eq!(diff.windows_differing, 0);
}

#[test]
fn durations_are_counted_in_frames() {
    // Stereo: 16 samples are 8 frames, not 16.
    let a = pcm(8000, 2, vec![0.1; 16]);
    let diff = diff_audio(&a, &a);
    assert_eq!(diff.duration_l, 0.001);
    assert_eq!(diff.duration_r, 0.001);
}

// ------------------------------------------------------------------
// Totality sweeps: hostile bytes must never panic, whatever the
// decoder answers
// ------------------------------------------------------------------

/// A small deterministic LCG for hostile fixture bytes.
struct Lcg(u64);

impl Lcg {
    fn next_byte(&mut self) -> u8 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u8
    }
}

#[test]
fn every_truncation_of_a_valid_wav_terminates() {
    // Cut the stream at every byte: each prefix is either a WAV or
    // None, never a panic. The WAV parser sees the prefixes through
    // the sniffer too.
    let mut data = Vec::new();
    for frame in 0..6u16 {
        let left = f32::from(frame) * 0.25;
        let right = -left;
        data.extend_from_slice(&left.to_le_bytes());
        data.extend_from_slice(&right.to_le_bytes());
    }
    let bytes = wav(3, 2, 48000, 32, &data);
    for end in 0..=bytes.len() {
        let prefix = &bytes[..end];
        let _ = decode_wav(prefix);
        let _ = decode_audio(prefix);
    }
}

#[test]
fn pseudo_random_bytes_never_panic_the_decoders() {
    // High-entropy buffers, straight (MP3 sniffer path) and wearing
    // a RIFF/WAVE header (WAV parser path): every answer is Some or
    // None, never a crash.
    let mut lcg = Lcg(0x1118_7ea1_2026_c0de);
    for _ in 0..400 {
        let len = (usize::from(lcg.next_byte()) * 3 + usize::from(lcg.next_byte())) % 700;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(lcg.next_byte());
        }
        let _ = decode_audio(&buf);
        let mut riffed = Vec::with_capacity(len + 12);
        riffed.extend_from_slice(b"RIFF");
        riffed.extend_from_slice(&u32::try_from(len as u64 + 4).unwrap_or(0).to_le_bytes());
        riffed.extend_from_slice(b"WAVE");
        riffed.extend_from_slice(&buf);
        let _ = decode_wav(&riffed);
        let _ = decode_audio(&riffed);
    }
}
