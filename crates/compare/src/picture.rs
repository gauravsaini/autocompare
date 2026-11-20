//! PNG picture comparison over the filesystem.
//!
//! Both files are read whole, decoded by the pure layer and compared
//! pixel by pixel over their overlap. A file that does not decode as
//! PNG is [`Error::Unsupported`], not a crash. When a heatmap path is
//! requested, the heatmap PNG is written (a declared file write) before
//! the comparison result returns.

use std::path::Path;

use autocompare_diff::{RgbaImage, decode_image, diff_pictures, encode_png, picture_heatmap};

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

/// Decodes one side's image bytes (PNG, BMP or P6 PPM via format sniffing);
/// undecodable content is [`Error::Unsupported`].
fn decode_side(path: &Path, bytes: &[u8]) -> Result<RgbaImage> {
    decode_image(bytes).ok_or(Error::Unsupported {
        path: path.display().to_string(),
        what: "not a decodable PNG, BMP or PPM",
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_side_sniffs_png_bmp_ppm() {
        let png_img = RgbaImage {
            width: 1,
            height: 1,
            pixels: vec![7, 7, 7, 255],
        };
        let png_bytes = encode_png(&png_img);
        assert!(png_bytes.is_some());
        if let Some(bytes) = png_bytes {
            let via_png = decode_side(Path::new("left.png"), &bytes);
            assert!(via_png.is_ok());
            if let Ok(img) = via_png {
                assert_eq!(img.pixels, png_img.pixels);
            }
        }

        // Minimal 1x1 24-bit bottom-up BMP: 54-byte header + 4-byte padded row.
        let mut bmp = vec![0u8; 58];
        if let Some(tag) = bmp.get_mut(0..2) {
            tag.copy_from_slice(b"BM");
        }
        if let Some(size) = bmp.get_mut(2..6) {
            size.copy_from_slice(&58u32.to_le_bytes());
        }
        if let Some(off) = bmp.get_mut(10..14) {
            off.copy_from_slice(&54u32.to_le_bytes());
        }
        if let Some(head) = bmp.get_mut(14..18) {
            head.copy_from_slice(&40u32.to_le_bytes());
        }
        if let Some(w) = bmp.get_mut(18..22) {
            w.copy_from_slice(&1i32.to_le_bytes());
        }
        if let Some(h) = bmp.get_mut(22..26) {
            h.copy_from_slice(&1i32.to_le_bytes());
        }
        if let Some(planes) = bmp.get_mut(26..28) {
            planes.copy_from_slice(&1u16.to_le_bytes());
        }
        if let Some(bpp) = bmp.get_mut(28..30) {
            bpp.copy_from_slice(&24u16.to_le_bytes());
        }
        if let Some(px) = bmp.get_mut(54..57) {
            px.copy_from_slice(&[30, 20, 10]);
        }
        let via_bmp = decode_side(Path::new("left.bmp"), &bmp);
        assert!(via_bmp.is_ok());
        if let Ok(img) = via_bmp {
            assert_eq!(img.pixels, vec![10, 20, 30, 255]);
        }

        let ppm = b"P6\n1 1\n255\n\x00\x80\xff";
        let via_ppm = decode_side(Path::new("left.ppm"), ppm);
        assert!(via_ppm.is_ok());
        if let Ok(img) = via_ppm {
            assert_eq!(img.pixels, vec![0, 128, 255, 255]);
        }

        let err = decode_side(Path::new("left.bin"), b"not an image");
        assert!(err.is_err());
        if let Err(failed) = err {
            assert!(matches!(failed, Error::Unsupported { .. }));
        }
    }
}
