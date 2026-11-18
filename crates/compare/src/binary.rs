//! Byte-level file comparison, streamed in 64 KiB chunks.
//!
//! Hunk semantics: on the first differing byte at offset `o` a hunk
//! collects up to 64 bytes from each side starting at `o`; the scan
//! then resumes at `o + 64`, so hunks never overlap and a small
//! bounded re-read (with seek) is used to fetch hunk bytes across
//! chunk boundaries.
//!
//! Truncation semantics: a "difference event" is either a differing
//! byte or one file ending while the other still has bytes (the
//! prefix case). `first_diff` records the first event of either
//! kind. Differing bytes consume a hunk slot; the prefix event never
//! does (a pure prefix reports no hunks). `truncated` is true when
//! an event is found but no hunk slot is left for it — in particular
//! `max_hunks == 0` marks any difference as truncated.

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use crate::{BinaryComparison, BinaryHunk, Error, Result};

/// Streaming chunk size for the byte scan.
const CHUNK: usize = 64 * 1024;
/// Maximum bytes shown per hunk.
const HUNK: usize = 64;

/// Compares two files byte by byte. See the module documentation.
pub(crate) fn compare_binary_files(
    left: &Path,
    right: &Path,
    max_hunks: usize,
) -> Result<BinaryComparison> {
    let left_size = file_size(left)?;
    let right_size = file_size(right)?;
    let mut lreader = open_buffered(left)?;
    let mut rreader = open_buffered(right)?;

    let mut first_diff: Option<u64> = None;
    let mut hunks: Vec<BinaryHunk> = Vec::new();
    let mut truncated = false;
    let mut base: u64 = 0;

    loop {
        let (llen, rlen, mismatch) = {
            let lb = match lreader.fill_buf() {
                Ok(buf) => buf,
                Err(e) => return Err(io_error(left, e)),
            };
            let rb = match rreader.fill_buf() {
                Ok(buf) => buf,
                Err(e) => return Err(io_error(right, e)),
            };
            let mismatch = lb.iter().zip(rb.iter()).position(|(a, b)| a != b);
            (lb.len(), rb.len(), mismatch)
        };

        if let Some(i) = mismatch {
            let offset = base.saturating_add(i as u64);
            if first_diff.is_none() {
                first_diff = Some(offset);
            }
            if hunks.len() >= max_hunks {
                truncated = true;
                break;
            }
            seek_to(&mut lreader, offset, left)?;
            let left_bytes = read_up_to(&mut lreader, HUNK, left)?;
            seek_to(&mut rreader, offset, right)?;
            let right_bytes = read_up_to(&mut rreader, HUNK, right)?;
            hunks.push(BinaryHunk {
                offset,
                left: left_bytes,
                right: right_bytes,
            });
            let resume = offset.saturating_add(HUNK as u64);
            seek_to(&mut lreader, resume, left)?;
            seek_to(&mut rreader, resume, right)?;
            base = resume;
            continue;
        }

        if llen == 0 && rlen == 0 {
            // Both sides at EOF with equal bytes so far.
            break;
        }
        if llen == 0 || rlen == 0 {
            // One file is a strict prefix of the other; the shorter
            // side has consumed exactly `base` bytes.
            if first_diff.is_none() {
                first_diff = Some(base);
            }
            if hunks.len() >= max_hunks {
                truncated = true;
            }
            break;
        }

        let common = llen.min(rlen);
        lreader.consume(common);
        rreader.consume(common);
        base = base.saturating_add(common as u64);
    }

    Ok(BinaryComparison {
        left_size,
        right_size,
        first_diff,
        hunks,
        truncated,
    })
}

/// Streams both files in chunks and reports whether their bytes are
/// equal. Used by folder comparison when `content_compare` is set.
/// Read errors propagate as [`Error::Io`] with the failing path.
pub(crate) fn files_equal(left: &Path, right: &Path) -> Result<bool> {
    let mut lreader = open_buffered(left)?;
    let mut rreader = open_buffered(right)?;
    loop {
        let (llen, rlen, equal) = {
            let lb = match lreader.fill_buf() {
                Ok(buf) => buf,
                Err(e) => return Err(io_error(left, e)),
            };
            let rb = match rreader.fill_buf() {
                Ok(buf) => buf,
                Err(e) => return Err(io_error(right, e)),
            };
            let equal = lb.iter().zip(rb.iter()).all(|(a, b)| a == b);
            (lb.len(), rb.len(), equal)
        };
        if !equal {
            return Ok(false);
        }
        if llen == 0 || rlen == 0 {
            // Equal only when both sides ended together.
            return Ok(llen == rlen);
        }
        let common = llen.min(rlen);
        lreader.consume(common);
        rreader.consume(common);
    }
}

/// Size of the file at `path`; a missing path is [`Error::Missing`].
fn file_size(path: &Path) -> Result<u64> {
    match std::fs::metadata(path) {
        Ok(meta) => Ok(meta.len()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::Missing {
            path: path.display().to_string(),
        }),
        Err(e) => Err(Error::Io {
            path: path.display().to_string(),
            source: e,
        }),
    }
}

/// Opens a buffered reader; directories fail on the first read with
/// an I/O error, which maps to [`Error::Io`].
fn open_buffered(path: &Path) -> Result<BufReader<File>> {
    let file = File::open(path).map_err(|e| io_error(path, e))?;
    Ok(BufReader::with_capacity(CHUNK, file))
}

/// Reads up to `max` bytes from `reader` via the buffer.
fn read_up_to(reader: &mut BufReader<File>, max: usize, path: &Path) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(max);
    while out.len() < max {
        let taken = {
            let buf = match reader.fill_buf() {
                Ok(buf) => buf,
                Err(e) => return Err(io_error(path, e)),
            };
            if buf.is_empty() {
                break;
            }
            let take = (max - out.len()).min(buf.len());
            match buf.get(..take) {
                Some(chunk) => {
                    out.extend_from_slice(chunk);
                    take
                }
                None => break, // unreachable in practice: take <= buf.len()
            }
        };
        reader.consume(taken);
    }
    Ok(out)
}

/// Seeks `reader` to the absolute offset `pos`.
fn seek_to(reader: &mut BufReader<File>, pos: u64, path: &Path) -> Result<()> {
    reader
        .seek(SeekFrom::Start(pos))
        .map_err(|e| io_error(path, e))?;
    Ok(())
}

/// Wraps an I/O error with the failing path.
fn io_error(path: &Path, source: std::io::Error) -> Error {
    Error::Io {
        path: path.display().to_string(),
        source,
    }
}
