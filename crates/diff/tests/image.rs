//! Tests for PNG decode/encode, pixel comparison and heatmap
//! rendering. Fixtures are built through the public round-trip
//! (`encode_png` -> `decode_png`) and, for non-RGBA color types,
//! with the `png` crate directly. Hostile inputs — garbage bytes,
//! empty input, truncated streams, oversized declared dimensions —
//! must all yield `None`, never a panic.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

use autocompare_diff::{RgbaImage, decode_png, diff_pictures, encode_png, picture_heatmap};

/// A `width * height` image filled with one RGBA value.
fn solid(width: u32, height: u32, rgba: [u8; 4]) -> RgbaImage {
    RgbaImage {
        width,
        height,
        pixels: rgba.repeat((width as usize) * (height as usize)),
    }
}

/// Overwrites the pixel at `(x, y)`; test-only, dimensions assumed.
fn set(img: &mut RgbaImage, x: u32, y: u32, rgba: [u8; 4]) {
    let start = ((y as usize) * (img.width as usize) + x as usize) * 4;
    img.pixels[start..start + 4].copy_from_slice(&rgba);
}

/// Reads the pixel at `(x, y)`; test-only, dimensions assumed.
fn get(img: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    let start = ((y as usize) * (img.width as usize) + x as usize) * 4;
    [
        img.pixels[start],
        img.pixels[start + 1],
        img.pixels[start + 2],
        img.pixels[start + 3],
    ]
}

/// A small deterministic LCG for high-entropy fixture bytes.
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

/// Standard CRC-32 (IEEE 802.3 polynomial, bitwise) for hand-patched
/// chunks.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Encodes a small image directly with the `png` crate in a chosen
/// color type, for decode-normalisation fixtures.
fn raw_png(color: png::ColorType, width: u32, height: u32, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(color);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(data).unwrap();
    writer.finish().unwrap();
    out
}

#[test]
fn round_trip_preserves_pixels_and_dimensions() {
    let mut image = solid(3, 2, [1, 2, 3, 255]);
    set(&mut image, 1, 0, [4, 5, 6, 255]);
    set(&mut image, 0, 1, [10, 11, 12, 0]);
    set(&mut image, 2, 1, [13, 14, 15, 128]);
    let bytes = encode_png(&image).expect("a well-formed image must encode");
    let decoded = decode_png(&bytes).expect("our own output must decode");
    assert_eq!(decoded.width, 3);
    assert_eq!(decoded.height, 2);
    assert_eq!(decoded, image);
}

#[test]
fn decode_garbage_and_empty_input_is_none() {
    assert_eq!(decode_png(b"clearly not a png, just text"), None);
    assert_eq!(decode_png(b""), None);
    assert_eq!(decode_png(b"0123456789abcdef0123456789abcdef"), None);
}

#[test]
fn decode_truncated_png_is_none() {
    // High-entropy 32x32 pixels so the compressed stream is long and
    // cutting the file in half lands inside the image data.
    let mut lcg = Lcg(0x5eed);
    let image = RgbaImage {
        width: 32,
        height: 32,
        pixels: (0..32 * 32 * 4).map(|_| lcg.next_byte()).collect(),
    };
    let bytes = encode_png(&image).expect("entropy image must encode");
    let cut = bytes.len() / 2;
    assert!(cut > 64, "the cut must be past the header chunks");
    assert_eq!(decode_png(&bytes[..cut]), None);
    // Cutting the tail into the compressed data also fails the decode.
    // (The last 12 bytes alone are just IEND; the png crate tolerates a
    // missing IEND when the image data itself is complete, so the cut
    // has to reach the IDAT stream.)
    let tail = bytes.len() - 30;
    assert!(bytes.len() - tail > 12, "the cut must reach the IDAT data");
    assert_eq!(decode_png(&bytes[..tail]), None);
}

#[test]
fn decode_oversized_declared_dimensions_is_none() {
    // Take a real 4x4 PNG, patch the IHDR to claim 100000x100000 and
    // fix the chunk CRC so the header parses; the pixel-buffer cap
    // must refuse the frame instead of trying to allocate ~40 GB.
    let bytes = encode_png(&solid(4, 4, [9, 9, 9, 9])).expect("fixture must encode");
    let mut hostile = bytes.clone();
    hostile[16..20].copy_from_slice(&100_000u32.to_be_bytes());
    hostile[20..24].copy_from_slice(&100_000u32.to_be_bytes());
    let crc = crc32(&hostile[12..29]);
    hostile[29..33].copy_from_slice(&crc.to_be_bytes());
    // The patch is a well-formed header with absurd dimensions.
    assert_eq!(decode_png(&hostile), None);
}

