use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Options tuning folder comparison behaviour for a session tab.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FolderCompareOptions {
    /// Whether to compare file content bytes when timestamps or sizes match.
    pub content_compare: bool,
    /// Maximum directory traversal depth limit, if any.
    pub depth_limit: Option<usize>,
    /// Whether file and directory name matching is case-insensitive.
    pub case_insensitive: bool,
    /// Pattern glob for files to include (e.g. `*.rs`).
    pub include: Option<String>,
    /// Pattern glob for files to exclude (e.g. `*.tmp`).
    pub exclude: Option<String>,
    /// Quick compare algorithm mode (e.g. "size", "timestamp", "crc", "contents").
    pub quick_compare: Option<String>,
    /// Whether to traverse and follow filesystem symlinks.
    pub follow_symlinks: bool,
    /// Whether to enter and inspect archive files (e.g. .zip, .tar).
    pub archive_traversal: bool,
}

impl FolderCompareOptions {
    /// Extracts folder compare options from a session tab options map.
    pub fn from_map(options: &HashMap<String, String>) -> Self {
        let content_compare = options
            .get("folder.content_compare")
            .is_some_and(|v| v == "true");

        let depth_limit = options
            .get("folder.depth_limit")
            .and_then(|v| v.parse::<usize>().ok());

        let case_insensitive = options
            .get("folder.case_insensitive")
            .is_some_and(|v| v == "true");

        let include = options
            .get("folder.include")
            .filter(|v| !v.is_empty())
            .cloned();

        let exclude = options
            .get("folder.exclude")
            .filter(|v| !v.is_empty())
            .cloned();

        let quick_compare = options
            .get("folder.quick_compare")
            .filter(|v| !v.is_empty())
            .cloned();

        let follow_symlinks = options
            .get("folder.follow_symlinks")
            .is_some_and(|v| v == "true");

        let archive_traversal = options
            .get("folder.archive_traversal")
            .is_some_and(|v| v == "true");

        Self {
            content_compare,
            depth_limit,
            case_insensitive,
            include,
            exclude,
            quick_compare,
            follow_symlinks,
            archive_traversal,
        }
    }

    /// Serializes options into key-value pairs suitable for a tab's options map.
    pub fn write_to_map(&self, options: &mut HashMap<String, String>) {
        if self.content_compare {
            options.insert("folder.content_compare".to_string(), "true".to_string());
        } else {
            options.remove("folder.content_compare");
        }

        if let Some(limit) = self.depth_limit {
            options.insert("folder.depth_limit".to_string(), limit.to_string());
        } else {
            options.remove("folder.depth_limit");
        }

        if self.case_insensitive {
            options.insert("folder.case_insensitive".to_string(), "true".to_string());
        } else {
            options.remove("folder.case_insensitive");
        }

        if let Some(ref inc) = self.include {
            if !inc.is_empty() {
                options.insert("folder.include".to_string(), inc.clone());
            } else {
                options.remove("folder.include");
            }
        } else {
            options.remove("folder.include");
        }

        if let Some(ref exc) = self.exclude {
            if !exc.is_empty() {
                options.insert("folder.exclude".to_string(), exc.clone());
            } else {
                options.remove("folder.exclude");
            }
        } else {
            options.remove("folder.exclude");
        }

        if let Some(ref qc) = self.quick_compare {
            if !qc.is_empty() {
                options.insert("folder.quick_compare".to_string(), qc.clone());
            } else {
                options.remove("folder.quick_compare");
            }
        } else {
            options.remove("folder.quick_compare");
        }

        if self.follow_symlinks {
            options.insert("folder.follow_symlinks".to_string(), "true".to_string());
        } else {
            options.remove("folder.follow_symlinks");
        }

        if self.archive_traversal {
            options.insert("folder.archive_traversal".to_string(), "true".to_string());
        } else {
            options.remove("folder.archive_traversal");
        }
    }
}

/// Options tuning folder synchronisation behaviour for a session tab.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SyncCompareOptions {
    /// High-level synchronization mode (`"update-right"`, `"update-left"`, `"both"`, `"mirror"`).
    pub mode: Option<String>,
    /// Explicit directional sync rule (`"left_to_right"`, `"right_to_left"`, `"both"`).
    pub direction: Option<String>,
    /// Delete one-sided destination entries not present on source.
    pub prune: bool,
    /// Delete empty directories after sync operations.
    pub delete_empty_folders: bool,
}

