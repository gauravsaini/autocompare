//! PNG picture comparison over the filesystem.
//!
//! Both files are read whole, decoded by the pure layer and compared
//! pixel by pixel over their overlap. A file that does not decode as
//! PNG is [`Error::Unsupported`], not a crash. When a heatmap path is
//! requested, the heatmap PNG is written (a declared file write) before
//! the comparison result returns.

use std::path::Path;

use autocompare_diff::{RgbaImage, decode_png, diff_pictures, encode_png, picture_heatmap};

use crate::{Error, PictureComparison, PictureOptions, Result};

/// Compares two PNG files. See the crate documentation.
pub(crate) fn compare_picture_files(
    left: &Path,
    right: &Path,
    opts: &PictureOptions,
) -> Result<PictureComparison> {
    let left_bytes = read_bytes(left)?;
    let right_bytes = read_bytes(right)?;
    let left_image = decode_side(left, &left_bytes)?;
    let right_image = decode_side(right, &right_bytes)?;
    let diff = diff_pictures(&left_image, &right_image);
    if let Some(path) = &opts.heatmap {
        let heatmap = picture_heatmap(&left_image, &right_image);
        let bytes = encode_png(&heatmap).ok_or(Error::Unsupported {
            path: path.display().to_string(),
            what: "heatmap encode failed",
        })?;
        std::fs::write(path, bytes).map_err(|source| Error::Io {
            path: path.display().to_string(),
            source,
        })?;
    }
    Ok(PictureComparison { diff })
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

/// Decodes one side's image bytes (PNG or uncompressed BMP); undecodable content is
/// [`Error::Unsupported`].
fn decode_side(path: &Path, bytes: &[u8]) -> Result<RgbaImage> {
    if let Some(img) = decode_png(bytes) {
        return Ok(img);
    }
    if bytes.starts_with(b"BM") {
        return decode_bmp_fallback(bytes).ok_or_else(|| Error::Unsupported {
            path: path.display().to_string(),
            what: "not a decodable PNG or BMP",
        });
    }
    Err(Error::Unsupported {
        path: path.display().to_string(),
        what: "not a decodable PNG or BMP",
    })
}

/// Fallback uncompressed BMP decoder for filesystem compare layer.
fn decode_bmp_fallback(bytes: &[u8]) -> Option<RgbaImage> {
    if bytes.len() < 54 || bytes.get(0..2) != Some(b"BM") {
        return None;
    }
    let data_offset = u32::from_le_bytes(bytes.get(10..14)?.try_into().ok()?) as usize;
    let dib_header_size = u32::from_le_bytes(bytes.get(14..18)?.try_into().ok()?);
    if dib_header_size < 40 {
        return None;
    }
    let raw_w = i32::from_le_bytes(bytes.get(18..22)?.try_into().ok()?);
    let raw_h = i32::from_le_bytes(bytes.get(22..26)?.try_into().ok()?);
    if raw_w <= 0 || raw_h == 0 {
        return None;
    }
    let width = raw_w as u32;
    let height = raw_h.unsigned_abs();
    let top_down = raw_h < 0;

    let bpp = u16::from_le_bytes(bytes.get(28..30)?.try_into().ok()?);
    if bpp != 24 && bpp != 32 {
        return None;
    }
    let bytes_per_pixel = usize::from(bpp / 8);
    let unpadded_row = usize::try_from(width).ok()?.checked_mul(bytes_per_pixel)?;
    let row_stride = unpadded_row.checked_add(3)? / 4 * 4;

    let rgba_len = usize::try_from(
        u64::from(width)
            .checked_mul(u64::from(height))?
            .checked_mul(4)?,
    )
    .ok()?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(rgba_len).ok()?;
    pixels.resize(rgba_len, 0);

    for y in 0..height {
        let bmp_row = if top_down {
            y as usize
        } else {
            usize::try_from(height.saturating_sub(1).saturating_sub(y)).ok()?
        };
        let row_start = data_offset.checked_add(bmp_row.checked_mul(row_stride)?)?;
        let row_end = row_start.checked_add(unpadded_row)?;
        if row_end > bytes.len() {
            return None;
        }
        let row_bytes = bytes.get(row_start..row_end)?;
        let dest_row_start = (y as usize)
            .checked_mul(usize::try_from(width).ok()?)?
            .checked_mul(4)?;

        for x in 0..width {
            let px_idx = (x as usize).checked_mul(bytes_per_pixel)?;
            let b = row_bytes.get(px_idx).copied()?;
            let g = row_bytes.get(px_idx.checked_add(1)?).copied()?;
            let r = row_bytes.get(px_idx.checked_add(2)?).copied()?;
            let a = if bytes_per_pixel == 4 {
                let raw_a = row_bytes.get(px_idx.checked_add(3)?).copied()?;
                if raw_a == 0 { 255 } else { raw_a }
            } else {
                255
            };
            let dest_px = dest_row_start.checked_add((x as usize).checked_mul(4)?)?;
            if let Some(slot) = pixels.get_mut(dest_px..dest_px.checked_add(4)?) {
                if let Some(s0) = slot.first_mut() {
                    *s0 = r;
                }
                if let Some(s1) = slot.get_mut(1) {
                    *s1 = g;
                }
                if let Some(s2) = slot.get_mut(2) {
                    *s2 = b;
                }
                if let Some(s3) = slot.get_mut(3) {
                    *s3 = a;
                }
            }
        }
    }

    Some(RgbaImage {
        width,
        height,
        pixels,
    })
}

/// Per-channel tolerance for picture comparison (Beyond Compare parity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub struct PictureTolerance {
    /// Max allowed absolute delta per channel (0 = exact).
    pub tolerance: u8,
    /// Max allowed sum of absolute channel deltas (0 = ignore).
    pub channel_range: u16,
    /// When true, compare on alpha-composited RGB over white.
    pub composite_on_white: bool,
}

