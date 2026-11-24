//! Hex (binary) compare IPC surface for the desktop shell.
//!
//! Ownership: wave-1b. Wave-1R wires these handlers into the Tauri
//! invoke handler (`lib.rs`) and owns `commands/mod.rs`.
//!
//! # Paged-fetch rule
//!
//! No command reads a whole file into memory or returns whole-file
//! bytes over IPC. [`hex_compare`] streams both files in 64 KiB chunks
//! and keeps only sizes plus the first-difference offset; the session
//! store keeps paths and summary metadata (never file bytes).
//! [`hex_get_page`] serves bounded windows (`len` clamped to
//! [`MAX_PAGE_LEN`]); [`hex_find`] streams from `from_offset` keeping
//! only a needle-sized overlap buffered.
//!
//! # Engine swap note (Wave-1R / later wave)
//!
//! This module streams bytes directly so it needs no new crate
//! dependency (this wave may not touch `Cargo.toml`). Once
//! `autocompare-compare` is added to the desktop crate, the body of
//! [`summarize_paths`] can delegate to
//! `autocompare_compare::compare_binary_files(left, right, 0)` (or
//! `compare_binary_files_aligned(.., HexAlignment::Complete)` when the
//! UI needs shift-aware offsets) and map `BinaryComparison` onto
//! [`HexSummary`]. DTO shapes stay unchanged, so `ipc/hex.ts` is
//! unaffected.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Maximum bytes served per side by one [`hex_get_page`] call.
pub const MAX_PAGE_LEN: u64 = 4096;

/// Streaming chunk size for summary and find scans.
const CHUNK: usize = 64 * 1024;

/// Maximum accepted needle size in bytes for [`hex_find`].
pub const MAX_NEEDLE_BYTES: usize = 256;

/// Summary of one hex comparison. Mirrors `ipc/hex.ts` `HexSummary`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HexSummary {
    /// Opaque session id for follow-up `hex_get_page` / `hex_find` calls.
    pub session_id: String,
    /// Size of the left file in bytes.
    pub left_size: u64,
    /// Size of the right file in bytes.
    pub right_size: u64,
    /// True when both files are byte-identical.
    pub equal: bool,
    /// Offset of the first differing byte, if any.
    pub first_diff: Option<u64>,
}

/// One bounded byte window over both files. Mirrors `ipc/hex.ts` `HexPage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HexPage {
    /// Session this page belongs to.
    pub session_id: String,
    /// Requested window start.
    pub offset: u64,
    /// Effective window length (requested `len` clamped to [`MAX_PAGE_LEN`]).
    pub len: u64,
    /// Left-file bytes at `[offset, offset + left.len())`; empty past EOF.
    pub left: Vec<u8>,
    /// Right-file bytes at `[offset, offset + right.len())`; empty past EOF.
    pub right: Vec<u8>,
    /// `max(left_size, right_size)`; the UI pages `[0, total)`.
    pub total: u64,
    /// True when bytes remain beyond this window.
    pub truncated: bool,
}

/// Server-side session: paths plus sizes for paging.
/// The summary itself is returned to the caller at compare time;
/// file bytes are never cached here.
#[derive(Debug, Clone)]
struct HexSession {
    left: PathBuf,
    right: PathBuf,
    left_size: u64,
    right_size: u64,
}

static SESSIONS: LazyLock<Mutex<HashMap<String, HexSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn new_session_id(prefix: &str) -> String {
    let n = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let nanos: u128 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_nanos(),
        Err(_) => 0,
    };
    format!("{prefix}-{nanos:x}-{n}-{pid}", pid = std::process::id())
}

fn missing_msg(path: &Path) -> String {
    format!("path not found: {}", path.display())
}

fn io_msg(path: &Path, err: &std::io::Error) -> String {
    format!("reading {} failed: {err}", path.display())
}

fn open_msg(path: &Path, err: &std::io::Error) -> String {
    if err.kind() == std::io::ErrorKind::NotFound {
        missing_msg(path)
    } else {
        io_msg(path, err)
    }
}

fn store_msg() -> String {
    "hex session store unavailable".to_string()
}

fn unknown_session_msg(id: &str) -> String {
    format!("unknown hex session: {id}")
}