impl SyncCompareOptions {
    /// Extracts sync compare options from a session tab options map.
    pub fn from_map(options: &HashMap<String, String>) -> Self {
        let mode = options.get("sync.mode").filter(|v| !v.is_empty()).cloned();

        let direction = options
            .get("sync.direction")
            .filter(|v| !v.is_empty())
            .cloned();

        let prune = options.get("sync.prune").is_some_and(|v| v == "true");

        let delete_empty_folders = options
            .get("sync.delete_empty_folders")
            .is_some_and(|v| v == "true");

        Self {
            mode,
            direction,
            prune,
            delete_empty_folders,
        }
    }

    /// Serializes sync options into key-value pairs in the options map.
    pub fn write_to_map(&self, options: &mut HashMap<String, String>) {
        if let Some(ref m) = self.mode {
            if !m.is_empty() {
                options.insert("sync.mode".to_string(), m.clone());
            } else {
                options.remove("sync.mode");
            }
        } else {
            options.remove("sync.mode");
        }

        if let Some(ref d) = self.direction {
            if !d.is_empty() {
                options.insert("sync.direction".to_string(), d.clone());
            } else {
                options.remove("sync.direction");
            }
        } else {
            options.remove("sync.direction");
        }

        if self.prune {
            options.insert("sync.prune".to_string(), "true".to_string());
        } else {
            options.remove("sync.prune");
        }

        if self.delete_empty_folders {
            options.insert("sync.delete_empty_folders".to_string(), "true".to_string());
        } else {
            options.remove("sync.delete_empty_folders");
        }
    }
}

/// Options tuning text difference comparison behaviour for a session tab.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TextCompareOptions {
    /// Ignore letter casing differences.
    pub ignore_case: bool,
    /// Ignore whitespace differences completely.
    pub ignore_whitespace: bool,
    /// Ignore whitespace changes (leading, trailing, consecutive spaces).
    pub ignore_space_change: bool,
    /// Ignore completely blank lines.
    pub ignore_blank_lines: bool,
    /// Ignore differences in line endings (CRLF vs LF vs CR).
    pub ignore_line_endings: bool,
    /// Ignore differences inside code comments.
    pub ignore_comments: bool,
    /// Name of the grammar preset to apply.
    pub grammar_preset: Option<String>,
    /// Character encoding override for the left file.
    pub encoding_left: Option<String>,
    /// Character encoding override for the right file.
    pub encoding_right: Option<String>,
}

impl TextCompareOptions {
    /// Extracts text compare options from a session tab options map.
    pub fn from_map(options: &HashMap<String, String>) -> Self {
        let ignore_case = options.get("text.ignore_case").is_some_and(|v| v == "true");

        let ignore_whitespace = options
            .get("text.ignore_whitespace")
            .is_some_and(|v| v == "true");

        let ignore_space_change = options
            .get("text.ignore_space_change")
            .is_some_and(|v| v == "true");

        let ignore_blank_lines = options
            .get("text.ignore_blank_lines")
            .is_some_and(|v| v == "true");

        let ignore_line_endings = options
            .get("text.ignore_line_endings")
            .is_some_and(|v| v == "true");

        let ignore_comments = options
            .get("text.ignore_comments")
            .is_some_and(|v| v == "true");

        let grammar_preset = options
            .get("grammar.preset")
            .filter(|v| !v.is_empty())
            .cloned();

        let encoding_left = options
            .get("text.encoding_left")
            .filter(|v| !v.is_empty())
            .cloned();

        let encoding_right = options
            .get("text.encoding_right")
            .filter(|v| !v.is_empty())
            .cloned();

        Self {
            ignore_case,
            ignore_whitespace,
            ignore_space_change,
            ignore_blank_lines,
            ignore_line_endings,
            ignore_comments,
            grammar_preset,
            encoding_left,
            encoding_right,
        }
    }

