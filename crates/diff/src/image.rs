//! PNG decoding, pixel comparison and heatmap rendering.
//!
//! Pure: bytes in, pixels out. No I/O, no panics; undecodable input is
//! `None`, never an error. All pixel math uses checked indexing
//! helpers, and image sizes are bounded: the decoder's own limits cap
//! its internal buffers, and [`MAX_PIXEL_BYTES`] caps the caller-side
//! frame and pixel buffers so hostile header dimensions become `None`
//! instead of an out-of-memory abort.

use std::io::Cursor;

/// Upper bound on decoded pixel bytes (256 MiB). A PNG whose decoded
/// frame would exceed this is treated as undecodable. Roomy enough
/// for a 64-megapixel RGBA image, small enough that hostile headers
/// cannot exhaust memory.
const MAX_PIXEL_BYTES: usize = 256 * 1024 * 1024;

/// A decoded RGBA image: 4 bytes per pixel, row-major, top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8 pixel bytes; length is `width * height * 4`.
    pub pixels: Vec<u8>,
}

/// Decodes PNG bytes into an RGBA image. `None` when the bytes are not
/// a decodable PNG; any color type is accepted and normalised to RGBA
/// (missing alpha becomes fully opaque).
pub fn decode_png(bytes: &[u8]) -> Option<RgbaImage> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    // Expand palettes and low-bit grayscale, strip 16-bit samples:
    // every input becomes 8-bit grayscale/RGB, with an alpha channel
    // when the source carries one.
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let frame_len = reader.output_buffer_size()?;
    if frame_len > MAX_PIXEL_BYTES {
        return None;
    }
    let mut frame: Vec<u8> = Vec::new();
    frame.try_reserve_exact(frame_len).ok()?;
    frame.resize(frame_len, 0);
    let output = reader.next_frame(&mut frame).ok()?;
    if frame.len() < output.buffer_size() {
        return None;
    }
    let packed = frame.get(..output.buffer_size())?;
    rgba_from_packed(
        packed,
        output.color_type,
        output.bit_depth,
        output.width,
        output.height,
    )
}

/// Converts one packed, normalised frame into RGBA pixel bytes.
///
/// The normalised decoder output is always 8-bit, in one of four
/// layouts (1, 2, 3 or 4 bytes per pixel); anything else — including
/// an output this code was not built for — is `None`, never a guess.
fn rgba_from_packed(
    packed: &[u8],
    color: png::ColorType,
    depth: png::BitDepth,
    width: u32,
    height: u32,
) -> Option<RgbaImage> {
    if depth != png::BitDepth::Eight {
        return None;
    }
    let samples = color.samples();
    let pixel_count = u64::from(width).saturating_mul(u64::from(height));
    let rgba_len = pixel_count.saturating_mul(4);
    if rgba_len > MAX_PIXEL_BYTES as u64 {
        return None;
    }
    let rgba_len = usize::try_from(rgba_len).ok()?;
    let mut pixels: Vec<u8> = Vec::new();
    pixels.try_reserve_exact(rgba_len).ok()?;
    for py in 0..height {
        // Row offset in bytes; per-pixel offsets stay inside the row.
        let row_start = usize::try_from(py)
            .ok()?
            .checked_mul(usize::try_from(width).ok()?)?
            .checked_mul(samples)?;
        for px in 0..width {
            let start = row_start.checked_add(usize::try_from(px).ok()?.checked_mul(samples)?)?;
            let end = start.checked_add(samples)?;
            let sample = packed.get(start..end)?;
            let [r, g, b, a] = rgba_sample(sample, color)?;
            pixels.extend_from_slice(&[r, g, b, a]);
        }
    }
    Some(RgbaImage {
        width,
        height,
        pixels,
    })
}

/// Expands one packed 8-bit sample to RGBA. Indexed never appears
/// after normalisation, so it is rejected like any unknown layout.
fn rgba_sample(sample: &[u8], color: png::ColorType) -> Option<[u8; 4]> {
    match color {
        png::ColorType::Grayscale => {
            let g = sample.first().copied()?;
            Some([g, g, g, 255])
        }
        png::ColorType::GrayscaleAlpha => {
            let g = sample.first().copied()?;
            let a = sample.get(1).copied()?;
            Some([g, g, g, a])
        }
        png::ColorType::Rgb => {
            let r = sample.first().copied()?;
            let g = sample.get(1).copied()?;
            let b = sample.get(2).copied()?;
            Some([r, g, b, 255])
        }
        png::ColorType::Rgba => {
            let r = sample.first().copied()?;
            let g = sample.get(1).copied()?;
            let b = sample.get(2).copied()?;
            let a = sample.get(3).copied()?;
            Some([r, g, b, a])
        }
        png::ColorType::Indexed => None,
    }
}