#[test]
fn decode_rgb_png_gains_full_alpha() {
    let bytes = raw_png(png::ColorType::Rgb, 2, 1, &[10, 20, 30, 40, 50, 60]);
    let image = decode_png(&bytes).expect("RGB PNG must decode");
    assert_eq!(image.width, 2);
    assert_eq!(image.height, 1);
    assert_eq!(image.pixels, vec![10, 20, 30, 255, 40, 50, 60, 255]);
}

#[test]
fn decode_grayscale_png_expands_to_rgb_channels() {
    let bytes = raw_png(png::ColorType::Grayscale, 1, 1, &[77]);
    let image = decode_png(&bytes).expect("grayscale PNG must decode");
    assert_eq!(image.pixels, vec![77, 77, 77, 255]);
}

#[test]
fn decode_gray_alpha_png_keeps_alpha() {
    let bytes = raw_png(png::ColorType::GrayscaleAlpha, 1, 1, &[100, 200]);
    let image = decode_png(&bytes).expect("gray+alpha PNG must decode");
    assert_eq!(image.pixels, vec![100, 100, 100, 200]);
}

#[test]
fn encode_refuses_zero_sized_and_inconsistent_images() {
    let empty = RgbaImage {
        width: 0,
        height: 0,
        pixels: Vec::new(),
    };
    assert_eq!(encode_png(&empty), None);
    let inconsistent = RgbaImage {
        width: 2,
        height: 2,
        pixels: vec![0, 0, 0],
    };
    assert_eq!(encode_png(&inconsistent), None);
}

#[test]
fn encode_decode_round_trips_non_square_image() {
    let image = solid(1, 3, [200, 100, 50, 25]);
    let bytes = encode_png(&image).expect("1x3 image must encode");
    assert_eq!(decode_png(&bytes).expect("must decode"), image);
}

#[test]
fn diff_identical_images_reports_no_difference() {
    let a = solid(5, 4, [7, 7, 7, 255]);
    let b = solid(5, 4, [7, 7, 7, 255]);
    let d = diff_pictures(&a, &b);
    assert!(d.identical);
    assert_eq!(d.differing, 0);
    assert_eq!(d.total, 20);
    assert_eq!(d.bbox, None);
    assert_eq!((d.width_l, d.height_l), (5, 4));
    assert_eq!((d.width_r, d.height_r), (5, 4));
    assert_eq!((d.overlap_w, d.overlap_h), (5, 4));
}

#[test]
fn diff_single_changed_pixel_bounding_box_is_exact() {
    let mut b = solid(5, 4, [7, 7, 7, 255]);
    set(&mut b, 2, 3, [7, 7, 8, 255]);
    let a = solid(5, 4, [7, 7, 7, 255]);
    let d = diff_pictures(&a, &b);
    assert!(!d.identical);
    assert_eq!(d.differing, 1);
    assert_eq!(d.total, 20);
    assert_eq!(d.bbox, Some((2, 3, 2, 3)));
}

#[test]
fn diff_alpha_channel_counts_as_difference() {
    let mut b = solid(5, 4, [7, 7, 7, 255]);
    set(&mut b, 0, 0, [7, 7, 7, 254]);
    let a = solid(5, 4, [7, 7, 7, 255]);
    let d = diff_pictures(&a, &b);
    assert!(!d.identical);
    assert_eq!(d.differing, 1);
    assert_eq!(d.bbox, Some((0, 0, 0, 0)));
}

#[test]
fn diff_scattered_pixels_count_and_inclusive_bounds() {
    let a = solid(8, 6, [1, 1, 1, 1]);
    let mut b = solid(8, 6, [1, 1, 1, 1]);
    // Corners: top-left-ish, top-right, bottom-left-ish, and one in
    // the middle; the box must cover (0,1)..(7,5) inclusive.
    set(&mut b, 1, 1, [2, 1, 1, 1]);
    set(&mut b, 7, 1, [1, 2, 1, 1]);
    set(&mut b, 0, 3, [1, 1, 2, 1]);
    set(&mut b, 4, 5, [1, 1, 1, 2]);
    let d = diff_pictures(&a, &b);
    assert!(!d.identical);
    assert_eq!(d.differing, 4);
    assert_eq!(d.total, 48);
    assert_eq!(d.bbox, Some((0, 1, 7, 5)));
}