/// Compares two picture files with per-channel tolerance, channel range
/// and optional white-backdrop compositing (Beyond Compare parity).
///
/// Decoding accepts PNG, uncompressed BMP and binary Netpbm P6 PPM;
/// undecodable sides are [`Error::Unsupported`]. The heatmap (when
/// requested via `opts`) honours the same tolerance.
#[allow(dead_code)]
pub fn compare_picture_files_with_tolerance(
    left: &Path,
    right: &Path,
    opts: &PictureOptions,
    tol: &PictureTolerance,
) -> Result<PictureComparison> {
    let left_bytes = read_bytes(left)?;
    let right_bytes = read_bytes(right)?;
    let left_image = decode_side_any(left, &left_bytes)?;
    let right_image = decode_side_any(right, &right_bytes)?;
    let diff = diff_with_tolerance(&left_image, &right_image, tol);
    if let Some(path) = &opts.heatmap {
        let heatmap = heatmap_with_tolerance(&left_image, &right_image, tol);
        let bytes = encode_png(&heatmap).ok_or(Error::Unsupported {
            path: path.display().to_string(),
            what: "heatmap encode failed",
        })?;
        std::fs::write(path, bytes).map_err(|source| Error::Io {
            path: path.display().to_string(),
            source,
        })?;
    }
    Ok(PictureComparison { diff })
}

/// Pixel match under tolerance/range/composite settings.
#[allow(dead_code)]
fn pixels_match_tol(a: [u8; 4], b: [u8; 4], tol: &PictureTolerance) -> bool {
    let (ca, cb) = if tol.composite_on_white {
        (composite_white(a), composite_white(b))
    } else {
        (a, b)
    };
    let deltas = [
        ca[0].abs_diff(cb[0]),
        ca[1].abs_diff(cb[1]),
        ca[2].abs_diff(cb[2]),
        ca[3].abs_diff(cb[3]),
    ];
    for d in deltas {
        if d > tol.tolerance {
            return false;
        }
    }
    if tol.channel_range > 0 {
        let total: u16 = deltas.iter().map(|d| u16::from(*d)).sum();
        if total > tol.channel_range {
            return false;
        }
    }
    true
}

