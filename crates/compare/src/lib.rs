//! File, folder and binary comparison over the filesystem.
//!
//! Layer 1: builds on the pure [`autocompare_diff`] algorithms. All
//! functions return errors instead of panicking; hostile paths, huge
//! files and deep trees are handled with limits, never crashes.

#![forbid(unsafe_code)]

use std::fmt;
use std::path::Path;
use std::time::SystemTime;

use autocompare_diff::DiffScript;

/// Errors from comparison, with plain-English messages.
#[derive(Debug)]
pub enum Error {
    /// A path does not exist or is not the expected kind.
    Missing { path: String },
    /// Reading a path failed.
    Io {
        path: String,
        source: std::io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Missing { path } => write!(f, "path not found: {path}"),
            Error::Io { path, source } => write!(f, "reading {path} failed: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Shorthand for comparison results.
pub type Result<T> = std::result::Result<T, Error>;

/// Options that tune how text comparison decides equality.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextOptions {
    /// Ignore differences in letter case.
    pub ignore_case: bool,
    /// Ignore whitespace differences inside lines: leading, trailing
    /// and repeated internal spaces or tabs.
    pub ignore_space_change: bool,
}

/// Loads a text file for comparison: reads the bytes, decodes UTF-8
/// leniently (invalid bytes become replacement characters) and splits
/// into lines. A trailing newline does not create a last empty line.
pub fn load_lines(path: &Path) -> Result<Vec<String>> {
    // STUB: replaced by the real implementation in crates/compare.
    let _ = path;
    Ok(Vec::new())
}

/// The result of comparing two text files.
pub struct TextComparison {
    /// Display lines of the left file.
    pub left_lines: Vec<String>,
    /// Display lines of the right file.
    pub right_lines: Vec<String>,
    /// The edit script between the comparison keys of both sides.
    pub script: DiffScript,
}

/// Compares two text files line by line, honouring `opts`.
pub fn compare_text_files(left: &Path, right: &Path, opts: &TextOptions) -> Result<TextComparison> {
    // STUB: replaced by the real implementation in crates/compare.
    let _ = (left, right, opts);
    Ok(TextComparison {
        left_lines: Vec::new(),
        right_lines: Vec::new(),
        script: DiffScript {
            ops: Vec::new(),
            exact: true,
        },
    })
}

/// Metadata for one side of a compared node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeInfo {
    /// Size in bytes (0 for directories).
    pub size: u64,
    /// Last modified time.
    pub modified: SystemTime,
}

/// How one node compares between the two folder trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderStatus {
    /// Present on both sides and equal by the active comparison rule.
    Same,
    /// Present on both sides but different (size or content).
    Different,
    /// Present on both sides; the left copy is newer.
    NewerLeft,
    /// Present on both sides; the right copy is newer.
    NewerRight,
    /// Only present on the left side.
    LeftOnly,
    /// Only present on the right side.
    RightOnly,
}

/// One row of a folder comparison.
#[derive(Debug, Clone)]
pub struct FolderEntry {
    /// Path relative to the compared roots, '/'-separated.
    pub rel_path: String,
    /// Whether the node is a directory.
    pub is_dir: bool,
    /// Left-side metadata when the node exists there.
    pub left: Option<NodeInfo>,
    /// Right-side metadata when the node exists there.
    pub right: Option<NodeInfo>,
    /// The comparison verdict.
    pub status: FolderStatus,
}

/// Options for a folder comparison.
#[derive(Debug, Clone)]
pub struct FolderOptions {
    /// Compare file bytes when sizes match, not only size and time.
    pub content_compare: bool,
    /// Maximum recursion depth; deeper levels are skipped and the
    /// result is marked truncated.
    pub depth_limit: usize,
    /// Match names case-insensitively.
    pub case_insensitive: bool,
    /// Only include names matching this simple glob ('*' and '?').
    pub include: Option<String>,
    /// Exclude names matching this simple glob ('*' and '?').
    pub exclude: Option<String>,
}

impl Default for FolderOptions {
    fn default() -> Self {
        Self {
            content_compare: false,
            depth_limit: 32,
            case_insensitive: false,
            include: None,
            exclude: None,
        }
    }
}

/// The result of comparing two folder trees.
#[derive(Debug, Clone)]
pub struct FolderComparison {
    /// Entries sorted by relative path.
    pub entries: Vec<FolderEntry>,
    /// Whether the depth limit stopped the walk early.
    pub truncated: bool,
}

/// Compares two folder trees.
///
/// Classification: nodes missing on one side are `LeftOnly` or
/// `RightOnly`. When both exist, files differing in size are
/// `Different`; otherwise `content_compare` decides by bytes, and
/// without it the newer side (by modified time) wins, or `Same` when
/// times match. Directories are compared by modified time and walked
/// recursively. Symlinks are never followed.
pub fn compare_folders(
    left: &Path,
    right: &Path,
    opts: &FolderOptions,
) -> Result<FolderComparison> {
    // STUB: replaced by the real implementation in crates/compare.
    let _ = (left, right, opts);
    Ok(FolderComparison {
        entries: Vec::new(),
        truncated: false,
    })
}

/// One run of differing bytes, capped in length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryHunk {
    /// Offset of the first differing byte.
    pub offset: u64,
    /// At most 64 differing bytes from the left file.
    pub left: Vec<u8>,
    /// The same span from the right file.
    pub right: Vec<u8>,
}

/// The result of a byte-by-byte comparison.
#[derive(Debug, Clone, Default)]
pub struct BinaryComparison {
    /// Size of the left file.
    pub left_size: u64,
    /// Size of the right file.
    pub right_size: u64,
    /// Offset of the first differing byte, if any.
    pub first_diff: Option<u64>,
    /// Differing runs, at most `max_hunks`.
    pub hunks: Vec<BinaryHunk>,
    /// Whether more differing runs exist beyond the hunk cap.
    pub truncated: bool,
}

/// Compares two files byte by byte, collecting up to `max_hunks`
/// differing runs of at most 64 bytes each.
///
/// Files that differ only in length report `first_diff` at the first
/// differing byte, or at the shorter length when prefixes match.
pub fn compare_binary_files(
    left: &Path,
    right: &Path,
    max_hunks: usize,
) -> Result<BinaryComparison> {
    // STUB: replaced by the real implementation in crates/compare.
    let _ = (left, right, max_hunks);
    Ok(BinaryComparison::default())
}
