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

/// Decodes uncompressed 24-bit or 32-bit BMP bytes into an [`RgbaImage`].
/// Supports both bottom-up (standard positive height) and top-down (negative height) layouts.
#[allow(dead_code)]
pub fn decode_bmp(bytes: &[u8]) -> Option<RgbaImage> {
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

    let planes = u16::from_le_bytes(bytes.get(26..28)?.try_into().ok()?);
    if planes != 1 {
        return None;
    }
    let bpp = u16::from_le_bytes(bytes.get(28..30)?.try_into().ok()?);
    if bpp != 24 && bpp != 32 {
        return None;
    }
    let compression = u32::from_le_bytes(bytes.get(30..34)?.try_into().ok()?);
    if compression != 0 && compression != 3 {
        return None;
    }

    let pixel_count = u64::from(width).checked_mul(u64::from(height))?;
    let rgba_len = pixel_count.checked_mul(4)?;
    if rgba_len > MAX_PIXEL_BYTES as u64 {
        return None;
    }
    let rgba_len = usize::try_from(rgba_len).ok()?;

    let bytes_per_pixel = usize::from(bpp / 8);
    let unpadded_row = usize::try_from(width).ok()?.checked_mul(bytes_per_pixel)?;
    let row_stride = unpadded_row.checked_add(3)? / 4 * 4;

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

/// Encodes an [`RgbaImage`] as a 24-bit uncompressed Windows BMP file.
#[allow(dead_code)]
pub fn encode_bmp(image: &RgbaImage) -> Option<Vec<u8>> {
    if image.width == 0 || image.height == 0 {
        return None;
    }
    let w = usize::try_from(image.width).ok()?;
    let h = usize::try_from(image.height).ok()?;
    let unpadded_row = w.checked_mul(3)?;
    let row_stride = unpadded_row.checked_add(3)? / 4 * 4;
    let image_data_size = row_stride.checked_mul(h)?;
    let file_size = 54usize.checked_add(image_data_size)?;

    let mut out = Vec::new();
    out.try_reserve_exact(file_size).ok()?;

    // BITMAPFILEHEADER (14 bytes)
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(file_size as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // reserved
    out.extend_from_slice(&54u32.to_le_bytes()); // pixel data offset

    // BITMAPINFOHEADER (40 bytes)
    out.extend_from_slice(&40u32.to_le_bytes()); // header size
    out.extend_from_slice(&(image.width as i32).to_le_bytes());
    out.extend_from_slice(&(image.height as i32).to_le_bytes()); // positive = bottom-up
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&24u16.to_le_bytes()); // bpp
    out.extend_from_slice(&0u32.to_le_bytes()); // compression (BI_RGB)
    out.extend_from_slice(&(image_data_size as u32).to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes()); // 72 DPI
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // colors used
    out.extend_from_slice(&0u32.to_le_bytes()); // important colors

    // Pixel data (bottom-up, BGR)
    let pad_len = row_stride.saturating_sub(unpadded_row);
    for y in (0..h).rev() {
        let src_row = y.checked_mul(w)?.checked_mul(4)?;
        for x in 0..w {
            let px_idx = src_row.checked_add(x.checked_mul(4)?)?;
            let r = image.pixels.get(px_idx).copied()?;
            let g = image.pixels.get(px_idx.checked_add(1)?).copied()?;
            let b = image.pixels.get(px_idx.checked_add(2)?).copied()?;
            out.push(b);
            out.push(g);
            out.push(r);
        }
        out.extend(std::iter::repeat_n(0, pad_len));
    }

    Some(out)
}

/// Decodes a binary Netpbm PPM (P6 format) image into an [`RgbaImage`].
#[allow(dead_code)]
pub fn decode_ppm(bytes: &[u8]) -> Option<RgbaImage> {
    if bytes.len() < 10 || bytes.get(0..2) != Some(b"P6") {
        return None;
    }
    let mut pos = 2usize;
    let mut tokens = Vec::new();

    while tokens.len() < 3 && pos < bytes.len() {
        while let Some(&b) = bytes.get(pos) {
            if b == b'#' {
                while let Some(&c) = bytes.get(pos) {
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
        while let Some(&b) = bytes.get(pos) {
            if !b.is_ascii_whitespace() && b != b'#' {
                pos = pos.saturating_add(1);
            } else {
                break;
            }
        }
        let tok_bytes = bytes.get(start..pos)?;
        let tok_str = std::str::from_utf8(tok_bytes).ok()?;
        tokens.push(tok_str.parse::<u32>().ok()?);
    }

    if tokens.len() < 3 {
        return None;
    }
    let width = tokens.first().copied()?;
    let height = tokens.get(1).copied()?;
    let max_val = tokens.get(2).copied()?;
    if width == 0 || height == 0 || max_val == 0 || max_val > 255 {
        return None;
    }

    if bytes.get(pos).is_some_and(|b| b.is_ascii_whitespace()) {
        pos = pos.saturating_add(1);
    }

    let pixel_count = u64::from(width).checked_mul(u64::from(height))?;
    let expected_rgb_len = usize::try_from(pixel_count.checked_mul(3)?).ok()?;
    let rgb_data = bytes.get(pos..pos.checked_add(expected_rgb_len)?)?;

    let rgba_len = usize::try_from(pixel_count.checked_mul(4)?).ok()?;
    if rgba_len > MAX_PIXEL_BYTES {
        return None;
    }
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(rgba_len).ok()?;

    for &[r, g, b] in rgb_data.as_chunks::<3>().0 {
        pixels.extend_from_slice(&[r, g, b, 255]);
    }

    Some(RgbaImage {
        width,
        height,
        pixels,
    })
}

/// Decodes image bytes into an [`RgbaImage`], sniffing format (PNG, BMP, PPM).
#[allow(dead_code)]
pub fn decode_image(bytes: &[u8]) -> Option<RgbaImage> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return decode_png(bytes);
    }
    if bytes.starts_with(b"BM") {
        return decode_bmp(bytes);
    }
    if bytes.starts_with(b"P6") {
        return decode_ppm(bytes);
    }
    decode_png(bytes)
        .or_else(|| decode_bmp(bytes))
        .or_else(|| decode_ppm(bytes))
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

impl PictureDiff {
    /// Fraction of overlapping pixels that differ, in `[0.0, 1.0]`.
    /// Empty overlap reports `0.0` (nothing to differ on).
    pub fn differ_ratio(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.differing as f64) / (self.total as f64)
        }
    }
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

/// Compares two images pixel by pixel over their overlap with an (offset_x, offset_y)
/// nudge applied to the right image.
#[allow(dead_code)]
pub fn diff_pictures_with_offset(
    a: &RgbaImage,
    b: &RgbaImage,
    offset_x: i32,
    offset_y: i32,
    opts: &PictureOptions,
) -> PictureDiff {
    let w_a = a.width as i32;
    let h_a = a.height as i32;
    let w_b = b.width as i32;
    let h_b = b.height as i32;

    let min_x = 0.max(offset_x);
    let max_x = w_a.min(offset_x.saturating_add(w_b));
    let min_y = 0.max(offset_y);
    let max_y = h_a.min(offset_y.saturating_add(h_b));

    if min_x >= max_x || min_y >= max_y {
        return PictureDiff {
            width_l: a.width,
            height_l: a.height,
            width_r: b.width,
            height_r: b.height,
            overlap_w: 0,
            overlap_h: 0,
            differing: 0,
            total: 0,
            bbox: None,
            identical: false,
        };
    }

    let overlap_w = (max_x - min_x) as u32;
    let overlap_h = (max_y - min_y) as u32;
    let total = u64::from(overlap_w).saturating_mul(u64::from(overlap_h));
    let mut differing: u64 = 0;
    let mut bbox: Option<(u32, u32, u32, u32)> = None;

    for y in min_y..max_y {
        for x in min_x..max_x {
            let bx = x.saturating_sub(offset_x);
            let by = y.saturating_sub(offset_y);
            let same = match (pixel(a, x as u32, y as u32), pixel(b, bx as u32, by as u32)) {
                (Some(pa), Some(pb)) => pixel_matches(pa, pb, opts),
                _ => false,
            };
            if !same {
                differing = differing.saturating_add(1);
                let ux = x as u32;
                let uy = y as u32;
                bbox = Some(match bbox {
                    None => (ux, uy, ux, uy),
                    Some((min_bx, min_by, max_bx, max_by)) => (
                        min_bx.min(ux),
                        min_by.min(uy),
                        max_bx.max(ux),
                        max_by.max(uy),
                    ),
                });
            }
        }
    }

    let identical = offset_x == 0
        && offset_y == 0
        && a.width == b.width
        && a.height == b.height
        && differing == 0;

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

/// Generates a difference heatmap over the overlapping region with (offset_x, offset_y) nudge.
#[allow(dead_code)]
pub fn picture_heatmap_with_offset(
    a: &RgbaImage,
    b: &RgbaImage,
    offset_x: i32,
    offset_y: i32,
    opts: &PictureOptions,
) -> RgbaImage {
    let w_a = a.width as i32;
    let h_a = a.height as i32;
    let w_b = b.width as i32;
    let h_b = b.height as i32;

    let min_x = 0.max(offset_x);
    let max_x = w_a.min(offset_x.saturating_add(w_b));
    let min_y = 0.max(offset_y);
    let max_y = h_a.min(offset_y.saturating_add(h_b));

    if min_x >= max_x || min_y >= max_y {
        return RgbaImage {
            width: 0,
            height: 0,
            pixels: Vec::new(),
        };
    }

    let overlap_w = (max_x - min_x) as u32;
    let overlap_h = (max_y - min_y) as u32;
    let byte_len = match usize::try_from(
        u64::from(overlap_w)
            .saturating_mul(u64::from(overlap_h))
            .saturating_mul(4),
    ) {
        Ok(l) => l,
        Err(_) => {
            return RgbaImage {
                width: 0,
                height: 0,
                pixels: Vec::new(),
            };
        }
    };

    let mut pixels = Vec::new();
    if pixels.try_reserve_exact(byte_len).is_err() {
        return RgbaImage {
            width: 0,
            height: 0,
            pixels: Vec::new(),
        };
    }

    for y in min_y..max_y {
        for x in min_x..max_x {
            let bx = x.saturating_sub(offset_x);
            let by = y.saturating_sub(offset_y);
            let out = match (pixel(a, x as u32, y as u32), pixel(b, bx as u32, by as u32)) {
                (Some(pa), Some(pb)) if pixel_matches(pa, pb, opts) => {
                    let [r, g, b_ch, _] = pa;
                    let luma =
                        0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b_ch);
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

/// Nudges an image by `(offset_x, offset_y)`, returning a shifted image of identical size
/// with newly exposed areas filled with `fill`.
#[allow(dead_code)]
pub fn nudge_image(img: &RgbaImage, offset_x: i32, offset_y: i32, fill: [u8; 4]) -> RgbaImage {
    let w = img.width as i32;
    let h = img.height as i32;
    let byte_len = match usize::try_from(
        u64::from(img.width)
            .saturating_mul(u64::from(img.height))
            .saturating_mul(4),
    ) {
        Ok(l) => l,
        Err(_) => return img.clone(),
    };
    let mut pixels = Vec::new();
    if pixels.try_reserve_exact(byte_len).is_err() {
        return img.clone();
    }

    for y in 0..h {
        for x in 0..w {
            let src_x = x.saturating_sub(offset_x);
            let src_y = y.saturating_sub(offset_y);
            let px = if src_x >= 0 && src_x < w && src_y >= 0 && src_y < h {
                pixel(img, src_x as u32, src_y as u32).unwrap_or(fill)
            } else {
                fill
            };
            pixels.extend_from_slice(&px);
        }
    }

    RgbaImage {
        width: img.width,
        height: img.height,
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

    #[test]
    fn test_bmp_encode_decode_roundtrip() {
        let img = RgbaImage {
            width: 2,
            height: 2,
            pixels: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        };
        let bmp_bytes = encode_bmp(&img);
        assert!(bmp_bytes.is_some());
        if let Some(bytes) = bmp_bytes {
            let decoded = decode_bmp(&bytes);
            assert!(decoded.is_some());
            if let Some(dec) = decoded {
                assert_eq!(dec.width, 2);
                assert_eq!(dec.height, 2);
                assert_eq!(dec.pixels, img.pixels);
            }
        }
    }

    #[test]
    fn test_ppm_decode() {
        let ppm_data = b"P6\n2 2\n255\n\xFF\x00\x00\x00\xFF\x00\x00\x00\xFF\xFF\xFF\xFF";
        let decoded = decode_ppm(ppm_data);
        assert!(decoded.is_some());
        if let Some(dec) = decoded {
            assert_eq!(dec.width, 2);
            assert_eq!(dec.height, 2);
            assert_eq!(
                dec.pixels,
                vec![
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
                ]
            );
        }
    }

    #[test]
    fn test_diff_pictures_with_offset_nudge() {
        let a = RgbaImage {
            width: 3,
            height: 3,
            pixels: vec![
                10, 10, 10, 255, 20, 20, 20, 255, 30, 30, 30, 255, 40, 40, 40, 255, 50, 50, 50,
                255, 60, 60, 60, 255, 70, 70, 70, 255, 80, 80, 80, 255, 90, 90, 90, 255,
            ],
        };
        // Nudge b by (1, 0)
        let b = RgbaImage {
            width: 2,
            height: 3,
            pixels: vec![
                20, 20, 20, 255, 30, 30, 30, 255, 50, 50, 50, 255, 60, 60, 60, 255, 80, 80, 80,
                255, 90, 90, 90, 255,
            ],
        };
        let opts = PictureOptions::default();
        let diff = diff_pictures_with_offset(&a, &b, 1, 0, &opts);
        assert_eq!(diff.overlap_w, 2);
        assert_eq!(diff.overlap_h, 3);
        assert_eq!(diff.differing, 0);

        let nudged = nudge_image(&a, 1, 0, [0, 0, 0, 255]);
        assert_eq!(nudged.width, 3);
        assert_eq!(nudged.height, 3);
    }
}
