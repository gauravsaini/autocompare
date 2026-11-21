//! File, folder and binary comparison over the filesystem.
//!
//! Layer 1: builds on the pure [`autocompare_diff`] algorithms. All
//! functions return errors instead of panicking; hostile paths, huge
//! files and deep trees are handled with limits, never crashes.

#![forbid(unsafe_code)]

mod binary;
mod folder;
mod folder_merge;
pub mod jobs;
mod media;
mod picture;
mod sync;
mod table;
mod text;
mod vfs_compare;

pub use autocompare_diff::hex::HexAlignment;
pub use folder::{FolderCompare, compare_folders_cancellable};
pub use folder_merge::FolderMerge;
pub use jobs::{JobHandle, JobProgress, JobRunner};
pub use media::{AudioTolerance, compare_audio_files_with_tolerance};
pub use sync::{
    SyncLiveLog, SyncMode, SyncPlanner, gui_action_for, gui_skip_reason_for, gui_sync_modes,
    gui_sync_options, is_sync_conflict_reason, sync_conflict_next, sync_conflict_prev,
};
pub use table::{TableKeyFileComparison, TableKeyOptions};
pub use vfs_compare::{compare_vfs_folders, compare_vfs_folders_cancellable};

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
    /// A path exists but its content cannot be handled as requested.
    Unsupported { path: String, what: &'static str },
    /// Operation was cancelled.
    Cancelled,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Missing { path } => write!(f, "path not found: {path}"),
            Error::Io { path, source } => write!(f, "reading {path} failed: {source}"),
            Error::Unsupported { path, what } => write!(f, "{path}: {what}"),
            Error::Cancelled => write!(f, "operation cancelled"),
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
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
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
    text::load_lines(path)
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
    text::compare_text_files(left, right, opts)
}

/// Options for table comparison.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct TableOptions {
    /// Field delimiter (',' by default).
    pub delimiter: char,
    /// Treat the first row as a header.
    pub header: bool,
}

impl Default for TableOptions {
    fn default() -> Self {
        Self {
            delimiter: ',',
            header: true,
        }
    }
}

/// The result of comparing two delimited tables.
pub struct TableComparison {
    /// Header cells of the left table (empty when no header).
    pub header_l: Vec<String>,
    /// Header cells of the right table.
    pub header_r: Vec<String>,
    /// Data rows of the left table.
    pub rows_l: Vec<Vec<String>>,
    /// Data rows of the right table.
    pub rows_r: Vec<Vec<String>>,
    /// Row-level edit script between the data rows.
    pub script: DiffScript,
}

/// Compares two delimited files row by row, cell by cell. The files
/// are read whole (quoted fields may span lines) and decoded
/// leniently.
pub fn compare_table_files(
    left: &Path,
    right: &Path,
    opts: &TableOptions,
) -> Result<TableComparison> {
    table::compare_table_files(left, right, opts)
}

/// Compares two delimited files aligned by key columns with tolerant
/// cell comparison (numeric tolerance, case folding, space trimming
/// via `key_opts`). Headers are still exposed separately and take no
/// part in the row alignment.
pub fn compare_table_files_by_key(
    left: &Path,
    right: &Path,
    opts: &TableOptions,
    key_opts: &TableKeyOptions,
) -> Result<TableKeyFileComparison> {
    table::compare_table_files_by_key(left, right, opts, key_opts)
}

/// Metadata for one side of a compared node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NodeInfo {
    /// Size in bytes (0 for directories).
    pub size: u64,
    /// Last modified time.
    pub modified: SystemTime,
}

/// How one node compares between the two folder trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    folder::compare_folders(left, right, opts)
}

/// One run of differing bytes, capped in length.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BinaryHunk {
    /// Offset of the first differing byte.
    pub offset: u64,
    /// At most 64 differing bytes from the left file.
    pub left: Vec<u8>,
    /// The same span from the right file.
    pub right: Vec<u8>,
}

/// The result of a byte-by-byte comparison.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
    binary::compare_binary_files(left, right, max_hunks)
}

