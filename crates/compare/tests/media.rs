//! Tests for audio file comparison over the filesystem: error
//! mapping for missing and undecodable files, difference statistics
//! for real WAV fixtures, and the honest config-mismatch report.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

mod common;

use autocompare_compare::{Error, compare_audio_files};
use common::{cleanup, temp_root, write_bytes};
use std::path::{Path, PathBuf};

/// A well-formed RIFF chunk: id, little-endian size, payload, even
/// padding.
fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + payload.len() + 1);
    out.extend_from_slice(id);
    out.extend_from_slice(
        &u32::try_from(payload.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
    out
}

/// WAV bytes for a 16-bit mono stream at `rate` with the given
/// signed samples.
fn wav16(rate: u32, samples: &[i16]) -> Vec<u8> {
    let mut data = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        data.extend_from_slice(&sample.to_le_bytes());
    }
    let fmt = {
        let mut body = Vec::with_capacity(16);
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&rate.to_le_bytes());
        body.extend_from_slice(&(rate * 2).to_le_bytes());
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&16u16.to_le_bytes());
        body
    };
    let body: Vec<u8> = [chunk(b"fmt ", &fmt), chunk(b"data", &data)].concat();
    let mut out = Vec::with_capacity(12 + body.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(
        &u32::try_from(4 + body.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&body);
    out
}

/// Writes a WAV fixture and returns its path.
fn write_wav(dir: &Path, name: &str, rate: u32, samples: &[i16]) -> PathBuf {
    let path = dir.join(name);
    write_bytes(&path, &wav16(rate, samples));
    path
}

/// Eight quiet frames of a mono 8000 Hz stream.
const BASE: [i16; 8] = [100, 200, 300, 400, 500, 600, 700, 800];

#[test]
fn missing_file_reports_missing_with_path() {
    let dir = temp_root();
    let left = write_wav(&dir, "left.wav", 8000, &BASE);
    let right = dir.join("does-not-exist.wav");
    let err = compare_audio_files(&left, &right)
        .err()
        .expect("missing file must fail");
    assert!(matches!(err, Error::Missing { .. }));
    assert!(err.to_string().contains(&right.display().to_string()));
    cleanup(&dir);
}

#[test]
fn non_audio_file_is_unsupported() {
    let dir = temp_root();
    let left = write_wav(&dir, "left.wav", 8000, &BASE);
    let right = dir.join("text.wav");
    write_bytes(&right, b"hello, i am not audio");
    let err = compare_audio_files(&left, &right)
        .err()
        .expect("undecodable input must fail");
    assert!(matches!(err, Error::Unsupported { .. }));
    assert!(err.to_string().contains("not a decodable"));
    assert!(err.to_string().contains(&right.display().to_string()));
    cleanup(&dir);
}

#[test]
fn identical_wavs_report_identical() {
    let dir = temp_root();
    let left = write_wav(&dir, "left.wav", 8000, &BASE);
    let right = write_wav(&dir, "right.wav", 8000, &BASE);
    let cmp = compare_audio_files(&left, &right).expect("valid files must compare");
    assert!(cmp.diff.identical);
    assert!(!cmp.diff.config_mismatch);
    assert_eq!(cmp.diff.first_diff_at, None);
    assert_eq!(cmp.diff.windows_differing, 0);
    assert_eq!(cmp.diff.duration_l, 0.001);
    assert_eq!(cmp.diff.duration_r, 0.001);
    cleanup(&dir);
}

#[test]
fn one_differing_sample_is_reported() {
    let dir = temp_root();
    let mut changed = BASE;
    changed[3] = -400;
    let left = write_wav(&dir, "left.wav", 8000, &BASE);
    let right = write_wav(&dir, "right.wav", 8000, &changed);
    let cmp = compare_audio_files(&left, &right).expect("valid files must compare");
    assert!(!cmp.diff.identical);
    // Frame 3 of an 8000 Hz stream: 3/8000 s.
    assert_eq!(cmp.diff.first_diff_at, Some(0.000375));
    assert_eq!(cmp.diff.windows_total, 0);
    assert_eq!(cmp.diff.windows_differing, 0);
    cleanup(&dir);
}

#[test]
fn config_mismatch_is_reported_not_errored() {
    let dir = temp_root();
    let left = write_wav(&dir, "left.wav", 8000, &BASE);
    let right = write_wav(&dir, "right.wav", 44100, &BASE);
    // A sample-rate mismatch is a diff result, not an error: the
    // comparison fields stay empty and nothing crashes.
    let cmp = compare_audio_files(&left, &right).expect("mismatched configs must compare");
    assert!(cmp.diff.config_mismatch);
    assert!(!cmp.diff.identical);
    assert_eq!(cmp.diff.first_diff_at, None);
    assert_eq!(cmp.diff.windows_total, 0);
    assert_eq!(cmp.diff.windows_differing, 0);
    assert_eq!(cmp.diff.sample_rate_l, 8000);
    assert_eq!(cmp.diff.sample_rate_r, 44100);
    cleanup(&dir);
}
