//! Tests for PNG picture comparison over the filesystem: error
//! mapping for missing and undecodable files, difference statistics
//! for real PNG fixtures, and the declared heatmap file write.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use autocompare_compare::{Error, PictureOptions, compare_picture_files};
use autocompare_diff::{RgbaImage, decode_png, encode_png};
use common::{cleanup, temp_root, write_bytes};
use std::path::{Path, PathBuf};

/// Writes `image` as a PNG fixture and returns its path.
fn write_png(dir: &Path, name: &str, image: &RgbaImage) -> PathBuf {
    let path = dir.join(name);
    let bytes = encode_png(image).expect("fixture image must encode");
    write_bytes(&path, &bytes);
    path
}

/// Overwrites the pixel at `(x, y)` in a fixture image.
fn set_pixel(img: &mut RgbaImage, x: u32, y: u32, rgba: [u8; 4]) {
    let start = ((y as usize) * (img.width as usize) + x as usize) * 4;
    img.pixels
        .get_mut(start..start + 4)
        .expect("pixel must be inside the fixture")
        .copy_from_slice(&rgba);
}

/// Reads the pixel at `(x, y)` from a fixture image.
fn get_pixel(img: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    let start = ((y as usize) * (img.width as usize) + x as usize) * 4;
    img.pixels
        .get(start..start + 4)
        .expect("pixel must be inside the fixture")
        .try_into()
        .expect("a pixel slice is four bytes")
}

/// A 4x4 image with one pixel differing at (1, 2), and its base.
fn fixture_pair() -> (RgbaImage, RgbaImage) {
    let base = RgbaImage {
        width: 4,
        height: 4,
        pixels: [30, 60, 90, 255].repeat(16),
    };
    let mut changed = base.clone();
    set_pixel(&mut changed, 1, 2, [30, 60, 91, 255]);
    (base, changed)
}

#[test]
fn missing_file_reports_missing_with_path() {
    let dir = temp_root();
    let (base, _) = fixture_pair();
    let left = write_png(&dir, "left.png", &base);
    let right = dir.join("does-not-exist.png");
    let err = compare_picture_files(&left, &right, &PictureOptions::default())
        .err()
        .expect("missing file must fail");
    assert!(matches!(err, Error::Missing { .. }));
    assert!(err.to_string().contains(&right.display().to_string()));
    cleanup(&dir);
}

#[test]
fn non_png_file_is_unsupported() {
    let dir = temp_root();
    let (base, _) = fixture_pair();
    let left = write_png(&dir, "left.png", &base);
    let right = dir.join("text.png");
    write_bytes(&right, b"hello, i am not a png");
    let err = compare_picture_files(&left, &right, &PictureOptions::default())
        .err()
        .expect("non-PNG input must fail");
    assert!(matches!(err, Error::Unsupported { .. }));
    assert!(err.to_string().contains("not a decodable PNG"));
    assert!(err.to_string().contains(&right.display().to_string()));
    cleanup(&dir);
}

#[test]
fn identical_png_files_report_identical() {
    let dir = temp_root();
    let (base, _) = fixture_pair();
    let left = write_png(&dir, "left.png", &base);
    let right = write_png(&dir, "right.png", &base);
    let cmp = compare_picture_files(&left, &right, &PictureOptions::default())
        .expect("identical files must compare");
    assert!(cmp.diff.identical);
    assert_eq!(cmp.diff.differing, 0);
    assert_eq!(cmp.diff.bbox, None);
    assert_eq!(cmp.diff.total, 16);
    cleanup(&dir);
}

#[test]
fn one_differing_pixel_is_reported() {
    let dir = temp_root();
    let (base, changed) = fixture_pair();
    let left = write_png(&dir, "left.png", &base);
    let right = write_png(&dir, "right.png", &changed);
    let cmp = compare_picture_files(&left, &right, &PictureOptions::default())
        .expect("valid files must compare");
    assert!(!cmp.diff.identical);
    assert_eq!(cmp.diff.differing, 1);
    assert_eq!(cmp.diff.total, 16);
    assert_eq!(cmp.diff.bbox, Some((1, 2, 1, 2)));
    cleanup(&dir);
}

#[test]
fn heatmap_option_writes_a_decodable_png() {
    let dir = temp_root();
    let (base, changed) = fixture_pair();
    let left = write_png(&dir, "left.png", &base);
    let right = write_png(&dir, "right.png", &changed);
    let heatmap = dir.join("heat.png");
    let opts = PictureOptions {
        heatmap: Some(heatmap.clone()),
    };
    let cmp =
        compare_picture_files(&left, &right, &opts).expect("comparison with heatmap must work");
    assert!(!cmp.diff.identical);
    let bytes = std::fs::read(&heatmap).expect("heatmap file must exist");
    let heat = decode_png(&bytes).expect("heatmap must decode as PNG");
    assert_eq!(heat.width, 4);
    assert_eq!(heat.height, 4);
    // The differing pixel (1, 2) is opaque red; identical pixels are
    // dimmed grayscale of the base fill (30, 60, 90): luma
    // 0.299*30 + 0.587*60 + 0.114*90 = 54.45; *30/100 = 16.335 -> 16.
    assert_eq!(get_pixel(&heat, 1, 2), [255, 0, 0, 255]);
    assert_eq!(get_pixel(&heat, 0, 0), [16, 16, 16, 255]);
    cleanup(&dir);
}

#[test]
fn heatmap_written_for_identical_files_too() {
    let dir = temp_root();
    let (base, _) = fixture_pair();
    let left = write_png(&dir, "left.png", &base);
    let right = write_png(&dir, "right.png", &base);
    let heatmap = dir.join("heat.png");
    let opts = PictureOptions {
        heatmap: Some(heatmap.clone()),
    };
    let cmp = compare_picture_files(&left, &right, &opts).expect("must compare");
    assert!(cmp.diff.identical);
    assert!(heatmap.is_file(), "heatmap is written even when identical");
    assert!(decode_png(&std::fs::read(&heatmap).unwrap()).is_some());
    cleanup(&dir);
}

#[test]
fn heatmap_at_unwritable_path_is_io_error() {
    let dir = temp_root();
    let (base, changed) = fixture_pair();
    let left = write_png(&dir, "left.png", &base);
    let right = write_png(&dir, "right.png", &changed);
    // A directory path: fs::write must fail with an I/O error, and
    // the comparison must report it instead of panicking.
    std::fs::create_dir_all(dir.join("blocked.png")).expect("fixture dir must be created");
    let opts = PictureOptions {
        heatmap: Some(dir.join("blocked.png")),
    };
    let err = compare_picture_files(&left, &right, &opts)
        .err()
        .expect("an unwritable heatmap path must fail");
    assert!(matches!(err, Error::Io { .. }));
    assert!(err.to_string().contains("blocked.png"));
    cleanup(&dir);
}
