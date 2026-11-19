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
use std::sync::atomic::{AtomicBool, Ordering};

use autocompare_diff::hex::{HexAlignment, HexOp, diff_hex};

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
    files_equal_cancellable(left, right, None)
}

/// Streams both files in chunks with optional cancellation.
pub(crate) fn files_equal_cancellable(
    left: &Path,
    right: &Path,
    cancel: Option<&AtomicBool>,
) -> Result<bool> {
    let mut lreader = open_buffered(left)?;
    let mut rreader = open_buffered(right)?;
    loop {
        if let Some(c) = cancel
            && c.load(Ordering::Relaxed)
        {
            return Err(Error::Cancelled);
        }
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

/// Maximum bytes read per file for alignment-aware hex comparison.
///
/// Aligned modes need the whole input in memory (`Complete` runs the
/// Myers engine, `Fast` looks ahead in a window), so files beyond
/// this cap report [`Error::Unsupported`] instead of comparing.
/// Unbounded files stay on [`compare_binary_files`].
pub const ALIGNED_HEX_MAX_BYTES: usize = 16 * 1024 * 1024;

/// Compares two files with the requested hex alignment strategy
/// ([`HexAlignment::None`], [`HexAlignment::Fast`] or
/// [`HexAlignment::Complete`]).
///
/// Both files are read whole up to [`ALIGNED_HEX_MAX_BYTES`] and
/// aligned by `autocompare_diff::hex::diff_hex`. Differing runs are
/// chunked into hunks of at most 64 bytes like [`compare_binary_files`];
/// gap runs carry bytes on one side only (the other side is empty).
/// A gap's hunk offset is its left offset when it has left bytes,
/// otherwise its right offset.
///
/// `first_diff` is the alignment's first difference; `truncated` is
/// true when more differing 64-byte chunks exist than `max_hunks`
/// allows (in particular `max_hunks == 0` with any difference).
pub(crate) fn compare_binary_files_aligned(
    left: &Path,
    right: &Path,
    max_hunks: usize,
    alignment: HexAlignment,
) -> Result<BinaryComparison> {
    let left_bytes = read_capped(left)?;
    let right_bytes = read_capped(right)?;
    let left_size = left_bytes.len() as u64;
    let right_size = right_bytes.len() as u64;
    let diff = diff_hex(&left_bytes, &right_bytes, alignment);

    let mut hunks: Vec<BinaryHunk> = Vec::new();
    let mut needed: usize = 0;
    for op in &diff.ops {
        match op {
            HexOp::Equal { .. } => {}
            HexOp::Different {
                left_offset,
                right_offset,
                len,
            } => {
                push_hunk_run(
                    &left_bytes,
                    &right_bytes,
                    *left_offset,
                    *right_offset,
                    *len,
                    true,
                    true,
                    max_hunks,
                    &mut hunks,
                    &mut needed,
                );
            }
            HexOp::LeftOnly { left_offset, len } => {
                push_hunk_run(
                    &left_bytes,
                    &right_bytes,
                    *left_offset,
                    0,
                    *len,
                    true,
                    false,
                    max_hunks,
                    &mut hunks,
                    &mut needed,
                );
            }
            HexOp::RightOnly { right_offset, len } => {
                push_hunk_run(
                    &left_bytes,
                    &right_bytes,
                    0,
                    *right_offset,
                    *len,
                    false,
                    true,
                    max_hunks,
                    &mut hunks,
                    &mut needed,
                );
            }
        }
    }

    Ok(BinaryComparison {
        left_size,
        right_size,
        first_diff: diff.first_diff,
        hunks,
        truncated: needed > max_hunks,
    })
}

/// Reads a whole file capped at [`ALIGNED_HEX_MAX_BYTES`].
///
/// Missing paths map to [`Error::Missing`], oversized files to
/// [`Error::Unsupported`], anything else to [`Error::Io`].
/// Directories fail on the read with [`Error::Io`], matching
/// [`compare_binary_files`].
fn read_capped(path: &Path) -> Result<Vec<u8>> {
    let size = file_size(path)?;
    if size > ALIGNED_HEX_MAX_BYTES as u64 {
        return Err(Error::Unsupported {
            path: path.display().to_string(),
            what: "file too large for aligned hex compare (16 MiB cap)",
        });
    }
    match std::fs::read(path) {
        Ok(bytes) => {
            if bytes.len() > ALIGNED_HEX_MAX_BYTES {
                Err(Error::Unsupported {
                    path: path.display().to_string(),
                    what: "file too large for aligned hex compare (16 MiB cap)",
                })
            } else {
                Ok(bytes)
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::Missing {
            path: path.display().to_string(),
        }),
        Err(e) => Err(io_error(path, e)),
    }
}

/// Copies one aligned run into 64-byte hunks.
///
/// `take_left`/`take_right` select which sides carry bytes; the hunk
/// offset is the left offset when the left side is present, otherwise
/// the right offset. `needed` counts every chunk the run requires so
/// the caller can mark truncation honestly.
#[allow(clippy::too_many_arguments)]
fn push_hunk_run(
    left_bytes: &[u8],
    right_bytes: &[u8],
    left_offset: u64,
    right_offset: u64,
    len: usize,
    take_left: bool,
    take_right: bool,
    max_hunks: usize,
    hunks: &mut Vec<BinaryHunk>,
    needed: &mut usize,
) {
    let mut done = 0usize;
    while done < len {
        let take = (len.saturating_sub(done)).min(HUNK);
        *needed = needed.saturating_add(1);
        if hunks.len() < max_hunks {
            let left_at = left_offset.saturating_add(done as u64);
            let right_at = right_offset.saturating_add(done as u64);
            let left = if take_left {
                slice_at(left_bytes, left_at, take)
            } else {
                Vec::new()
            };
            let right = if take_right {
                slice_at(right_bytes, right_at, take)
            } else {
                Vec::new()
            };
            let offset = if take_left { left_at } else { right_at };
            hunks.push(BinaryHunk {
                offset,
                left,
                right,
            });
        }
        done = done.saturating_add(take);
        if take == 0 {
            break;
        }
    }
}

/// Copies up to `len` bytes at `offset`; short reads clip, never panic.
fn slice_at(bytes: &[u8], offset: u64, len: usize) -> Vec<u8> {
    let start = (offset as usize).min(bytes.len());
    let end = start.saturating_add(len).min(bytes.len());
    bytes.get(start..end).unwrap_or(&[]).to_vec()
}