#[allow(dead_code)]
fn composite_white(px: [u8; 4]) -> [u8; 4] {
    let alpha = f64::from(px[3]) / 255.0;
    let blend = |fg: u8| ((f64::from(fg) * alpha) + (255.0 * (1.0 - alpha))).round() as u8;
    [blend(px[0]), blend(px[1]), blend(px[2]), 255]
}

#[allow(dead_code)]
fn pixel_at(img: &RgbaImage, x: u32, y: u32) -> Option<[u8; 4]> {
    let idx = usize::try_from(y)
        .ok()?
        .checked_mul(usize::try_from(img.width).ok()?)?
        .checked_add(usize::try_from(x).ok()?)?
        .checked_mul(4)?;
    img.pixels.get(idx..idx.checked_add(4)?)?.try_into().ok()
}

#[allow(dead_code)]
fn diff_with_tolerance(
    a: &RgbaImage,
    b: &RgbaImage,
    tol: &PictureTolerance,
) -> autocompare_diff::PictureDiff {
    let overlap_w = a.width.min(b.width);
    let overlap_h = a.height.min(b.height);
    let total = u64::from(overlap_w).saturating_mul(u64::from(overlap_h));
    let mut differing: u64 = 0;
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    for y in 0..overlap_h {
        for x in 0..overlap_w {
            let same = match (pixel_at(a, x, y), pixel_at(b, x, y)) {
                (Some(pa), Some(pb)) => pixels_match_tol(pa, pb, tol),
                _ => false,
            };
            if !same {
                differing = differing.saturating_add(1);
                bbox = Some(match bbox {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
            }
        }
    }
    let identical = a.width == b.width && a.height == b.height && differing == 0;
    autocompare_diff::PictureDiff {
        width_l: a.width,
        height_l: a.height,
        width_r: b.width,
        height_r: b.height,
        overlap_w,
        overlap_h,
        differing,
        total,
        bbox,
        identical,
    }
}

#[allow(dead_code)]
fn heatmap_with_tolerance(a: &RgbaImage, b: &RgbaImage, tol: &PictureTolerance) -> RgbaImage {
    let overlap_w = a.width.min(b.width);
    let overlap_h = a.height.min(b.height);
    let byte_len = usize::try_from(
        u64::from(overlap_w)
            .saturating_mul(u64::from(overlap_h))
            .saturating_mul(4),
    )
    .unwrap_or(0);
    let mut pixels: Vec<u8> = Vec::new();
    if pixels.try_reserve_exact(byte_len).is_err() {
        return RgbaImage {
            width: 0,
            height: 0,
            pixels,
        };
    }
    for y in 0..overlap_h {
        for x in 0..overlap_w {
            let out = match (pixel_at(a, x, y), pixel_at(b, x, y)) {
                (Some(pa), Some(pb)) if pixels_match_tol(pa, pb, tol) => {
                    let luma = 0.299 * f64::from(pa[0])
                        + 0.587 * f64::from(pa[1])
                        + 0.114 * f64::from(pa[2]);
                    let gray = (luma * 30.0 / 100.0).round().clamp(0.0, 255.0) as u8;
                    [gray, gray, gray, 255]
                }
                _ => [255, 0, 0, 255],
            };
            pixels.extend_from_slice(&out);
        }
    }
    RgbaImage {
        width: overlap_w,
        height: overlap_h,
        pixels,
    }
}

/// Decodes PNG, BMP or P6 PPM; `None` when undecodable.
#[allow(dead_code)]
fn decode_side_any(path: &Path, bytes: &[u8]) -> Result<RgbaImage> {
    if let Some(img) = decode_png(bytes) {
        return Ok(img);
    }
    if bytes.starts_with(b"BM")
        && let Some(img) = decode_bmp_fallback(bytes)
    {
        return Ok(img);
    }
    if bytes.starts_with(b"P6")
        && let Some(img) = decode_ppm_fallback(bytes)
    {
        return Ok(img);
    }
    Err(Error::Unsupported {
        path: path.display().to_string(),
        what: "not a decodable PNG, BMP or PPM",
    })
}

/// Minimal total P6 PPM decoder (max-val 1..=255, single whitespace
/// separator, comments allowed). Returns `None` on hostile input.
#[allow(dead_code)]
fn decode_ppm_fallback(bytes: &[u8]) -> Option<RgbaImage> {
    if bytes.len() < 10 || bytes.get(0..2) != Some(b"P6") {
        return None;
    }
    let mut pos = 2usize;
    let mut tokens: Vec<u32> = Vec::new();
    while tokens.len() < 3 && pos < bytes.len() {
        while let Some(b) = bytes.get(pos).copied() {
            if b == b'#' {
                while let Some(c) = bytes.get(pos).copied() {
                    pos = pos.saturating_add(1);
                    if c == b'\n' {
                        break;
                    }
                }
            } else if b.is_ascii_whitespace() {
                pos = pos.saturating_add(1);
            } else {
                break;
            }
        }
        if pos >= bytes.len() {
            break;
        }
        let start = pos;
        while let Some(b) = bytes.get(pos).copied() {
            if !b.is_ascii_whitespace() && b != b'#' {
                pos = pos.saturating_add(1);
            } else {
                break;
            }
        }
        let tok = bytes.get(start..pos)?;
        let s = std::str::from_utf8(tok).ok()?;
        tokens.push(s.parse::<u32>().ok()?);
    }
    if tokens.len() < 3 {
        return None;
    }
    let width = *tokens.first()?;
    let height = *tokens.get(1)?;
    let max_val = *tokens.get(2)?;
    if width == 0 || height == 0 || max_val == 0 || max_val > 255 {
        return None;
    }
    if bytes.get(pos).is_some_and(|b| b.is_ascii_whitespace()) {
        pos = pos.saturating_add(1);
    }
    let pixel_count = u64::from(width).checked_mul(u64::from(height))?;
    let expected = usize::try_from(pixel_count.checked_mul(3)?).ok()?;
    let rgb = bytes.get(pos..pos.checked_add(expected)?)?;
    let rgba_len = usize::try_from(pixel_count.checked_mul(4)?).ok()?;
    if rgba_len > 256 * 1024 * 1024 {
        return None;
    }
    let mut pixels: Vec<u8> = Vec::new();
    if pixels.try_reserve_exact(rgba_len).is_err() {
        return None;
    }
    for chunk in rgb.as_chunks::<3>().0 {
        let r = chunk.first().copied().unwrap_or(0);
        let g = chunk.get(1).copied().unwrap_or(0);
        let b = chunk.get(2).copied().unwrap_or(0);
        pixels.extend_from_slice(&[r, g, b, 255]);
    }
    Some(RgbaImage {
        width,
        height,
        pixels,
    })
}

/// Compares two picture files with an (offset_x, offset_y) nudge applied to the right image.
#[allow(dead_code)]
pub fn compare_picture_files_with_offset(
    left: &Path,
    right: &Path,
    offset_x: i32,
    offset_y: i32,
    opts: &PictureOptions,
) -> Result<PictureComparison> {
    let left_bytes = read_bytes(left)?;
    let right_bytes = read_bytes(right)?;
    let left_image = decode_side(left, &left_bytes)?;
    let right_image = decode_side(right, &right_bytes)?;

    let w_a = left_image.width as i32;
    let h_a = left_image.height as i32;
    let w_b = right_image.width as i32;
    let h_b = right_image.height as i32;

    let min_x = 0.max(offset_x);
    let max_x = w_a.min(offset_x.saturating_add(w_b));
    let min_y = 0.max(offset_y);
    let max_y = h_a.min(offset_y.saturating_add(h_b));

    if min_x >= max_x || min_y >= max_y {
        let diff = autocompare_diff::PictureDiff {
            width_l: left_image.width,
            height_l: left_image.height,
            width_r: right_image.width,
            height_r: right_image.height,
            overlap_w: 0,
            overlap_h: 0,
            differing: 0,
            total: 0,
            bbox: None,
            identical: false,
        };
        return Ok(PictureComparison { diff });
    }

    let overlap_w = (max_x - min_x) as u32;
    let overlap_h = (max_y - min_y) as u32;

    let crop = |img: &RgbaImage, start_x: u32, start_y: u32, w: u32, h: u32| -> Option<RgbaImage> {
        let byte_len =
            usize::try_from(u64::from(w).checked_mul(u64::from(h))?.checked_mul(4)?).ok()?;
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(byte_len).ok()?;
        for y in 0..h {
            let sy = start_y.checked_add(y)?;
            let row_start = (sy as usize)
                .checked_mul(usize::try_from(img.width).ok()?)?
                .checked_mul(4)?;
            for x in 0..w {
                let sx = start_x.checked_add(x)?;
                let px_idx = row_start.checked_add((sx as usize).checked_mul(4)?)?;
                pixels.extend_from_slice(img.pixels.get(px_idx..px_idx.checked_add(4)?)?);
            }
        }
        Some(RgbaImage {
            width: w,
            height: h,
            pixels,
        })
    };

    let bx_start = (min_x.saturating_sub(offset_x)) as u32;
    let by_start = (min_y.saturating_sub(offset_y)) as u32;

    let sub_l = crop(
        &left_image,
        min_x as u32,
        min_y as u32,
        overlap_w,
        overlap_h,
    )
    .ok_or_else(|| Error::Unsupported {
        path: left.display().to_string(),
        what: "failed to crop left overlap",
    })?;
    let sub_r = crop(&right_image, bx_start, by_start, overlap_w, overlap_h).ok_or_else(|| {
        Error::Unsupported {
            path: right.display().to_string(),
            what: "failed to crop right overlap",
        }
    })?;

    let mut diff = diff_pictures(&sub_l, &sub_r);
    diff.width_l = left_image.width;
    diff.height_l = left_image.height;
    diff.width_r = right_image.width;
    diff.height_r = right_image.height;
    diff.identical = offset_x == 0 && offset_y == 0 && diff.identical;

    if let Some((min_bx, min_by, max_bx, max_by)) = diff.bbox {
        diff.bbox = Some((
            min_bx.saturating_add(min_x as u32),
            min_by.saturating_add(min_y as u32),
            max_bx.saturating_add(min_x as u32),
            max_by.saturating_add(min_y as u32),
        ));
    }

    if let Some(path) = &opts.heatmap {
        let heatmap = picture_heatmap(&sub_l, &sub_r);
        let bytes = encode_png(&heatmap).ok_or(Error::Unsupported {
            path: path.display().to_string(),
            what: "heatmap encode failed",
        })?;
        std::fs::write(path, bytes).map_err(|source| Error::Io {
            path: path.display().to_string(),
            source,
        })?;
    }

    Ok(PictureComparison { diff })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_tolerance_absorbs_small_delta() {
        let a = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![100, 100, 100, 255],
        };
        let b = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![103, 102, 101, 255],
        };
        let strict = PictureTolerance::default();
        assert_eq!(diff_with_tolerance(&a, &b, &strict).differing, 1);
        let relaxed = PictureTolerance {
            tolerance: 5,
            channel_range: 0,
            composite_on_white: false,
        };
        let d = diff_with_tolerance(&a, &b, &relaxed);
        assert_eq!(d.differing, 0);
        assert!(d.identical);
    }

    #[test]
    fn test_channel_range_rejects_spread_delta() {
        let a = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![10, 10, 10, 255],
        };
        let b = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![14, 10, 10, 255],
        };
        let tight = PictureTolerance {
            tolerance: 10,
            channel_range: 3,
            composite_on_white: false,
        };
        assert_eq!(diff_with_tolerance(&a, &b, &tight).differing, 1);
        let loose = PictureTolerance {
            tolerance: 10,
            channel_range: 5,
            composite_on_white: false,
        };
        assert_eq!(diff_with_tolerance(&a, &b, &loose).differing, 0);
    }

    #[test]
    fn test_hostile_ppm_returns_none() {
        assert!(decode_ppm_fallback(b"").is_none());
        assert!(decode_ppm_fallback(b"P6\n1 1\n255\n\xff").is_none());
        let good = b"P6\n1 1\n255\n\x00\x80\xff";
        let img = decode_ppm_fallback(good);
        assert!(img.is_some());
    }
}