/// Encodes an RGBA image as PNG bytes. `None` on encoder failure, for
/// zero-sized images (the PNG format requires dimensions of at least
/// one pixel) or when the pixel buffer disagrees with the declared
/// dimensions.
pub fn encode_png(image: &RgbaImage) -> Option<Vec<u8>> {
    if image.width == 0 || image.height == 0 {
        return None;
    }
    let mut out: Vec<u8> = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&image.pixels).ok()?;
        writer.finish().ok()?;
    }
    Some(out)
}

/// Blend or composition mode for image comparison and rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum PictureBlendMode {
    /// Exact raw RGBA channel comparison.
    #[default]
    Exact,
    /// Alpha-composites pixels over a solid RGB backdrop before comparing.
    CompositeBackdrop([u8; 3]),
    /// Absolute difference blend evaluation.
    Difference,
}

/// Options controlling picture comparison sensitivity, tolerance, range, and blending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureOptions {
    /// Maximum difference allowed on any individual channel (0..=255) for two pixels to match.
    pub tolerance: u8,
    /// Maximum total channel delta (sum of absolute differences across all channels, 0..=1020).
    /// If 0, only `tolerance` is checked.
    pub channel_range: u16,
    /// Blending / composition strategy.
    pub blend_mode: PictureBlendMode,
}

impl Default for PictureOptions {
    fn default() -> Self {
        Self {
            tolerance: 0,
            channel_range: 0,
            blend_mode: PictureBlendMode::Exact,
        }
    }
}

/// Statistics of a pixel-wise comparison over the two images'
/// overlapping region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureDiff {
    /// Left image width.
    pub width_l: u32,
    /// Left image height.
    pub height_l: u32,
    /// Right image width.
    pub width_r: u32,
    /// Right image height.
    pub height_r: u32,
    /// The smaller of the two widths.
    pub overlap_w: u32,
    /// The smaller of the two heights.
    pub overlap_h: u32,
    /// Pixels differing in any channel within the overlap.
    pub differing: u64,
    /// Total pixels in the overlap.
    pub total: u64,
    /// Inclusive bounding box `(min_x, min_y, max_x, max_y)` of
    /// differing pixels, or `None` when none differ.
    pub bbox: Option<(u32, u32, u32, u32)>,
    /// Same dimensions and no differing pixel anywhere.
    pub identical: bool,
}

/// Compares two images pixel by pixel over their overlap with default options (exact RGBA match).
pub fn diff_pictures(a: &RgbaImage, b: &RgbaImage) -> PictureDiff {
    diff_pictures_with_options(a, b, &PictureOptions::default())
}