/// Compares two files with a hex alignment strategy (`None`, `Fast`
/// or `Complete`; see [`HexAlignment`).
///
/// Unlike [`compare_binary_files`], both files are read whole (up to
/// 16 MiB each — larger files are [`Error::Unsupported`]) and
/// aligned: `Complete` finds the minimal edit script so an insertion
/// shows as a gap, while `None` compares strictly positionally.
/// Differing runs are chunked into hunks of at most 64 bytes; gap
/// hunks carry bytes on one side only.
pub fn compare_binary_files_aligned(
    left: &Path,
    right: &Path,
    max_hunks: usize,
    alignment: HexAlignment,
) -> Result<BinaryComparison> {
    binary::compare_binary_files_aligned(left, right, max_hunks, alignment)
}

/// Direction of a folder sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SyncDirection {
    /// Copy left-side content over the right side.
    LeftToRight,
    /// Copy right-side content over the left side.
    RightToLeft,
    /// Copy whichever side is newer over the older one.
    NewerWins,
}

/// Options for building a sync plan.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SyncOptions {
    /// Which side wins.
    pub direction: SyncDirection,
    /// Delete one-sided entries on the losing side (mirror). Only
    /// meaningful for one-directional syncs.
    pub prune: bool,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            direction: SyncDirection::LeftToRight,
            prune: false,
        }
    }
}

/// What one planned step does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SyncAction {
    /// Copy the left copy over the right one.
    CopyToRight,
    /// Copy the right copy over the left one.
    CopyToLeft,
    /// Delete the right copy (prune).
    DeleteRight,
    /// Delete the left copy (prune).
    DeleteLeft,
}

/// One planned change.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SyncStep {
    /// Path relative to the roots, '/'-separated.
    pub rel_path: String,
    /// Whether the subject is a directory.
    pub is_dir: bool,
    /// What would happen.
    pub action: SyncAction,
}

/// A whole sync plan: the steps to apply and the entries intentionally
/// left alone, with reasons.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SyncPlan {
    /// Actions to apply, parents before children.
    pub steps: Vec<SyncStep>,
    /// Entries left alone: (relative path, plain-English reason).
    pub skipped: Vec<(String, String)>,
    /// Whether the underlying comparison stopped at the depth limit;
    /// deeper entries are not planned.
    pub truncated: bool,
}

/// Outcome of applying a plan.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SyncReport {
    /// Files successfully copied.
    pub copied: usize,
    /// Files or directories successfully deleted.
    pub deleted: usize,
    /// Steps that failed: (relative path, plain-English error).
    pub failed: Vec<(String, String)>,
}

/// Builds a sync plan from a folder comparison. Read-only: building a
/// plan never touches the filesystem beyond the comparison walk.
///
/// Rules: `NewerLeft`/`LeftOnly` copy rightwards (in LeftToRight and
/// NewerWins), `NewerRight`/`RightOnly` copy leftwards (in RightToLeft
/// and NewerWins); a one-directional sync never overwrites a newer
/// destination (that step is skipped with a reason instead).
/// `Different` entries are always skipped: no side wins on content.
/// With `prune`, one-sided entries on the losing side are deleted
/// instead of being left alone. Symlinks are never followed, so they
/// are never copied or deleted.
///
/// The walk uses [`FolderOptions::default`]; for depth limits,
/// content compare or name filters see [`plan_sync_with_compare`].
pub fn plan_sync(left: &Path, right: &Path, opts: &SyncOptions) -> Result<SyncPlan> {
    sync::plan_sync(left, right, opts)
}

/// Builds a sync plan with explicit folder comparison rules. Same
/// rules as [`plan_sync`], but the walk honours `compare` (depth
/// limit, content compare, case folding, include/exclude filters):
/// deeper entries are not planned and the plan reports `truncated`,
/// filtered-out names appear in neither steps nor skips.
pub fn plan_sync_with_compare(
    left: &Path,
    right: &Path,
    opts: &SyncOptions,
    compare: &FolderOptions,
) -> Result<SyncPlan> {
    sync::plan_sync_with_compare(left, right, opts, compare)
}

/// Applies a plan: copies files (creating parent directories as
/// needed) and deletes files or whole directories. Each step's
/// failure is recorded in the report and the run continues; the roots
/// themselves must exist. Never panics.
pub fn apply_sync(left: &Path, right: &Path, plan: &SyncPlan) -> Result<SyncReport> {
    sync::apply_sync(left, right, plan)
}

