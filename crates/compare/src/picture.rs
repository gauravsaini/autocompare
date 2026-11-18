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

/// Decodes one side's PNG bytes; undecodable content is
/// [`Error::Unsupported`].
fn decode_side(path: &Path, bytes: &[u8]) -> Result<RgbaImage> {
    decode_png(bytes).ok_or(Error::Unsupported {
        path: path.display().to_string(),
        what: "not a decodable PNG",
    })
}
