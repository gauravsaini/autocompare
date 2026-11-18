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

/// Compares two images pixel by pixel over their overlap.
///
/// Pixels are compared on all four RGBA channels. Regions beyond the
/// smaller image's extent are not compared; `identical` requires the
/// same dimensions and zero differing pixels. A pixel whose data is
/// missing on either side (only possible in a hand-built
/// [`RgbaImage`] with inconsistent fields) counts as differing.
pub fn diff_pictures(a: &RgbaImage, b: &RgbaImage) -> PictureDiff {
    let overlap_w = a.width.min(b.width);
    let overlap_h = a.height.min(b.height);
    let total = u64::from(overlap_w).saturating_mul(u64::from(overlap_h));
    let mut differing: u64 = 0;
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    for y in 0..overlap_h {
        for x in 0..overlap_w {
            let same = matches!(
                (pixel(a, x, y), pixel(b, x, y)),
                (Some(pa), Some(pb)) if pa == pb
            );
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

/// A heatmap of the differences over the overlap: identical pixels
/// dimmed to 30% grayscale of the left image, differing pixels opaque
/// red `(255, 0, 0, 255)`.
///
/// The heatmap covers only the overlap of the two images. If the
/// overlap is too large for the machine's address space (only
/// possible with hand-built images), an empty image is returned
/// instead of panicking.
pub fn picture_heatmap(a: &RgbaImage, b: &RgbaImage) -> RgbaImage {
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
                (Some(pa), Some(pb)) if pa == pb => {
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
