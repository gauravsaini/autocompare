//! Audio file comparison over the filesystem.
//!
//! Both files are read whole, decoded by the pure layer and compared
//! sample by sample over the shorter stream's duration. A file that
//! does not decode as WAV or MP3 is [`Error::Unsupported`], not a
//! crash; a sample-rate or channel mismatch is reported through the
//! diff (not comparable, honestly), not as an error.

use std::path::Path;

use autocompare_diff::{PcmAudio, decode_audio, diff_audio};

use crate::{AudioComparison, Error, Result};

/// Compares two audio files. See the crate documentation.
pub(crate) fn compare_audio_files(left: &Path, right: &Path) -> Result<AudioComparison> {
    let left_bytes = read_bytes(left)?;
    let right_bytes = read_bytes(right)?;
    let left_audio = decode_side(left, &left_bytes)?;
    let right_audio = decode_side(right, &right_bytes)?;
    let diff = diff_audio(&left_audio, &right_audio);
    Ok(AudioComparison { diff })
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

/// Decodes one side's audio bytes; undecodable content is
/// [`Error::Unsupported`].
fn decode_side(path: &Path, bytes: &[u8]) -> Result<PcmAudio> {
    decode_audio(bytes).ok_or(Error::Unsupported {
        path: path.display().to_string(),
        what: "not a decodable WAV or MP3",
    })
}
