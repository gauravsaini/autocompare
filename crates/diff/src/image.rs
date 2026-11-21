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

/// Decodes a binary Netpbm PPM (P6 format) image into an [`RgbaImage`].
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

/// Options controlling picture comparison sensitivity, tolerance and range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PictureOptions {
    /// Maximum difference allowed on any individual channel (0..=255) for two pixels to match.
    pub tolerance: u8,
    /// Maximum total channel delta (sum of absolute differences across all channels, 0..=1020).
    /// If 0, only `tolerance` is checked.
    pub channel_range: u16,
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

/// Decides whether two RGBA pixels match given tolerance and range settings.
fn pixel_matches(pa: [u8; 4], pb: [u8; 4], opts: &PictureOptions) -> bool {
    let dr = pa
        .first()
        .copied()
        .unwrap_or(0)
        .abs_diff(pb.first().copied().unwrap_or(0));
    let dg = pa
        .get(1)
        .copied()
        .unwrap_or(0)
        .abs_diff(pb.get(1).copied().unwrap_or(0));
    let db = pa
        .get(2)
        .copied()
        .unwrap_or(0)
        .abs_diff(pb.get(2).copied().unwrap_or(0));
    let da = pa
        .get(3)
        .copied()
        .unwrap_or(0)
        .abs_diff(pb.get(3).copied().unwrap_or(0));

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

/// A heatmap of differences over the overlap with default exact comparison.
pub fn picture_heatmap(a: &RgbaImage, b: &RgbaImage) -> RgbaImage {
    picture_heatmap_with_options(a, b, &PictureOptions::default())
}

/// A heatmap of differences over the overlap using custom tolerance and range settings.
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

/// Viewport, integer zoom, offset and blend helpers on an already
/// decoded image. All total: short pixel buffers yield `None` (or a
/// differing count), never a panic; allocation failure degrades to an
/// empty image or `None`.
impl RgbaImage {
    /// Copy with every pixel alpha forced to 255 (opaque).
    ///
    /// Headless wgpu readback leaves alpha 0 on fully-covered windows;
    /// viewers tolerate it but an honest opaque screenshot stores 255.
    /// RGB bytes are untouched; short buffers pass through as-is.
    pub fn opaque(&self) -> RgbaImage {
        let mut pixels = self.pixels.clone();
        let mut i = 3usize;
        while let Some(a) = pixels.get_mut(i) {
            *a = 255;
            i = i.saturating_add(4);
        }
        RgbaImage {
            width: self.width,
            height: self.height,
            pixels,
        }
    }

    /// New image with content shifted by `(dx, dy)` (pan/offset-nudge).
    ///
    /// Same dimensions; pixels pushed outside are dropped and vacated
    /// slots become transparent black. A short pixel buffer reads as
    /// transparent wherever the bytes are missing.
    pub fn nudge(&self, dx: i32, dy: i32) -> RgbaImage {
        let byte_len = u64::from(self.width)
            .saturating_mul(u64::from(self.height))
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
        for y in 0..self.height {
            for x in 0..self.width {
                let sx = i64::from(x).saturating_sub(i64::from(dx));
                let sy = i64::from(y).saturating_sub(i64::from(dy));
                let px =
                    if sx >= 0 && sy >= 0 && sx <= i64::from(u32::MAX) && sy <= i64::from(u32::MAX)
                    {
                        let (ux, uy) = (sx as u32, sy as u32);
                        pixel(self, ux, uy).unwrap_or([0, 0, 0, 0])
                    } else {
                        [0, 0, 0, 0]
                    };
                pixels.extend_from_slice(&px);
            }
        }
        RgbaImage {
            width: self.width,
            height: self.height,
            pixels,
        }
    }

    /// Cropped view of the rectangle at `(x, y)` with size `w` by `h`
    /// (pan window). `None` when the rectangle leaves the image, has
    /// zero area, or the pixel buffer is short.
    pub fn viewport(&self, x: u32, y: u32, w: u32, h: u32) -> Option<RgbaImage> {
        if w == 0 || h == 0 {
            return None;
        }
        if x.checked_add(w).is_none_or(|end| end > self.width) {
            return None;
        }
        if y.checked_add(h).is_none_or(|end| end > self.height) {
            return None;
        }
        let byte_len = u64::from(w).saturating_mul(u64::from(h)).saturating_mul(4);
        let byte_len = usize::try_from(byte_len).ok()?;
        let mut pixels: Vec<u8> = Vec::new();
        pixels.try_reserve_exact(byte_len).ok()?;
        for row in 0..h {
            for col in 0..w {
                let px = pixel(self, x.saturating_add(col), y.saturating_add(row))?;
                pixels.extend_from_slice(&px);
            }
        }
        Some(RgbaImage {
            width: w,
            height: h,
            pixels,
        })
    }

    /// Integer nearest-neighbour zoom by `factor` (each source pixel
    /// becomes a `factor` by `factor` block). `None` for factor 0,
    /// overflowing dimensions, hostile sizes, or a short buffer.
    pub fn scaled(&self, factor: u32) -> Option<RgbaImage> {
        if factor == 0 {
            return None;
        }
        let width = self.width.checked_mul(factor)?;
        let height = self.height.checked_mul(factor)?;
        let byte_len = u64::from(width)
            .checked_mul(u64::from(height))?
            .checked_mul(4)?;
        if byte_len > MAX_PIXEL_BYTES as u64 {
            return None;
        }
        let byte_len = usize::try_from(byte_len).ok()?;
        let mut pixels: Vec<u8> = Vec::new();
        pixels.try_reserve_exact(byte_len).ok()?;
        for y in 0..height {
            for x in 0..width {
                let px = pixel(self, x.checked_div(factor)?, y.checked_div(factor)?)?;
                pixels.extend_from_slice(&px);
            }
        }
        Some(RgbaImage {
            width,
            height,
            pixels,
        })
    }

    /// Per-channel absolute difference over the overlap (difference
    /// blend): each output pixel is `|a - b|` per channel, opaque.
    /// `None` when the overlap is empty or a buffer is short.
    pub fn difference_blend(&self, other: &RgbaImage) -> Option<RgbaImage> {
        let width = self.width.min(other.width);
        let height = self.height.min(other.height);
        if width == 0 || height == 0 {
            return None;
        }
        let byte_len = u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(4);
        let byte_len = usize::try_from(byte_len).ok()?;
        let mut pixels: Vec<u8> = Vec::new();
        pixels.try_reserve_exact(byte_len).ok()?;
        for y in 0..height {
            for x in 0..width {
                let pa = pixel(self, x, y)?;
                let pb = pixel(other, x, y)?;
                let out = [
                    pa.first()
                        .copied()
                        .unwrap_or(0)
                        .abs_diff(pb.first().copied().unwrap_or(0)),
                    pa.get(1)
                        .copied()
                        .unwrap_or(0)
                        .abs_diff(pb.get(1).copied().unwrap_or(0)),
                    pa.get(2)
                        .copied()
                        .unwrap_or(0)
                        .abs_diff(pb.get(2).copied().unwrap_or(0)),
                    255,
                ];
                pixels.extend_from_slice(&out);
            }
        }
        Some(RgbaImage {
            width,
            height,
            pixels,
        })
    }

    /// Compares `other`'s frame against this image shifted by
    /// `(dx, dy)` with per-channel `tolerance` (range 0).
    ///
    /// Only in-frame pairs count: `total` is the number of compared
    /// pixels, so a nudge that lines the pictures up reports 0
    /// differing where the plain overlap diff reports many.
    /// `identical` additionally requires zero offset and equal
    /// dimensions.
    pub fn diff_with_offset(
        &self,
        other: &RgbaImage,
        dx: i32,
        dy: i32,
        tolerance: u8,
    ) -> PictureDiff {
        let opts = PictureOptions {
            tolerance,
            channel_range: 0,
        };
        let mut total: u64 = 0;
        let mut differing: u64 = 0;
        let mut bbox: Option<(u32, u32, u32, u32)> = None;
        for y in 0..other.height {
            for x in 0..other.width {
                let sx = i64::from(x).saturating_sub(i64::from(dx));
                let sy = i64::from(y).saturating_sub(i64::from(dy));
                let in_frame =
                    sx >= 0 && sy >= 0 && sx <= i64::from(u32::MAX) && sy <= i64::from(u32::MAX);
                if !in_frame {
                    continue;
                }
                total = total.saturating_add(1);
                let same = match (pixel(self, sx as u32, sy as u32), pixel(other, x, y)) {
                    (Some(pa), Some(pb)) => pixel_matches(pa, pb, &opts),
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
        let identical = dx == 0
            && dy == 0
            && self.width == other.width
            && self.height == other.height
            && differing == 0;
        PictureDiff {
            width_l: self.width,
            height_l: self.height,
            width_r: other.width,
            height_r: other.height,
            overlap_w: self.width.min(other.width),
            overlap_h: self.height.min(other.height),
            differing,
            total,
            bbox,
            identical,
        }
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
        };
        assert_eq!(diff_pictures_with_options(&a, &b, &opts_fail).differing, 1);

        let opts_pass = PictureOptions {
            tolerance: 10,
            channel_range: 5,
        };
        assert_eq!(diff_pictures_with_options(&a, &b, &opts_pass).differing, 0);
    }

    #[test]
    fn test_bmp_decode_bottom_up() {
        // Hand-built 2x1 24-bit BMP (bottom-up, row stride padded to 4).
        let mut bmp = Vec::new();
        bmp.extend_from_slice(b"BM");
        bmp.extend_from_slice(&62u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&54u32.to_le_bytes());
        bmp.extend_from_slice(&40u32.to_le_bytes());
        bmp.extend_from_slice(&2i32.to_le_bytes());
        bmp.extend_from_slice(&1i32.to_le_bytes());
        bmp.extend_from_slice(&1u16.to_le_bytes());
        bmp.extend_from_slice(&24u16.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&8u32.to_le_bytes());
        bmp.extend_from_slice(&2835u32.to_le_bytes());
        bmp.extend_from_slice(&2835u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        // BGR pixels: red, green, then 2 pad bytes.
        bmp.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        let decoded = decode_bmp(&bmp);
        assert!(decoded.is_some());
        if let Some(dec) = decoded {
            assert_eq!(dec.width, 2);
            assert_eq!(dec.height, 1);
            assert_eq!(dec.pixels, vec![255, 0, 0, 255, 0, 255, 0, 255]);
        }
    }

    #[test]
    fn test_decode_image_sniffs_format() {
        let png_img = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![9, 9, 9, 255],
        };
        let png_bytes = encode_png(&png_img);
        assert!(png_bytes.is_some());
        if let Some(bytes) = png_bytes {
            let decoded = decode_image(&bytes);
            assert!(decoded.is_some());
            if let Some(dec) = decoded {
                assert_eq!(dec.pixels, png_img.pixels);
            }
        }
        assert!(decode_image(b"definitely not an image").is_none());
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
    fn test_decode_image_rejects_hostile_input() {
        assert!(decode_image(b"").is_none());
        assert!(decode_image(b"BM\x00\x01").is_none());
        assert!(decode_image(b"P6\n1 1\n255\n\xff").is_none());
    }

    fn rgb_image(pixels: &[[u8; 4]]) -> RgbaImage {
        let mut flat = Vec::new();
        for px in pixels {
            flat.extend_from_slice(px);
        }
        RgbaImage {
            width: 2,
            height: 1,
            pixels: flat,
        }
    }

    #[test]
    fn test_nudge_shifts_and_vacates() {
        let a = rgb_image(&[[10, 0, 0, 255], [20, 0, 0, 255]]);
        let right = a.nudge(1, 0);
        assert_eq!(
            right.pixels,
            vec![0, 0, 0, 0, 10, 0, 0, 255],
            "shift right drops G, vacates with transparent"
        );
        let left = a.nudge(-1, 0);
        assert_eq!(left.pixels, vec![20, 0, 0, 255, 0, 0, 0, 0]);
        let same = a.nudge(0, 0);
        assert_eq!(same.pixels, a.pixels);
    }

    #[test]
    fn test_viewport_crop_and_reject() {
        let img = RgbaImage {
            width: 2,
            height: 2,
            pixels: vec![
                1, 0, 0, 255, 2, 0, 0, 255, //
                3, 0, 0, 255, 4, 0, 0, 255,
            ],
        };
        if let Some(crop) = img.viewport(1, 0, 1, 2) {
            assert_eq!(crop.width, 1);
            assert_eq!(crop.height, 2);
            assert_eq!(crop.pixels, vec![2, 0, 0, 255, 4, 0, 0, 255]);
        } else {
            assert_eq!(img.width, 0, "in-bounds crop must succeed");
        }
        assert!(
            img.viewport(1, 0, 2, 2).is_none(),
            "leaving the frame is None"
        );
        assert!(img.viewport(0, 0, 0, 1).is_none(), "zero area is None");
    }

    #[test]
    fn test_scaled_integer_zoom() {
        let one = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![9, 8, 7, 255],
        };
        if let Some(big) = one.scaled(2) {
            assert_eq!((big.width, big.height), (2, 2));
            assert_eq!(big.pixels.len(), 16);
            assert!(
                big.pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|px| *px == [9, 8, 7, 255])
            );
        } else {
            assert_eq!(one.width, 0, "x2 zoom of 1x1 must succeed");
        }
        assert!(one.scaled(0).is_none(), "factor 0 is None");
    }

    #[test]
    fn test_difference_blend_overlap() {
        let a = rgb_image(&[[10, 0, 0, 255], [20, 0, 0, 255]]);
        let same = rgb_image(&[[10, 0, 0, 255], [20, 0, 0, 255]]);
        if let Some(zero) = a.difference_blend(&same) {
            assert_eq!(zero.pixels, vec![0, 0, 0, 255, 0, 0, 0, 255]);
        } else {
            assert_eq!(a.width, 0, "identical blend must succeed");
        }
        let one = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![0, 0, 0, 255],
        };
        // Overlap is the 1x1 minimum; surplus pixels are ignored.
        if let Some(min) = a.difference_blend(&one) {
            assert_eq!((min.width, min.height), (1, 1));
            assert_eq!(min.pixels, vec![10, 0, 0, 255]);
        } else {
            assert_eq!(a.width, 0, "overlap blend must succeed");
        }
    }

    #[test]
    fn test_opaque_forces_alpha_leaves_rgb() {
        let img = RgbaImage {
            width: 2,
            height: 1,
            pixels: vec![10, 20, 30, 0, 40, 50, 60, 0],
        };
        let op = img.opaque();
        assert_eq!(op.width, 2);
        assert_eq!(op.height, 1);
        assert_eq!(op.pixels, vec![10, 20, 30, 255, 40, 50, 60, 255]);
        // Honest opaque bytes survive the PNG round-trip.
        if let Some(bytes) = encode_png(&op) {
            if let Some(back) = decode_png(&bytes) {
                assert_eq!(back.pixels, op.pixels);
            } else {
                assert_eq!(op.width, 0, "opaque PNG must decode");
            }
        } else {
            assert_eq!(op.width, 0, "opaque image must encode");
        }
    }

    #[test]
    fn test_diff_with_offset_aligns_nudge() {
        let a = rgb_image(&[[10, 0, 0, 255], [20, 0, 0, 255]]);
        let b = a.nudge(1, 0);
        assert_eq!(diff_pictures(&a, &b).differing, 2);
        let aligned = a.diff_with_offset(&b, 1, 0, 0);
        assert_eq!(aligned.differing, 0);
        assert_eq!(aligned.total, 1, "only the in-frame pair counts");
        assert!(!aligned.identical, "offset compare is never identical");
        let exact = a.diff_with_offset(&a, 0, 0, 0);
        assert_eq!(exact.differing, 0);
        assert_eq!(exact.total, 2);
        assert!(exact.identical);
        // Tolerance still applies under offset.
        let c = rgb_image(&[[12, 0, 0, 255], [20, 0, 0, 255]]);
        assert_eq!(a.diff_with_offset(&c, 0, 0, 0).differing, 1);
        assert_eq!(a.diff_with_offset(&c, 0, 0, 5).differing, 0);
    }
}
