//! Audio file comparison over the filesystem.
//!
//! Both files are read whole, decoded by the pure layer and compared
//! sample by sample over the shorter stream's duration. A file that
//! does not decode as WAV or MP3 is [`Error::Unsupported`], not a
//! crash; a sample-rate or channel mismatch is reported through the
//! diff (not comparable, honestly), not as an error.

use std::path::Path;

use autocompare_diff::media_tags::{MediaTags, MediaTagsDiff, diff_media_tags, read_media_tags};
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

/// Loads media metadata tags (FLAC, MP3, MP4, or AAC) from a file.
#[allow(dead_code)]
pub fn load_media_tags(path: &Path) -> Result<MediaTags> {
    let bytes = read_bytes(path)?;
    read_media_tags(&bytes).ok_or_else(|| Error::Unsupported {
        path: path.display().to_string(),
        what: "no decodable media tags (FLAC/MP3/MP4/AAC)",
    })
}

/// The result of comparing media metadata tags across two audio files.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct MediaTagsComparison {
    /// Left audio file tags.
    pub left_tags: MediaTags,
    /// Right audio file tags.
    pub right_tags: MediaTags,
    /// Detailed tag diff.
    pub diff: MediaTagsDiff,
}

/// Compares metadata tags (FLAC, MP3, MP4/AAC) between two media files.
#[allow(dead_code)]
pub fn compare_media_tags_files(left: &Path, right: &Path) -> Result<MediaTagsComparison> {
    let left_tags = load_media_tags(left)?;
    let right_tags = load_media_tags(right)?;
    let diff = diff_media_tags(&left_tags, &right_tags);
    Ok(MediaTagsComparison {
        left_tags,
        right_tags,
        diff,
    })
}

/// Combined comparison of media tags and PCM audio between two files.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct FullMediaComparison {
    /// Tag difference if tags are present on either side.
    pub tags: Option<MediaTagsComparison>,
    /// PCM audio sample diff if audio is decodable on both sides.
    pub audio_diff: Option<autocompare_diff::AudioDiff>,
}

/// Compares both metadata tags and PCM audio streams between two media files.
#[allow(dead_code)]
pub fn compare_media_files_full(left: &Path, right: &Path) -> Result<FullMediaComparison> {
    let tags_result = compare_media_tags_files(left, right).ok();
    let audio_result = compare_audio_files(left, right).ok().map(|c| c.diff);

    if tags_result.is_none() && audio_result.is_none() {
        return Err(Error::Unsupported {
            path: format!("{} / {}", left.display(), right.display()),
            what: "neither media tags nor PCM audio could be decoded",
        });
    }

    Ok(FullMediaComparison {
        tags: tags_result,
        audio_diff: audio_result,
    })
}