fn file_len(path: &Path) -> Result<u64, String> {
    match std::fs::metadata(path) {
        Ok(meta) => Ok(meta.len()),
        Err(err) => Err(open_msg(path, &err)),
    }
}

fn get_session(id: &str) -> Result<HexSession, String> {
    match SESSIONS.lock() {
        Ok(guard) => match guard.get(id) {
            Some(session) => Ok(session.clone()),
            None => Err(unknown_session_msg(id)),
        },
        Err(_) => Err(store_msg()),
    }
}

fn put_session(id: &str, session: HexSession) -> Result<(), String> {
    match SESSIONS.lock() {
        Ok(mut guard) => {
            guard.insert(id.to_string(), session);
            Ok(())
        }
        Err(_) => Err(store_msg()),
    }
}

/// Streams both files and reports `(left_size, right_size,
/// first_diff, equal)`. Stops at the first difference: hunks are a
/// later wave's job, summaries only need the first offset.
fn summarize_paths(left: &Path, right: &Path) -> Result<(u64, u64, Option<u64>, bool), String> {
    let left_size = file_len(left)?;
    let right_size = file_len(right)?;
    let left_file = match File::open(left) {
        Ok(f) => f,
        Err(err) => return Err(open_msg(left, &err)),
    };
    let right_file = match File::open(right) {
        Ok(f) => f,
        Err(err) => return Err(open_msg(right, &err)),
    };
    let mut lreader = BufReader::with_capacity(CHUNK, left_file);
    let mut rreader = BufReader::with_capacity(CHUNK, right_file);
    let mut base: u64 = 0;
    let mut first_diff: Option<u64> = None;

    loop {
        let (llen, rlen, mismatch) = {
            let lb = match lreader.fill_buf() {
                Ok(buf) => buf,
                Err(err) => return Err(io_msg(left, &err)),
            };
            let rb = match rreader.fill_buf() {
                Ok(buf) => buf,
                Err(err) => return Err(io_msg(right, &err)),
            };
            let at = lb.iter().zip(rb.iter()).position(|(a, b)| a != b);
            (lb.len(), rb.len(), at)
        };

        if let Some(i) = mismatch {
            let offset = base.saturating_add(i as u64);
            if first_diff.is_none() {
                first_diff = Some(offset);
            }
            break;
        }
        if llen == 0 && rlen == 0 {
            break;
        }
        if llen == 0 || rlen == 0 {
            // One file is a strict prefix of the other.
            if first_diff.is_none() {
                first_diff = Some(base);
            }
            break;
        }
        let common = llen.min(rlen);
        lreader.consume(common);
        rreader.consume(common);
        base = base.saturating_add(common as u64);
    }

    let equal = first_diff.is_none();
    Ok((left_size, right_size, first_diff, equal))
}

/// Reads up to `len` bytes at `offset` (clamped to [`MAX_PAGE_LEN`]).
fn read_window(path: &Path, offset: u64, len: u64) -> Result<Vec<u8>, String> {
    let take = len.min(MAX_PAGE_LEN);
    let count = match usize::try_from(take) {
        Ok(n) => n,
        Err(_) => return Err(format!("page length out of range: {take}")),
    };
    let file = match File::open(path) {
        Ok(f) => f,
        Err(err) => return Err(open_msg(path, &err)),
    };
    let mut cursor = file;
    match cursor.seek(SeekFrom::Start(offset)) {
        Ok(_) => {}
        Err(err) => return Err(io_msg(path, &err)),
    }
    let mut buf = Vec::new();
    match cursor.take(count as u64).read_to_end(&mut buf) {
        Ok(_) => Ok(buf),
        Err(err) => Err(io_msg(path, &err)),
    }
}

fn hex_val(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        'a'..='f' => Some(c as u8 - b'a' + 10),
        'A'..='F' => Some(c as u8 - b'A' + 10),
        _ => None,
    }
}