    /// Serializes text options into key-value pairs in the options map.
    pub fn write_to_map(&self, options: &mut HashMap<String, String>) {
        if self.ignore_case {
            options.insert("text.ignore_case".to_string(), "true".to_string());
        } else {
            options.remove("text.ignore_case");
        }

        if self.ignore_whitespace {
            options.insert("text.ignore_whitespace".to_string(), "true".to_string());
        } else {
            options.remove("text.ignore_whitespace");
        }

        if self.ignore_space_change {
            options.insert("text.ignore_space_change".to_string(), "true".to_string());
        } else {
            options.remove("text.ignore_space_change");
        }

        if self.ignore_blank_lines {
            options.insert("text.ignore_blank_lines".to_string(), "true".to_string());
        } else {
            options.remove("text.ignore_blank_lines");
        }

        if self.ignore_line_endings {
            options.insert("text.ignore_line_endings".to_string(), "true".to_string());
        } else {
            options.remove("text.ignore_line_endings");
        }

        if self.ignore_comments {
            options.insert("text.ignore_comments".to_string(), "true".to_string());
        } else {
            options.remove("text.ignore_comments");
        }

        if let Some(ref gp) = self.grammar_preset {
            if !gp.is_empty() {
                options.insert("grammar.preset".to_string(), gp.clone());
            } else {
                options.remove("grammar.preset");
            }
        } else {
            options.remove("grammar.preset");
        }

        if let Some(ref el) = self.encoding_left {
            if !el.is_empty() {
                options.insert("text.encoding_left".to_string(), el.clone());
            } else {
                options.remove("text.encoding_left");
            }
        } else {
            options.remove("text.encoding_left");
        }

        if let Some(ref er) = self.encoding_right {
            if !er.is_empty() {
                options.insert("text.encoding_right".to_string(), er.clone());
            } else {
                options.remove("text.encoding_right");
            }
        } else {
            options.remove("text.encoding_right");
        }
    }
}

/// Options tuning tabular structured data comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCompareOptions {
    /// Delimiter character (e.g. `','`, `'\t'`, `';'`).
    pub delimiter: Option<char>,
    /// Whether the first line is treated as header names.
    pub header: bool,
    /// Zero-based column index designated as key column for row alignment.
    pub key_column: Option<usize>,
}

impl Default for TableCompareOptions {
    fn default() -> Self {
        Self {
            delimiter: Some(','),
            header: true,
            key_column: None,
        }
    }
}

impl TableCompareOptions {
    /// Extracts table options from options map.
    pub fn from_map(options: &HashMap<String, String>) -> Self {
        let delimiter = options
            .get("table.delimiter")
            .and_then(|v| v.chars().next());

        let header = options
            .get("table.header")
            .map(|v| v == "true")
            .unwrap_or(true);

        let key_column = options
            .get("table.key_column")
            .and_then(|v| v.parse::<usize>().ok());

        Self {
            delimiter,
            header,
            key_column,
        }
    }

    /// Serializes table options into key-value pairs in the options map.
    pub fn write_to_map(&self, options: &mut HashMap<String, String>) {
        if let Some(delim) = self.delimiter {
            options.insert("table.delimiter".to_string(), delim.to_string());
        } else {
            options.remove("table.delimiter");
        }

        if self.header {
            options.insert("table.header".to_string(), "true".to_string());
        } else {
            options.insert("table.header".to_string(), "false".to_string());
        }

        if let Some(col) = self.key_column {
            options.insert("table.key_column".to_string(), col.to_string());
        } else {
            options.remove("table.key_column");
        }
    }
}

/// Options tuning image and bitmap comparison.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PictureCompareOptions {
    /// Optional file path where difference heatmap image is generated.
    pub heatmap_path: Option<String>,
    /// Tolerance threshold for pixel RGB variance (0 to 255).
    pub tolerance: Option<u8>,
    /// Whether to ignore alpha channel transparency differences.
    pub ignore_alpha: bool,
}

impl PictureCompareOptions {
    /// Extracts picture options from options map.
    pub fn from_map(options: &HashMap<String, String>) -> Self {
        let heatmap_path = options
            .get("picture.heatmap")
            .filter(|v| !v.is_empty())
            .cloned();

        let tolerance = options
            .get("picture.tolerance")
            .and_then(|v| v.parse::<u8>().ok());

        let ignore_alpha = options
            .get("picture.ignore_alpha")
            .is_some_and(|v| v == "true");

        Self {
            heatmap_path,
            tolerance,
            ignore_alpha,
        }
    }

    /// Serializes picture options into key-value pairs in the options map.
    pub fn write_to_map(&self, options: &mut HashMap<String, String>) {
        if let Some(ref path) = self.heatmap_path {
            if !path.is_empty() {
                options.insert("picture.heatmap".to_string(), path.clone());
            } else {
                options.remove("picture.heatmap");
            }
        } else {
            options.remove("picture.heatmap");
        }

        if let Some(tol) = self.tolerance {
            options.insert("picture.tolerance".to_string(), tol.to_string());
        } else {
            options.remove("picture.tolerance");
        }

        if self.ignore_alpha {
            options.insert("picture.ignore_alpha".to_string(), "true".to_string());
        } else {
            options.remove("picture.ignore_alpha");
        }
    }
}