#[test]
fn diff_equal_overlap_but_different_sizes_is_not_identical() {
    let a = solid(4, 4, [7, 7, 7, 7]);
    let b = solid(8, 8, [7, 7, 7, 7]);
    let d = diff_pictures(&a, &b);
    assert!(!d.identical, "sizes differing is a difference");
    assert_eq!(d.differing, 0);
    assert_eq!(d.bbox, None);
    assert_eq!(d.overlap_w, 4);
    assert_eq!(d.overlap_h, 4);
    assert_eq!(d.total, 16);
    // Flipped argument order mirrors the sizes.
    let d = diff_pictures(&b, &a);
    assert!(!d.identical);
    assert_eq!((d.width_l, d.height_l), (8, 8));
    assert_eq!((d.width_r, d.height_r), (4, 4));
    assert_eq!(d.differing, 0);
}

#[test]
fn diff_one_by_one_against_sixty_four_square_does_not_panic() {
    let a = solid(1, 1, [9, 9, 9, 9]);
    let b = solid(64, 64, [5, 5, 5, 5]);
    let d = diff_pictures(&a, &b);
    assert!(!d.identical);
    assert_eq!(d.overlap_w, 1);
    assert_eq!(d.overlap_h, 1);
    assert_eq!(d.total, 1);
    assert_eq!(d.differing, 1);
    assert_eq!(d.bbox, Some((0, 0, 0, 0)));
    // And the mirror direction with equal overlap pixels.
    let c = solid(64, 64, [9, 9, 9, 9]);
    let d = diff_pictures(&a, &c);
    assert!(!d.identical);
    assert_eq!(d.differing, 0);
    assert_eq!(d.bbox, None);
}

#[test]
fn diff_zero_sized_images_have_empty_overlap() {
    let empty = RgbaImage {
        width: 0,
        height: 0,
        pixels: Vec::new(),
    };
    let d = diff_pictures(&empty, &empty);
    assert_eq!(d.total, 0);
    assert_eq!(d.differing, 0);
    assert_eq!(d.bbox, None);
    assert!(d.identical);
    // Against a real image the overlap is still empty, but the
    // differing sizes make the pair not identical.
    let b = solid(2, 2, [1, 1, 1, 1]);
    let d = diff_pictures(&empty, &b);
    assert_eq!(d.total, 0);
    assert!(!d.identical);
}

#[test]
fn heatmap_identical_region_is_dimmed_grayscale_of_left() {
    let mut a = solid(2, 1, [0, 0, 0, 255]);
    set(&mut a, 0, 0, [100, 150, 200, 255]);
    set(&mut a, 1, 0, [10, 20, 30, 255]);
    let b = a.clone();
    let heat = picture_heatmap(&a, &b);
    assert_eq!(heat.width, 2);
    assert_eq!(heat.height, 1);
    // Luma with the usual weights, dimmed to 30%:
    // 0.299*100 + 0.587*150 + 0.114*200 = 140.75; 140.75 * 30/100
    // rounds to 42. 0.299*10 + 0.587*20 + 0.114*30 = 18.15; *30/100
    // rounds to 5.
    let want_first = (0.299f64 * 100.0 + 0.587 * 150.0 + 0.114 * 200.0) * 30.0 / 100.0;
    assert_eq!(want_first.round(), 42.0);
    assert_eq!(get(&heat, 0, 0), [42, 42, 42, 255]);
    assert_eq!(get(&heat, 1, 0), [5, 5, 5, 255]);
}

#[test]
fn heatmap_differing_pixel_is_opaque_red() {
    let a = solid(2, 2, [1, 2, 3, 255]);
    let mut b = solid(2, 2, [1, 2, 3, 255]);
    set(&mut b, 1, 1, [9, 9, 9, 255]);
    let heat = picture_heatmap(&a, &b);
    // Luma of (1,2,3) = 1.815; *30/100 = 0.5445, rounds to 1.
    assert_eq!(get(&heat, 0, 0), [1, 1, 1, 255]);
    assert_eq!(get(&heat, 1, 1), [255, 0, 0, 255]);
}

#[test]
fn heatmap_covers_only_the_overlap() {
    let a = solid(3, 2, [4, 4, 4, 255]);
    let mut b = solid(8, 5, [4, 4, 4, 255]);
    set(&mut b, 2, 1, [8, 8, 8, 255]);
    let heat = picture_heatmap(&a, &b);
    assert_eq!(heat.width, 3);
    assert_eq!(heat.height, 2);
    assert_eq!(heat.pixels.len(), 3 * 2 * 4);
    assert_eq!(get(&heat, 2, 1), [255, 0, 0, 255]);
    assert_eq!(get(&heat, 0, 0), [1, 1, 1, 255]);
}

#[test]
fn heatmap_of_zero_sized_images_is_empty_not_panicking() {
    let empty = RgbaImage {
        width: 0,
        height: 0,
        pixels: Vec::new(),
    };
    let heat = picture_heatmap(&empty, &empty);
    assert_eq!(heat.width, 0);
    assert_eq!(heat.height, 0);
    assert!(heat.pixels.is_empty());
}