/// Parses user-supplied hex (`"de ad 0A"`; whitespace ignored) into bytes.
fn parse_needle_hex(raw: &str) -> Result<Vec<u8>, String> {
    let digits: Vec<char> = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if digits.is_empty() {
        return Err("empty hex needle".to_string());
    }
    if !digits.len().is_multiple_of(2) {
        return Err("hex needle must have an even number of digits".to_string());
    }
    if digits.len() / 2 > MAX_NEEDLE_BYTES {
        return Err(format!(
            "hex needle too long (max {MAX_NEEDLE_BYTES} bytes)"
        ));
    }
    let mut out = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks(2) {
        let (hi, lo) = match (pair.first(), pair.get(1)) {
            (Some(h), Some(l)) => (*h, *l),
            _ => return Err("hex needle must have an even number of digits".to_string()),
        };
        match (hex_val(hi), hex_val(lo)) {
            (Some(h), Some(l)) => out.push(h * 16 + l),
            _ => return Err(format!("invalid hex digit in needle: {hi}{lo}")),
        }
    }
    Ok(out)
}

/// Streams the left file from `from` and reports the first needle offset.
fn find_in_file(path: &Path, needle: &[u8], from: u64) -> Result<Option<u64>, String> {
    if needle.is_empty() {
        return Err("empty hex needle".to_string());
    }
    let file = match File::open(path) {
        Ok(f) => f,
        Err(err) => return Err(open_msg(path, &err)),
    };
    let mut cursor = file;
    match cursor.seek(SeekFrom::Start(from)) {
        Ok(_) => {}
        Err(err) => return Err(io_msg(path, &err)),
    }
    let keep = needle.len().saturating_sub(1);
    let mut tail: Vec<u8> = Vec::new();
    let mut base = from;
    loop {
        let mut chunk = Vec::new();
        match (&mut cursor).take(CHUNK as u64).read_to_end(&mut chunk) {
            Ok(_) => {}
            Err(err) => return Err(io_msg(path, &err)),
        }
        if chunk.is_empty() {
            return Ok(None);
        }
        let mut haystack = Vec::with_capacity(tail.len() + chunk.len());
        haystack.extend(tail.iter().copied());
        haystack.extend(chunk.iter().copied());
        let window_start = base.saturating_sub(tail.len() as u64);
        if let Some(at) = haystack.windows(needle.len()).position(|w| w == needle) {
            return Ok(Some(window_start.saturating_add(at as u64)));
        }
        let keep_now = keep.min(haystack.len());
        let skip = haystack.len().saturating_sub(keep_now);
        tail = haystack.iter().skip(skip).copied().collect();
        base = base.saturating_add(chunk.len() as u64);
    }
}

/// Compares two files byte by byte and opens a hex session.
///
/// Returns sizes, the first-difference offset and the equality flag.
/// Never returns file bytes; page them with [`hex_get_page`].
#[tauri::command]
pub fn hex_compare(left: String, right: String) -> Result<HexSummary, String> {
    let left_path = Path::new(left.as_str());
    let right_path = Path::new(right.as_str());
    let (left_size, right_size, first_diff, equal) = summarize_paths(left_path, right_path)?;
    let session_id = new_session_id("hex");
    put_session(
        session_id.as_str(),
        HexSession {
            left: left_path.to_path_buf(),
            right: right_path.to_path_buf(),
            left_size,
            right_size,
        },
    )?;
    Ok(HexSummary {
        session_id,
        left_size,
        right_size,
        equal,
        first_diff,
    })
}

/// Serves one bounded byte window over both session files.
///
/// `len` is clamped to [`MAX_PAGE_LEN`]. Offsets past EOF yield empty
/// sides with `truncated == false`.
#[tauri::command]
pub fn hex_get_page(session: String, offset: u64, len: u64) -> Result<HexPage, String> {
    let snapshot = get_session(session.as_str())?;
    let take = len.min(MAX_PAGE_LEN);
    let left = read_window(snapshot.left.as_path(), offset, take)?;
    let right = read_window(snapshot.right.as_path(), offset, take)?;
    let total = snapshot.left_size.max(snapshot.right_size);
    let got = (left.len() as u64).max(right.len() as u64);
    let truncated = offset.saturating_add(got) < total;
    Ok(HexPage {
        session_id: session,
        offset,
        len: take,
        left,
        right,
        total,
        truncated,
    })
}