/// Options tuning hex binary comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HexCompareOptions {
    /// Number of bytes rendered per row (typically 16 or 32).
    pub bytes_per_row: Option<usize>,
    /// Grouping size in bytes (e.g. 1, 2, 4, 8).
    pub group_size: Option<usize>,
}

impl Default for HexCompareOptions {
    fn default() -> Self {
        Self {
            bytes_per_row: Some(16),
            group_size: Some(1),
        }
    }
}

impl HexCompareOptions {
    /// Extracts hex options from options map.
    pub fn from_map(options: &HashMap<String, String>) -> Self {
        let bytes_per_row = options
            .get("hex.bytes_per_row")
            .and_then(|v| v.parse::<usize>().ok());

        let group_size = options
            .get("hex.group_size")
            .and_then(|v| v.parse::<usize>().ok());

        Self {
            bytes_per_row,
            group_size,
        }
    }

    /// Serializes hex options into key-value pairs in the options map.
    pub fn write_to_map(&self, options: &mut HashMap<String, String>) {
        if let Some(bpr) = self.bytes_per_row {
            options.insert("hex.bytes_per_row".to_string(), bpr.to_string());
        } else {
            options.remove("hex.bytes_per_row");
        }

        if let Some(gs) = self.group_size {
            options.insert("hex.group_size".to_string(), gs.to_string());
        } else {
            options.remove("hex.group_size");
        }
    }
}

/// Options tuning audio comparison.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AudioCompareOptions {
    /// Expected sample rate in Hz.
    pub sample_rate: Option<u32>,
    /// Tolerance threshold for audio sample variance.
    pub tolerance: Option<f64>,
}

impl AudioCompareOptions {
    /// Extracts audio options from options map.
    pub fn from_map(options: &HashMap<String, String>) -> Self {
        let sample_rate = options
            .get("audio.sample_rate")
            .and_then(|v| v.parse::<u32>().ok());

        let tolerance = options
            .get("audio.tolerance")
            .and_then(|v| v.parse::<f64>().ok());

        Self {
            sample_rate,
            tolerance,
        }
    }

    /// Serializes audio options into key-value pairs in the options map.
    pub fn write_to_map(&self, options: &mut HashMap<String, String>) {
        if let Some(sr) = self.sample_rate {
            options.insert("audio.sample_rate".to_string(), sr.to_string());
        } else {
            options.remove("audio.sample_rate");
        }

        if let Some(tol) = self.tolerance {
            options.insert("audio.tolerance".to_string(), tol.to_string());
        } else {
            options.remove("audio.tolerance");
        }
    }
}

/// Options tuning 3-way text and folder merge comparison behaviour for a session tab.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MergeCompareOptions {
    /// Strategy or mode for conflict resolution ("manual", "take_left", "take_right", "take_ancestor").
    pub conflict_strategy: Option<String>,
    /// Whether to automatically merge non-conflicting changes.
    pub auto_merge: bool,
    /// Alignment algorithm used for 3-way diff (e.g. "myers", "patience").
    pub alignment: Option<String>,
}

impl MergeCompareOptions {
    /// Extracts merge options from options map.
    pub fn from_map(options: &HashMap<String, String>) -> Self {
        let conflict_strategy = options
            .get("merge.conflict_strategy")
            .filter(|v| !v.is_empty())
            .cloned();

        let auto_merge = options.get("merge.auto_merge").is_some_and(|v| v == "true");

        let alignment = options
            .get("merge.alignment")
            .filter(|v| !v.is_empty())
            .cloned();

        Self {
            conflict_strategy,
            auto_merge,
            alignment,
        }
    }

    /// Serializes merge options into key-value pairs in the options map.
    pub fn write_to_map(&self, options: &mut HashMap<String, String>) {
        if let Some(ref cs) = self.conflict_strategy {
            if !cs.is_empty() {
                options.insert("merge.conflict_strategy".to_string(), cs.clone());
            } else {
                options.remove("merge.conflict_strategy");
            }
        } else {
            options.remove("merge.conflict_strategy");
        }

        if self.auto_merge {
            options.insert("merge.auto_merge".to_string(), "true".to_string());
        } else {
            options.remove("merge.auto_merge");
        }

        if let Some(ref alg) = self.alignment {
            if !alg.is_empty() {
                options.insert("merge.alignment".to_string(), alg.clone());
            } else {
                options.remove("merge.alignment");
            }
        } else {
            options.remove("merge.alignment");
        }
    }
}