/// Compares two images pixel by pixel over their overlap with custom tolerance, range, and blend options.
pub fn diff_pictures_with_options(
    a: &RgbaImage,
    b: &RgbaImage,
    opts: &PictureOptions,
) -> PictureDiff {
    let overlap_w = a.width.min(b.width);
    let overlap_h = a.height.min(b.height);
    let total = u64::from(overlap_w).saturating_mul(u64::from(overlap_h));
    let mut differing: u64 = 0;
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    for y in 0..overlap_h {
        for x in 0..overlap_w {
            let same = match (pixel(a, x, y), pixel(b, x, y)) {
                (Some(pa), Some(pb)) => pixel_matches(pa, pb, opts),
                _ => false,
            };
            if !same {
                differing = differing.saturating_add(1);
                bbox = Some(match bbox {
                    None => (x, y, x, y),
                    Some((min_x, min_y, max_x, max_y)) => {
                        (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
                    }
                });
            }
        }
    }
    let identical = a.width == b.width && a.height == b.height && differing == 0;
    PictureDiff {
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

/// Decides whether two RGBA pixels match given tolerance, range, and blend settings.
fn pixel_matches(pa: [u8; 4], pb: [u8; 4], opts: &PictureOptions) -> bool {
    let (ca, cb) = match opts.blend_mode {
        PictureBlendMode::Exact => (pa, pb),
        PictureBlendMode::CompositeBackdrop(bg) => (composite_over(pa, bg), composite_over(pb, bg)),
        PictureBlendMode::Difference => (pa, pb),
    };

    let dr = ca
        .first()
        .copied()
        .unwrap_or(0)
        .abs_diff(cb.first().copied().unwrap_or(0));
    let dg = ca
        .get(1)
        .copied()
        .unwrap_or(0)
        .abs_diff(cb.get(1).copied().unwrap_or(0));
    let db = ca
        .get(2)
        .copied()
        .unwrap_or(0)
        .abs_diff(cb.get(2).copied().unwrap_or(0));
    let da = ca
        .get(3)
        .copied()
        .unwrap_or(0)
        .abs_diff(cb.get(3).copied().unwrap_or(0));

    if dr > opts.tolerance || dg > opts.tolerance || db > opts.tolerance || da > opts.tolerance {
        return false;
    }

    if opts.channel_range > 0 {
        let total_diff = u16::from(dr)
            .saturating_add(u16::from(dg))
            .saturating_add(u16::from(db))
            .saturating_add(u16::from(da));
        if total_diff > opts.channel_range {
            return false;
        }
    }

    true
}

/// Alpha composites an RGBA sample over a solid RGB background color.
fn composite_over(sample: [u8; 4], bg: [u8; 3]) -> [u8; 4] {
    let [r, g, b, a] = sample;
    let alpha = f64::from(a) / 255.0;
    let bg_r = f64::from(bg.first().copied().unwrap_or(0));
    let bg_g = f64::from(bg.get(1).copied().unwrap_or(0));
    let bg_b = f64::from(bg.get(2).copied().unwrap_or(0));

    let out_r = ((f64::from(r) * alpha) + (bg_r * (1.0 - alpha)))
        .round()
        .clamp(0.0, 255.0) as u8;
    let out_g = ((f64::from(g) * alpha) + (bg_g * (1.0 - alpha)))
        .round()
        .clamp(0.0, 255.0) as u8;
    let out_b = ((f64::from(b) * alpha) + (bg_b * (1.0 - alpha)))
        .round()
        .clamp(0.0, 255.0) as u8;

    [out_r, out_g, out_b, 255]
}

/// A heatmap of differences over the overlap with default exact comparison.
pub fn picture_heatmap(a: &RgbaImage, b: &RgbaImage) -> RgbaImage {
    picture_heatmap_with_options(a, b, &PictureOptions::default())
}

/// A heatmap of differences over the overlap using custom tolerance, range, and blend settings.
pub fn picture_heatmap_with_options(
    a: &RgbaImage,
    b: &RgbaImage,
    opts: &PictureOptions,
) -> RgbaImage {
    let overlap_w = a.width.min(b.width);
    let overlap_h = a.height.min(b.height);
    let byte_len = u64::from(overlap_w)
        .saturating_mul(u64::from(overlap_h))
        .saturating_mul(4);
    let byte_len = usize::try_from(byte_len).unwrap_or(usize::MAX);
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
            let out = match (pixel(a, x, y), pixel(b, x, y)) {
                (Some(pa), Some(pb)) if pixel_matches(pa, pb, opts) => {
                    let [r, g, b, _alpha] = pa;
                    let luma = 0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b);
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

/// Blends two images pixel-by-pixel with `alpha_ratio` (`0.0` = only `a`, `1.0` = only `b`).
#[allow(dead_code)]
pub fn picture_blend(a: &RgbaImage, b: &RgbaImage, alpha_ratio: f32) -> RgbaImage {
    let overlap_w = a.width.min(b.width);
    let overlap_h = a.height.min(b.height);
    let clamped_ratio = f64::from(alpha_ratio).clamp(0.0, 1.0);
    let byte_len = u64::from(overlap_w)
        .saturating_mul(u64::from(overlap_h))
        .saturating_mul(4);
    let byte_len = usize::try_from(byte_len).unwrap_or(usize::MAX);
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
            let pa = pixel(a, x, y).unwrap_or([0, 0, 0, 255]);
            let pb = pixel(b, x, y).unwrap_or([0, 0, 0, 255]);
            let r = (f64::from(pa[0]) * (1.0 - clamped_ratio) + f64::from(pb[0]) * clamped_ratio)
                .round() as u8;
            let g = (f64::from(pa[1]) * (1.0 - clamped_ratio) + f64::from(pb[1]) * clamped_ratio)
                .round() as u8;
            let b_ch = (f64::from(pa[2]) * (1.0 - clamped_ratio) + f64::from(pb[2]) * clamped_ratio)
                .round() as u8;
            let a_ch = (f64::from(pa[3]) * (1.0 - clamped_ratio) + f64::from(pb[3]) * clamped_ratio)
                .round() as u8;
            pixels.extend_from_slice(&[r, g, b_ch, a_ch]);
        }
    }
    RgbaImage {
        width: overlap_w,
        height: overlap_h,
        pixels,
    }
}

/// Computes an absolute difference blend image `|a - b|` over the overlap.
#[allow(dead_code)]
pub fn picture_difference_blend(a: &RgbaImage, b: &RgbaImage) -> RgbaImage {
    let overlap_w = a.width.min(b.width);
    let overlap_h = a.height.min(b.height);
    let byte_len = u64::from(overlap_w)
        .saturating_mul(u64::from(overlap_h))
        .saturating_mul(4);
    let byte_len = usize::try_from(byte_len).unwrap_or(usize::MAX);
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
            let pa = pixel(a, x, y).unwrap_or([0, 0, 0, 255]);
            let pb = pixel(b, x, y).unwrap_or([0, 0, 0, 255]);
            let r = pa[0].abs_diff(pb[0]);
            let g = pa[1].abs_diff(pb[1]);
            let b_ch = pa[2].abs_diff(pb[2]);
            pixels.extend_from_slice(&[r, g, b_ch, 255]);
        }
    }
    RgbaImage {
        width: overlap_w,
        height: overlap_h,
        pixels,
    }
}

/// The RGBA bytes at `(x, y)`, or `None` when the pixel buffer is
/// shorter than the declared dimensions imply.
fn pixel(img: &RgbaImage, x: u32, y: u32) -> Option<[u8; 4]> {
    let index = usize::try_from(y)
        .ok()?
        .checked_mul(usize::try_from(img.width).ok()?)?
        .checked_add(usize::try_from(x).ok()?)?
        .checked_mul(4)?;
    let bytes = img.pixels.get(index..index.checked_add(4)?)?;
    bytes.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tolerance_matching() {
        let a = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![100, 100, 100, 255],
        };
        let b = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![105, 103, 98, 255],
        };
        // Strict diff should report different
        let strict = diff_pictures(&a, &b);
        assert_eq!(strict.differing, 1);
        assert!(!strict.identical);

        // With tolerance = 5, channels differ by at most 5, so it should match
        let opts = PictureOptions {
            tolerance: 5,
            channel_range: 0,
            blend_mode: PictureBlendMode::Exact,
        };
        let relaxed = diff_pictures_with_options(&a, &b, &opts);
        assert_eq!(relaxed.differing, 0);
        assert!(relaxed.identical);
    }

    #[test]
    fn test_channel_range_matching() {
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
        // Tolerance = 10, range = 3: total diff is 4, which exceeds range 3!
        let opts_fail = PictureOptions {
            tolerance: 10,
            channel_range: 3,
            blend_mode: PictureBlendMode::Exact,
        };
        assert_eq!(diff_pictures_with_options(&a, &b, &opts_fail).differing, 1);

        let opts_pass = PictureOptions {
            tolerance: 10,
            channel_range: 5,
            blend_mode: PictureBlendMode::Exact,
        };
        assert_eq!(diff_pictures_with_options(&a, &b, &opts_pass).differing, 0);
    }

    #[test]
    fn test_blend_functions() {
        let a = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![0, 0, 0, 255],
        };
        let b = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![100, 100, 100, 255],
        };
        let blended = picture_blend(&a, &b, 0.5);
        assert_eq!(blended.pixels, vec![50, 50, 50, 255]);

        let diff_blend = picture_difference_blend(&a, &b);
        assert_eq!(diff_blend.pixels, vec![100, 100, 100, 255]);
    }
}
