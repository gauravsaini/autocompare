//! PNG decoding, pixel comparison and heatmap rendering.
//!
//! Pure: bytes in, pixels out. No I/O, no panics; undecodable input is
//! `None`, never an error. All pixel math uses checked indexing
//! helpers, and image sizes are bounded by the decoder's own limits.

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
/// a decodable PNG; any color type is accepted and normalised to RGBA.
pub fn decode_png(bytes: &[u8]) -> Option<RgbaImage> {
    // STUB: replaced by the real implementation in crates/diff.
    let _ = bytes;
    None
}

/// Encodes an RGBA image as PNG bytes. `None` on encoder failure.
pub fn encode_png(image: &RgbaImage) -> Option<Vec<u8>> {
    // STUB: replaced by the real implementation in crates/diff.
    let _ = image;
    None
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
/// same dimensions and zero differing pixels.
pub fn diff_pictures(a: &RgbaImage, b: &RgbaImage) -> PictureDiff {
    // STUB: replaced by the real implementation in crates/diff.
    let _ = (a, b);
    PictureDiff {
        width_l: 0,
        height_l: 0,
        width_r: 0,
        height_r: 0,
        overlap_w: 0,
        overlap_h: 0,
        differing: 0,
        total: 0,
        bbox: None,
        identical: true,
    }
}

/// A heatmap of the differences over the overlap: identical pixels
/// dimmed to 30% grayscale of the left image, differing pixels opaque
/// red `(255, 0, 0, 255)`.
pub fn picture_heatmap(a: &RgbaImage, b: &RgbaImage) -> RgbaImage {
    // STUB: replaced by the real implementation in crates/diff.
    let _ = (a, b);
    RgbaImage {
        width: 0,
        height: 0,
        pixels: Vec::new(),
    }
}