/// Options for picture comparison.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PictureOptions {
    /// When set, a heatmap PNG (identical pixels dimmed, differing
    /// pixels red) is written to this path. This is a real file write.
    pub heatmap: Option<std::path::PathBuf>,
}

/// The result of comparing two pictures.
pub struct PictureComparison {
    /// Sizes and difference statistics.
    pub diff: autocompare_diff::PictureDiff,
}

/// Compares two PNG files pixel by pixel over their overlap. Files
/// that do not decode as PNG are [`Error::Unsupported`]. When
/// `opts.heatmap` is set, the heatmap is written (a declared file
/// write) before the result returns.
pub fn compare_picture_files(
    left: &Path,
    right: &Path,
    opts: &PictureOptions,
) -> Result<PictureComparison> {
    picture::compare_picture_files(left, right, opts)
}

/// What one step of a three-way tree merge does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TreeMergeAction {
    /// Take the left side's copy.
    TakeLeft,
    /// Take the right side's copy.
    TakeRight,
    /// Write text merged from base/left/right (a clean merge; the
    /// lines are in the step's `merged`).
    MergedText,
    /// Both sides changed differently: the step carries either
    /// marker-laden text (`merged` set) or nothing (binary conflict:
    /// the left copy is written), and the path is recorded in the
    /// plan's conflicts.
    Conflict,
    /// Create this directory.
    MakeDir,
}

/// One planned write in a three-way tree merge.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TreeMergeStep {
    /// Path relative to the roots, '/'-separated.
    pub rel_path: String,
    /// What to do.
    pub action: TreeMergeAction,
    /// Lines to write for `MergedText` and text-conflict steps;
    /// `None` when the content is copied from a side.
    pub merged: Option<Vec<String>>,
}

/// A whole three-way merge plan.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TreeMergePlan {
    /// Steps sorted by relative path, `MakeDir` steps before their
    /// children.
    pub steps: Vec<TreeMergeStep>,
    /// Relative paths with conflicts, sorted.
    pub conflicts: Vec<String>,
    /// Entries left out of the output, with reasons: (path, reason).
    pub skipped: Vec<(String, String)>,
}

/// Outcome of applying a merge plan.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TreeMergeReport {
    /// Files written (copies and merges).
    pub written: usize,
    /// Directories created.
    pub dirs_created: usize,
    /// Steps that failed: (relative path, plain-English error).
    pub failed: Vec<(String, String)>,
}

/// Plans a three-way merge of two trees over their common base.
/// Read-only.
///
/// Per file: identical sides take the left copy; a one-sided change
/// takes the changed side; both-changed text files (everything
/// decodes as UTF-8, base included) merge via three-way content
/// merge — cleanly into `MergedText`, overlapping edits into a
/// `Conflict` carrying marker lines. Both-changed binaries conflict
/// with the left copy written. A side that deleted while the other
/// side left the file unchanged from base omits the file (recorded
/// with a reason); a delete against a change keeps the changed copy
/// and records a conflict. Two new files with different content
/// conflict (text merges over an empty base; binaries take the left
/// copy). Directories existing on either side produce `MakeDir`
/// steps so empty ones survive.
pub fn plan_tree_merge(base: &Path, left: &Path, right: &Path) -> Result<TreeMergePlan> {
    folder_merge::plan_tree_merge(base, left, right)
}

/// Applies a merge plan into `output`, taking copied content from
/// the `left` and `right` roots. The output directory must not
/// exist or must be empty (anything else is
/// [`Error::Unsupported`]); it is created as needed. Each step's
/// failure is recorded and the run continues. Never panics.
pub fn apply_tree_merge(
    left: &Path,
    right: &Path,
    plan: &TreeMergePlan,
    output: &Path,
) -> Result<TreeMergeReport> {
    folder_merge::apply_tree_merge(left, right, plan, output)
}

/// The result of comparing two audio files.
pub struct AudioComparison {
    /// Decoded stream statistics.
    pub diff: autocompare_diff::AudioDiff,
}

/// Compares two WAV or MP3 files sample by sample over the shorter
/// stream's duration. Files that do not decode are
/// [`Error::Unsupported`]; a sample-rate or channel mismatch is
/// reported through the diff (not comparable, honestly), not as an
/// error.
pub fn compare_audio_files(left: &Path, right: &Path) -> Result<AudioComparison> {
    media::compare_audio_files(left, right)
}