/// Finds the first occurrence of `needle_hex` in the session's left
/// file at or after `from_offset`. Returns `Ok(None)` when absent.
#[tauri::command]
pub fn hex_find(
    session: String,
    needle_hex: String,
    from_offset: u64,
) -> Result<Option<u64>, String> {
    let snapshot = get_session(session.as_str())?;
    let needle = parse_needle_hex(needle_hex.as_str())?;
    find_in_file(snapshot.left.as_path(), needle.as_slice(), from_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        let n = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "wave1b-hex-{tag}-{pid}-{n}.bin",
            pid = std::process::id()
        ))
    }

    fn write_bytes(path: &Path, bytes: &[u8]) -> bool {
        std::fs::write(path, bytes).is_ok()
    }

    fn remove_quiet(path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn identical_files_compare_equal() {
        let left = temp_path("identical-l");
        let right = temp_path("identical-r");
        let bytes = vec![1u8, 2, 3, 250, 0, 77];
        assert!(write_bytes(&left, bytes.as_slice()));
        assert!(write_bytes(&right, bytes.as_slice()));
        let res = hex_compare(left.display().to_string(), right.display().to_string());
        remove_quiet(&left);
        remove_quiet(&right);
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert_eq!(summary.first_diff, None);
        assert_eq!(summary.left_size, bytes.len() as u64);
        assert_eq!(summary.right_size, bytes.len() as u64);
        assert!(!summary.session_id.is_empty());
    }

    #[test]
    fn first_diff_offset_reported() {
        let left = temp_path("diff-l");
        let right = temp_path("diff-r");
        assert!(write_bytes(&left, &[1u8, 2, 3, 4]));
        assert!(write_bytes(&right, &[1u8, 2, 9, 4]));
        let res = hex_compare(left.display().to_string(), right.display().to_string());
        remove_quiet(&left);
        remove_quiet(&right);
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!summary.equal);
        assert_eq!(summary.first_diff, Some(2));
    }

    #[test]
    fn shorter_prefix_reports_its_len() {
        let left = temp_path("prefix-l");
        let right = temp_path("prefix-r");
        assert!(write_bytes(&left, &[1u8, 2]));
        assert!(write_bytes(&right, &[1u8, 2, 3]));
        let res = hex_compare(left.display().to_string(), right.display().to_string());
        remove_quiet(&left);
        remove_quiet(&right);
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!summary.equal);
        assert_eq!(summary.first_diff, Some(2));
    }

    #[test]
    fn empty_files_compare_equal() {
        let left = temp_path("empty-l");
        let right = temp_path("empty-r");
        let empty: Vec<u8> = Vec::new();
        assert!(write_bytes(&left, empty.as_slice()));
        assert!(write_bytes(&right, empty.as_slice()));
        let res = hex_compare(left.display().to_string(), right.display().to_string());
        remove_quiet(&left);
        remove_quiet(&right);
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert_eq!(summary.first_diff, None);
        assert_eq!(summary.left_size, 0);
        assert_eq!(summary.right_size, 0);
    }

    #[test]
    fn missing_file_errors() {
        let res = hex_compare(
            "/tmp/wave1b-definitely-missing-left.bin".to_string(),
            "/tmp/wave1b-definitely-missing-right.bin".to_string(),
        );
        assert!(res.is_err());
        let msg = match res {
            Ok(_) => return,
            Err(e) => e,
        };
        assert!(msg.contains("not found"));
    }

    #[test]
    fn page_len_is_clamped() {
        let left = temp_path("page-l");
        let right = temp_path("page-r");
        let bytes: Vec<u8> = (0u32..200).map(|i| (i % 251) as u8).collect();
        assert!(write_bytes(&left, bytes.as_slice()));
        assert!(write_bytes(&right, bytes.as_slice()));
        let cmp = hex_compare(left.display().to_string(), right.display().to_string());
        assert!(cmp.is_ok());
        let session_id = match cmp {
            Ok(s) => s.session_id,
            Err(_) => {
                remove_quiet(&left);
                remove_quiet(&right);
                return;
            }
        };
        let page = hex_get_page(session_id, 0, u64::MAX);
        remove_quiet(&left);
        remove_quiet(&right);
        assert!(page.is_ok());
        let page = match page {
            Ok(p) => p,
            Err(_) => return,
        };
        assert!(page.len <= MAX_PAGE_LEN);
        assert!((page.left.len() as u64) <= MAX_PAGE_LEN);
        assert_eq!(page.left, bytes);
        assert_eq!(page.right, bytes);
        assert!(!page.truncated);
    }

    #[test]
    fn page_past_eof_is_empty() {
        let left = temp_path("eof-l");
        let right = temp_path("eof-r");
        assert!(write_bytes(&left, &[7u8, 8]));
        assert!(write_bytes(&right, &[7u8, 8]));
        let cmp = hex_compare(left.display().to_string(), right.display().to_string());
        assert!(cmp.is_ok());
        let session_id = match cmp {
            Ok(s) => s.session_id,
            Err(_) => {
                remove_quiet(&left);
                remove_quiet(&right);
                return;
            }
        };
        let page = hex_get_page(session_id, 10_000, 64);
        remove_quiet(&left);
        remove_quiet(&right);
        assert!(page.is_ok());
        let page = match page {
            Ok(p) => p,
            Err(_) => return,
        };
        assert!(page.left.is_empty());
        assert!(page.right.is_empty());
        assert!(!page.truncated);
    }

    #[test]
    fn page_unknown_session_errors() {
        let res = hex_get_page("hex-no-such-session".to_string(), 0, 16);
        assert!(res.is_err());
        let msg = match res {
            Ok(_) => return,
            Err(e) => e,
        };
        assert!(msg.contains("unknown hex session"));
    }

    #[test]
    fn find_locates_needle() {
        let left = temp_path("find-l");
        let right = temp_path("find-r");
        assert!(write_bytes(&left, &[10u8, 20, 30, 40, 50]));
        assert!(write_bytes(&right, &[10u8, 20, 30, 40, 50]));
        let cmp = hex_compare(left.display().to_string(), right.display().to_string());
        assert!(cmp.is_ok());
        let session_id = match cmp {
            Ok(s) => s.session_id,
            Err(_) => {
                remove_quiet(&left);
                remove_quiet(&right);
                return;
            }
        };
        let hit = hex_find(session_id.clone(), "1E28".to_string(), 0);
        let miss = hex_find(session_id, "ff".to_string(), 0);
        remove_quiet(&left);
        remove_quiet(&right);
        assert!(hit.is_ok());
        assert!(miss.is_ok());
        assert_eq!(hit.ok(), Some(Some(2)));
        assert_eq!(miss.ok(), Some(None));
    }

    #[test]
    fn find_rejects_bad_hex() {
        for bad in ["", "abc", "zz", "0x1"] {
            let err = parse_needle_hex(bad);
            assert!(err.is_err());
        }
        assert!(parse_needle_hex("de ad 0A").is_ok());
    }

    #[test]
    fn summary_dto_round_trip() {
        let summary = HexSummary {
            session_id: "hex-abc".to_string(),
            left_size: 4,
            right_size: 5,
            equal: false,
            first_diff: Some(2),
        };
        let json_result = serde_json::to_string(&summary);
        assert!(json_result.is_ok());
        let json = match json_result {
            Ok(j) => j,
            Err(_) => return,
        };
        assert!(json.contains("sessionId"));
        assert!(json.contains("firstDiff"));
        let back: Result<HexSummary, _> = serde_json::from_str(json.as_str());
        assert!(back.is_ok());
        assert_eq!(back.ok(), Some(summary));
    }

    #[test]
    fn page_dto_round_trip() {
        let page = HexPage {
            session_id: "hex-abc".to_string(),
            offset: 0,
            len: 2,
            left: vec![1u8, 2],
            right: vec![1u8, 9],
            total: 4,
            truncated: true,
        };
        let json_result = serde_json::to_string(&page);
        assert!(json_result.is_ok());
        let json = match json_result {
            Ok(j) => j,
            Err(_) => return,
        };
        let back: Result<HexPage, _> = serde_json::from_str(json.as_str());
        assert!(back.is_ok());
        assert_eq!(back.ok(), Some(page));
    }
}
