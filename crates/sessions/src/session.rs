use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::SessionError;
pub use crate::options::{
    AudioCompareOptions, FolderCompareOptions, HexCompareOptions, MergeCompareOptions,
    PictureCompareOptions, SyncCompareOptions, TableCompareOptions, TextCompareOptions,
};

/// The current schema version for SessionDocument.
pub const CURRENT_SESSION_SCHEMA_VERSION: u32 = 1;

/// Alias for [`SessionDocument`] representing a versioned session configuration.
pub type SessionConfig = SessionDocument;

/// Represents a single tab within a compare session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTab {
    /// Unique identifier for the tab.
    pub id: String,
    /// Display title for the tab.
    pub title: String,
    /// Type of compare session (e.g., "text", "folder", "table", "hex", "picture", "audio", "sync", "merge", "folder_merge").
    #[serde(default = "default_tab_type")]
    pub session_type: String,
    /// Optional left path or URI.
    #[serde(default)]
    pub left_path: Option<String>,
    /// Optional right path or URI.
    #[serde(default)]
    pub right_path: Option<String>,
    /// Optional base/ancestor path for 3-way merge sessions.
    #[serde(default)]
    pub base_path: Option<String>,
    /// Optional output target path for merge sessions.
    #[serde(default)]
    pub output_path: Option<String>,
    /// Whether this tab is marked read-only.
    #[serde(default)]
    pub read_only: bool,
    /// Whether this tab is locked against modification.
    #[serde(default)]
    pub locked: bool,
    /// Additional session-specific options.
    #[serde(default)]
    pub options: HashMap<String, String>,
}

fn default_tab_type() -> String {
    "text".to_string()
}

impl SessionTab {
    /// Creates a new tab with default settings.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            session_type: default_tab_type(),
            left_path: None,
            right_path: None,
            base_path: None,
            output_path: None,
            read_only: false,
            locked: false,
            options: HashMap::new(),
        }
    }

    /// Creates a new text comparison tab.
    pub fn new_text(
        id: impl Into<String>,
        title: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        Self::new(id, title)
            .with_session_type("text")
            .with_paths(left, right)
    }

    /// Creates a new folder comparison tab.
    pub fn new_folder(
        id: impl Into<String>,
        title: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        Self::new(id, title)
            .with_session_type("folder")
            .with_paths(left, right)
    }

    /// Creates a new table comparison tab.
    pub fn new_table(
        id: impl Into<String>,
        title: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        Self::new(id, title)
            .with_session_type("table")
            .with_paths(left, right)
    }

    /// Creates a new picture comparison tab.
    pub fn new_picture(
        id: impl Into<String>,
        title: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        Self::new(id, title)
            .with_session_type("picture")
            .with_paths(left, right)
    }

    /// Creates a new hex binary comparison tab.
    pub fn new_hex(
        id: impl Into<String>,
        title: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        Self::new(id, title)
            .with_session_type("hex")
            .with_paths(left, right)
    }

    /// Creates a new audio comparison tab.
    pub fn new_audio(
        id: impl Into<String>,
        title: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        Self::new(id, title)
            .with_session_type("audio")
            .with_paths(left, right)
    }

    /// Creates a new sync comparison tab.
    pub fn new_sync(
        id: impl Into<String>,
        title: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        Self::new(id, title)
            .with_session_type("sync")
            .with_paths(left, right)
    }

    /// Creates a new 3-way text merge comparison tab.
    pub fn new_merge(
        id: impl Into<String>,
        title: impl Into<String>,
        base: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Self::new(id, title)
            .with_session_type("merge")
            .with_merge_paths(base, left, right, output)
    }

    /// Creates a new 3-way folder merge comparison tab.
    pub fn new_folder_merge(
        id: impl Into<String>,
        title: impl Into<String>,
        base: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Self::new(id, title)
            .with_session_type("folder_merge")
            .with_merge_paths(base, left, right, output)
    }

    /// Builder method to set left and right paths.
    pub fn with_paths(mut self, left: impl Into<String>, right: impl Into<String>) -> Self {
        self.left_path = Some(left.into());
        self.right_path = Some(right.into());
        self
    }

    /// Builder method to set 3-way merge paths (base, left, right, output).
    pub fn with_merge_paths(
        mut self,
        base: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        self.base_path = Some(base.into());
        self.left_path = Some(left.into());
        self.right_path = Some(right.into());
        self.output_path = Some(output.into());
        self
    }

    /// Builder method to set the base/ancestor path.
    pub fn with_base_path(mut self, base: impl Into<String>) -> Self {
        self.base_path = Some(base.into());
        self
    }

    /// Builder method to set the output destination path.
    pub fn with_output_path(mut self, output: impl Into<String>) -> Self {
        self.output_path = Some(output.into());
        self
    }

    /// Builder method to set the session type.
    pub fn with_session_type(mut self, session_type: impl Into<String>) -> Self {
        self.session_type = session_type.into();
        self
    }

    /// Builder method to set whether this tab is locked.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Builder method to set whether this tab is read-only.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    // --- Typed Options Views ---

    /// Extracts structured folder compare options.
    pub fn folder_options(&self) -> FolderCompareOptions {
        FolderCompareOptions::from_map(&self.options)
    }

    /// Sets structured folder compare options.
    pub fn set_folder_options(&mut self, opts: &FolderCompareOptions) {
        opts.write_to_map(&mut self.options);
    }

    /// Builder method to configure structured folder compare options.
    pub fn with_folder_options(mut self, opts: FolderCompareOptions) -> Self {
        opts.write_to_map(&mut self.options);
        self
    }

    /// Extracts structured folder sync options.
    pub fn sync_options(&self) -> SyncCompareOptions {
        SyncCompareOptions::from_map(&self.options)
    }

    /// Sets structured folder sync options.
    pub fn set_sync_options(&mut self, opts: &SyncCompareOptions) {
        opts.write_to_map(&mut self.options);
    }

    /// Builder method to configure structured folder sync options.
    pub fn with_sync_options(mut self, opts: SyncCompareOptions) -> Self {
        opts.write_to_map(&mut self.options);
        self
    }

    /// Extracts structured text compare options.
    pub fn text_options(&self) -> TextCompareOptions {
        TextCompareOptions::from_map(&self.options)
    }

    /// Sets structured text compare options.
    pub fn set_text_options(&mut self, opts: &TextCompareOptions) {
        opts.write_to_map(&mut self.options);
    }

    /// Builder method to configure structured text compare options.
    pub fn with_text_options(mut self, opts: TextCompareOptions) -> Self {
        opts.write_to_map(&mut self.options);
        self
    }

    /// Extracts structured table compare options.
    pub fn table_options(&self) -> TableCompareOptions {
        TableCompareOptions::from_map(&self.options)
    }

    /// Sets structured table compare options.
    pub fn set_table_options(&mut self, opts: &TableCompareOptions) {
        opts.write_to_map(&mut self.options);
    }

    /// Builder method to configure structured table compare options.
    pub fn with_table_options(mut self, opts: TableCompareOptions) -> Self {
        opts.write_to_map(&mut self.options);
        self
    }

    /// Extracts structured picture compare options.
    pub fn picture_options(&self) -> PictureCompareOptions {
        PictureCompareOptions::from_map(&self.options)
    }

    /// Sets structured picture compare options.
    pub fn set_picture_options(&mut self, opts: &PictureCompareOptions) {
        opts.write_to_map(&mut self.options);
    }

    /// Builder method to configure structured picture compare options.
    pub fn with_picture_options(mut self, opts: PictureCompareOptions) -> Self {
        opts.write_to_map(&mut self.options);
        self
    }

    /// Extracts structured hex compare options.
    pub fn hex_options(&self) -> HexCompareOptions {
        HexCompareOptions::from_map(&self.options)
    }

    /// Sets structured hex compare options.
    pub fn set_hex_options(&mut self, opts: &HexCompareOptions) {
        opts.write_to_map(&mut self.options);
    }

    /// Builder method to configure structured hex compare options.
    pub fn with_hex_options(mut self, opts: HexCompareOptions) -> Self {
        opts.write_to_map(&mut self.options);
        self
    }

    /// Extracts structured audio compare options.
    pub fn audio_options(&self) -> AudioCompareOptions {
        AudioCompareOptions::from_map(&self.options)
    }

    /// Sets structured audio compare options.
    pub fn set_audio_options(&mut self, opts: &AudioCompareOptions) {
        opts.write_to_map(&mut self.options);
    }

    /// Builder method to configure structured audio compare options.
    pub fn with_audio_options(mut self, opts: AudioCompareOptions) -> Self {
        opts.write_to_map(&mut self.options);
        self
    }

    // --- Folder & Sync individual properties ---

    /// Builder method to record the folder-sync mode for this tab
    /// (`"update-right"`, `"update-left"`, `"both"`, `"mirror"`).
    pub fn with_sync_mode(mut self, mode: impl Into<String>) -> Self {
        self.set_sync_mode(mode);
        self
    }

    /// Returns the recorded folder-sync mode, if any.
    pub fn sync_mode(&self) -> Option<&str> {
        self.options.get("sync.mode").map(String::as_str)
    }

    /// Records the folder-sync mode for this tab.
    pub fn set_sync_mode(&mut self, mode: impl Into<String>) {
        self.options.insert("sync.mode".to_string(), mode.into());
    }

    /// Returns the recorded folder depth limit, if present and valid.
    pub fn folder_depth_limit(&self) -> Option<usize> {
        self.options
            .get("folder.depth_limit")
            .and_then(|raw| raw.parse::<usize>().ok())
    }

    /// Records the folder depth limit for this tab.
    pub fn set_folder_depth_limit(&mut self, limit: usize) {
        self.options
            .insert("folder.depth_limit".to_string(), limit.to_string());
    }

    /// Returns whether content compare was recorded for this tab.
    pub fn folder_content_compare(&self) -> bool {
        self.options
            .get("folder.content_compare")
            .is_some_and(|raw| raw == "true")
    }

    /// Records whether to compare folder file bytes for this tab.
    pub fn set_folder_content_compare(&mut self, value: bool) {
        self.options
            .insert("folder.content_compare".to_string(), value.to_string());
    }

    /// Returns whether case-insensitive name matching was recorded.
    pub fn folder_case_insensitive(&self) -> bool {
        self.options
            .get("folder.case_insensitive")
            .is_some_and(|raw| raw == "true")
    }

    /// Records case-insensitive name matching for this tab.
    pub fn set_folder_case_insensitive(&mut self, value: bool) {
        self.options
            .insert("folder.case_insensitive".to_string(), value.to_string());
    }

    /// Returns the recorded include glob, if any.
    pub fn folder_include(&self) -> Option<&str> {
        self.options.get("folder.include").map(String::as_str)
    }

    /// Records the include glob; an empty value clears it.
    pub fn set_folder_include(&mut self, pattern: impl Into<String>) {
        let pattern = pattern.into();
        if pattern.is_empty() {
            self.options.remove("folder.include");
        } else {
            self.options.insert("folder.include".to_string(), pattern);
        }
    }

    /// Returns the recorded exclude glob, if any.
    pub fn folder_exclude(&self) -> Option<&str> {
        self.options.get("folder.exclude").map(String::as_str)
    }

    /// Records the exclude glob; an empty value clears it.
    pub fn set_folder_exclude(&mut self, pattern: impl Into<String>) {
        let pattern = pattern.into();
        if pattern.is_empty() {
            self.options.remove("folder.exclude");
        } else {
            self.options.insert("folder.exclude".to_string(), pattern);
        }
    }

    /// Returns whether symlinks should be followed in folder comparison.
    pub fn folder_follow_symlinks(&self) -> bool {
        self.options
            .get("folder.follow_symlinks")
            .is_some_and(|raw| raw == "true")
    }

    /// Records whether to follow symlinks in folder comparison.
    pub fn set_folder_follow_symlinks(&mut self, follow: bool) {
        self.options
            .insert("folder.follow_symlinks".to_string(), follow.to_string());
    }

    /// Returns whether archive traversal is enabled.
    pub fn folder_archive_traversal(&self) -> bool {
        self.options
            .get("folder.archive_traversal")
            .is_some_and(|raw| raw == "true")
    }

    /// Records whether archive traversal is enabled.
    pub fn set_folder_archive_traversal(&mut self, traverse: bool) {
        self.options
            .insert("folder.archive_traversal".to_string(), traverse.to_string());
    }

    /// Returns the recorded quick compare algorithm mode, if any.
    pub fn folder_quick_compare(&self) -> Option<&str> {
        self.options.get("folder.quick_compare").map(String::as_str)
    }

    /// Records the quick compare algorithm mode (e.g. "size", "timestamp", "crc").
    pub fn set_folder_quick_compare(&mut self, mode: impl Into<String>) {
        let mode = mode.into();
        if mode.is_empty() {
            self.options.remove("folder.quick_compare");
        } else {
            self.options
                .insert("folder.quick_compare".to_string(), mode);
        }
    }

    /// Returns the recorded directional sync rule, if any.
    pub fn sync_direction(&self) -> Option<&str> {
        self.options.get("sync.direction").map(String::as_str)
    }

    /// Records the directional sync rule (`"left_to_right"`, `"right_to_left"`, `"both"`).
    pub fn set_sync_direction(&mut self, dir: impl Into<String>) {
        self.options
            .insert("sync.direction".to_string(), dir.into());
    }

    /// Returns whether sync prunes destination entries not present on source.
    pub fn sync_prune(&self) -> bool {
        self.options
            .get("sync.prune")
            .is_some_and(|raw| raw == "true")
    }

    /// Records whether sync prunes destination entries.
    pub fn set_sync_prune(&mut self, prune: bool) {
        self.options
            .insert("sync.prune".to_string(), prune.to_string());
    }

    // --- Text & Grammar properties ---

    /// Returns whether case differences are ignored for text comparison.
    pub fn text_ignore_case(&self) -> bool {
        self.options
            .get("text.ignore_case")
            .is_some_and(|raw| raw == "true")
    }

    /// Records whether to ignore letter case differences.
    pub fn set_text_ignore_case(&mut self, ignore: bool) {
        self.options
            .insert("text.ignore_case".to_string(), ignore.to_string());
    }

    /// Returns whether whitespace differences are completely ignored.
    pub fn text_ignore_whitespace(&self) -> bool {
        self.options
            .get("text.ignore_whitespace")
            .is_some_and(|raw| raw == "true")
    }

    /// Records whether to ignore whitespace differences.
    pub fn set_text_ignore_whitespace(&mut self, ignore: bool) {
        self.options
            .insert("text.ignore_whitespace".to_string(), ignore.to_string());
    }

    /// Returns whether whitespace changes (leading, trailing, consecutive) are ignored.
    pub fn text_ignore_space_change(&self) -> bool {
        self.options
            .get("text.ignore_space_change")
            .is_some_and(|raw| raw == "true")
    }

    /// Records whether to ignore whitespace changes.
    pub fn set_text_ignore_space_change(&mut self, ignore: bool) {
        self.options
            .insert("text.ignore_space_change".to_string(), ignore.to_string());
    }

    /// Returns whether blank lines are ignored.
    pub fn text_ignore_blank_lines(&self) -> bool {
        self.options
            .get("text.ignore_blank_lines")
            .is_some_and(|raw| raw == "true")
    }

    /// Records whether to ignore blank lines.
    pub fn set_text_ignore_blank_lines(&mut self, ignore: bool) {
        self.options
            .insert("text.ignore_blank_lines".to_string(), ignore.to_string());
    }

    /// Returns whether line endings (CRLF vs LF vs CR) are ignored.
    pub fn text_ignore_line_endings(&self) -> bool {
        self.options
            .get("text.ignore_line_endings")
            .is_some_and(|raw| raw == "true")
    }

    /// Records whether to ignore line endings.
    pub fn set_text_ignore_line_endings(&mut self, ignore: bool) {
        self.options
            .insert("text.ignore_line_endings".to_string(), ignore.to_string());
    }

    /// Returns whether comment differences are ignored.
    pub fn text_ignore_comments(&self) -> bool {
        self.options
            .get("text.ignore_comments")
            .is_some_and(|raw| raw == "true")
    }

    /// Records whether to ignore comment differences.
    pub fn set_text_ignore_comments(&mut self, ignore: bool) {
        self.options
            .insert("text.ignore_comments".to_string(), ignore.to_string());
    }

    /// Returns the applied grammar preset name, if configured.
    pub fn grammar_preset(&self) -> Option<&str> {
        self.options.get("grammar.preset").map(String::as_str)
    }

    /// Records the grammar preset name for this tab.
    pub fn set_grammar_preset(&mut self, preset: impl Into<String>) {
        let preset = preset.into();
        if preset.is_empty() {
            self.options.remove("grammar.preset");
        } else {
            self.options.insert("grammar.preset".to_string(), preset);
        }
    }

    /// Builder method to set grammar preset name.
    pub fn with_grammar_preset(mut self, preset: impl Into<String>) -> Self {
        self.set_grammar_preset(preset);
        self
    }

    // --- Table properties ---

    /// Returns the table delimiter character, if set.
    pub fn table_delimiter(&self) -> Option<char> {
        self.options
            .get("table.delimiter")
            .and_then(|v| v.chars().next())
    }

    /// Records the table delimiter character.
    pub fn set_table_delimiter(&mut self, delimiter: char) {
        self.options
            .insert("table.delimiter".to_string(), delimiter.to_string());
    }

    /// Returns whether the table has a header line (defaults to true).
    pub fn table_header(&self) -> bool {
        self.options
            .get("table.header")
            .map(|v| v == "true")
            .unwrap_or(true)
    }

    /// Records whether the table has a header line.
    pub fn set_table_header(&mut self, header: bool) {
        self.options
            .insert("table.header".to_string(), header.to_string());
    }

    // --- Picture properties ---

    /// Returns the heatmap output path, if configured.
    pub fn picture_heatmap_path(&self) -> Option<&str> {
        self.options.get("picture.heatmap").map(String::as_str)
    }

    /// Records the heatmap output path.
    pub fn set_picture_heatmap_path(&mut self, path: impl Into<String>) {
        let path = path.into();
        if path.is_empty() {
            self.options.remove("picture.heatmap");
        } else {
            self.options.insert("picture.heatmap".to_string(), path);
        }
    }

    /// Returns the picture pixel RGB tolerance, if configured.
    pub fn picture_tolerance(&self) -> Option<u8> {
        self.options
            .get("picture.tolerance")
            .and_then(|v| v.parse::<u8>().ok())
    }

    /// Records the picture pixel RGB tolerance threshold.
    pub fn set_picture_tolerance(&mut self, tolerance: u8) {
        self.options
            .insert("picture.tolerance".to_string(), tolerance.to_string());
    }

    // --- Merge properties ---

    /// Extracts structured 3-way merge options.
    pub fn merge_options(&self) -> MergeCompareOptions {
        MergeCompareOptions::from_map(&self.options)
    }

    /// Sets structured 3-way merge options.
    pub fn set_merge_options(&mut self, opts: &MergeCompareOptions) {
        opts.write_to_map(&mut self.options);
    }

    /// Builder method to configure structured 3-way merge options.
    pub fn with_merge_options(mut self, opts: MergeCompareOptions) -> Self {
        opts.write_to_map(&mut self.options);
        self
    }

    /// Returns the recorded conflict resolution strategy, if any.
    pub fn merge_conflict_strategy(&self) -> Option<&str> {
        self.options
            .get("merge.conflict_strategy")
            .map(String::as_str)
    }

    /// Records the conflict resolution strategy for this tab.
    pub fn set_merge_conflict_strategy(&mut self, strategy: impl Into<String>) {
        self.options
            .insert("merge.conflict_strategy".to_string(), strategy.into());
    }

    /// Returns whether auto-merge is recorded for this tab.
    pub fn merge_auto_merge(&self) -> bool {
        self.options
            .get("merge.auto_merge")
            .is_some_and(|raw| raw == "true")
    }

    /// Records whether to auto-merge non-conflicting changes for this tab.
    pub fn set_merge_auto_merge(&mut self, auto: bool) {
        self.options
            .insert("merge.auto_merge".to_string(), auto.to_string());
    }
}

/// A serialized session document containing one or more tabs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDocument {
    /// Schema version for forward/backward compatibility.
    pub schema_version: u32,
    /// Open tabs in this session.
    pub tabs: Vec<SessionTab>,
    /// Zero-based index of the active tab.
    #[serde(default)]
    pub active_tab_index: usize,
    /// Optional user-assigned session name.
    #[serde(default)]
    pub name: Option<String>,
    /// Whether this session is locked against accidental modification or overwrite.
    #[serde(default)]
    pub locked: bool,
}

impl Default for SessionDocument {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SESSION_SCHEMA_VERSION,
            tabs: Vec::new(),
            active_tab_index: 0,
            name: None,
            locked: false,
        }
    }
}

impl SessionDocument {
    /// Creates a new empty session document at the current schema version.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a session document containing the specified tabs.
    pub fn with_tabs(tabs: Vec<SessionTab>) -> Self {
        Self {
            schema_version: CURRENT_SESSION_SCHEMA_VERSION,
            tabs,
            active_tab_index: 0,
            name: None,
            locked: false,
        }
    }

    /// Returns the number of tabs in this session document.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Returns true if the session contains no tabs.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Returns a reference to the active tab, if one exists.
    pub fn active_tab(&self) -> Option<&SessionTab> {
        self.tabs.get(self.active_tab_index)
    }

    /// Returns a mutable reference to the active tab, if one exists.
    pub fn active_tab_mut(&mut self) -> Option<&mut SessionTab> {
        self.tabs.get_mut(self.active_tab_index)
    }

    /// Sets the active tab index if valid, returning true if successfully updated.
    pub fn set_active_tab_index(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_tab_index = index;
            true
        } else {
            false
        }
    }

    /// Adds a tab to the session, returning the index of the newly added tab.
    pub fn add_tab(&mut self, tab: SessionTab) -> usize {
        let index = self.tabs.len();
        self.tabs.push(tab);
        index
    }

    /// Removes a tab by index, adjusting active_tab_index if needed.
    pub fn remove_tab(&mut self, index: usize) -> Option<SessionTab> {
        if index < self.tabs.len() {
            let removed = self.tabs.remove(index);
            if self.active_tab_index >= self.tabs.len() && !self.tabs.is_empty() {
                self.active_tab_index = self.tabs.len().saturating_sub(1);
            }
            Some(removed)
        } else {
            None
        }
    }

    /// Finds a tab by unique identifier.
    pub fn find_tab(&self, id: &str) -> Option<&SessionTab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    /// Finds a tab by unique identifier (mutable).
    pub fn find_tab_mut(&mut self, id: &str) -> Option<&mut SessionTab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    /// Builder method to set session name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Returns true if the session document is locked against modification.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Sets whether the session document is locked.
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    /// Builder method to set whether the session is locked.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Serializes the session document to a compact JSON string.
    pub fn to_json(&self) -> Result<String, SessionError> {
        serde_json::to_string(self).map_err(SessionError::Json)
    }

    /// Serializes the session document to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, SessionError> {
        serde_json::to_string_pretty(self).map_err(SessionError::Json)
    }

    /// Deserializes and automatically migrates any supported JSON version to the current version.
    pub fn from_json(json_str: &str) -> Result<Self, SessionError> {
        migrate(json_str)
    }

    /// Saves the session document as pretty-printed JSON to the specified file path.
    ///
    /// If the session is locked (`self.locked == true`), returns an I/O PermissionDenied error.
    /// To bypass the lock check, use [`save_to_file_force`].
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SessionError> {
        if self.locked {
            return Err(SessionError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot overwrite locked session document; unlock or use save_to_file_force",
            )));
        }
        self.save_to_file_force(path)
    }

    /// Saves the session document unconditionally, bypassing any lock check.
    /// The write is atomic (sibling `.tmp` file + rename): a crash can never
    /// leave a torn half-written document behind.
    pub fn save_to_file_force<P: AsRef<Path>>(&self, path: P) -> Result<(), SessionError> {
        let json = self.to_json_pretty()?;
        crate::atomic_write(path.as_ref(), json.as_bytes())
    }

    /// Loads a session document from a file path, automatically migrating if needed.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, SessionError> {
        let content = std::fs::read_to_string(path).map_err(SessionError::Io)?;
        Self::from_json(&content)
    }

    /// Migrates an older JSON representation (such as v0) to the current schema version.
    pub fn migrate(old_json: &str) -> Result<Self, SessionError> {
        migrate(old_json)
    }
}

/// Clamps a dangling `active_tab_index` (past the end of `tabs`) back to 0,
/// so a hand-edited or migrated document never loads with no active tab.
fn clamp_active_index(doc: &mut SessionDocument) {
    if doc.active_tab_index >= doc.tabs.len() {
        doc.active_tab_index = 0;
    }
}

/// Migrates an arbitrary JSON string into a current [`SessionDocument`].
pub fn migrate(old_json: &str) -> Result<SessionDocument, SessionError> {
    if old_json.trim().is_empty() {
        return Err(SessionError::CorruptInput("Empty JSON input".to_string()));
    }

    let root: serde_json::Value = serde_json::from_str(old_json)?;

    match root {
        serde_json::Value::Object(ref map) => {
            let version_opt = map.get("schema_version");
            let version = match version_opt {
                None => 0,
                Some(serde_json::Value::Null) => 0,
                Some(serde_json::Value::Number(num)) => match num.as_u64() {
                    Some(v) => {
                        if v > u32::MAX as u64 {
                            return Err(SessionError::UnsupportedVersion {
                                found: u32::MAX,
                                max_supported: CURRENT_SESSION_SCHEMA_VERSION,
                            });
                        }
                        v as u32
                    }
                    None => {
                        return Err(SessionError::CorruptInput(
                            "schema_version must be an unsigned integer".to_string(),
                        ));
                    }
                },
                Some(_) => {
                    return Err(SessionError::CorruptInput(
                        "schema_version must be a numeric integer".to_string(),
                    ));
                }
            };

            if version > CURRENT_SESSION_SCHEMA_VERSION {
                return Err(SessionError::UnsupportedVersion {
                    found: version,
                    max_supported: CURRENT_SESSION_SCHEMA_VERSION,
                });
            }

            if version == CURRENT_SESSION_SCHEMA_VERSION {
                let mut doc: SessionDocument = serde_json::from_value(root)?;
                clamp_active_index(&mut doc);
                Ok(doc)
            } else {
                // v0 migration path
                migrate_v0_object(map)
            }
        }
        serde_json::Value::Array(items) => {
            // v0 array-of-tabs migration path
            migrate_v0_array(&items)
        }
        _ => Err(SessionError::CorruptInput(
            "Root JSON must be an object or array".to_string(),
        )),
    }
}

fn migrate_v0_object(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<SessionDocument, SessionError> {
    let raw_tabs = map
        .get("tabs")
        .or_else(|| map.get("sessions"))
        .or_else(|| map.get("items"))
        .or_else(|| map.get("tab_list"))
        .or_else(|| map.get("open_tabs"))
        .or_else(|| map.get("comparisons"));

    let tabs = match raw_tabs {
        Some(serde_json::Value::Array(items)) => parse_v0_tabs(items)?,
        Some(_) => {
            return Err(SessionError::CorruptInput(
                "v0 tabs field must be a JSON array".to_string(),
            ));
        }
        None => {
            // Check if the root object itself represents a single tab
            if map.contains_key("left_path")
                || map.contains_key("left")
                || map.contains_key("right_path")
                || map.contains_key("right")
                || map.contains_key("base_path")
                || map.contains_key("base")
                || map.contains_key("ancestor")
                || map.contains_key("output_path")
                || map.contains_key("output")
                || map.contains_key("session_type")
            {
                let single_val = serde_json::Value::Object(map.clone());
                parse_v0_tabs(&[single_val])?
            } else {
                Vec::new()
            }
        }
    };

    let active_tab_index = map
        .get("active_tab_index")
        .or_else(|| map.get("active"))
        .or_else(|| map.get("active_tab"))
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(0);

    let active_tab_index = if active_tab_index >= tabs.len() {
        0
    } else {
        active_tab_index
    };

    let name = map
        .get("name")
        .or_else(|| map.get("title"))
        .or_else(|| map.get("session_name"))
        .or_else(|| map.get("label"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let locked = map
        .get("locked")
        .or_else(|| map.get("is_locked"))
        .or_else(|| map.get("read_only_session"))
        .or_else(|| map.get("read_only"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(SessionDocument {
        schema_version: CURRENT_SESSION_SCHEMA_VERSION,
        tabs,
        active_tab_index,
        name,
        locked,
    })
}

fn migrate_v0_array(items: &[serde_json::Value]) -> Result<SessionDocument, SessionError> {
    let tabs = parse_v0_tabs(items)?;
    Ok(SessionDocument {
        schema_version: CURRENT_SESSION_SCHEMA_VERSION,
        tabs,
        active_tab_index: 0,
        name: None,
        locked: false,
    })
}

fn parse_v0_tabs(items: &[serde_json::Value]) -> Result<Vec<SessionTab>, SessionError> {
    let mut tabs = Vec::with_capacity(items.len());

    for (idx, item) in items.iter().enumerate() {
        match item {
            serde_json::Value::Object(tab_map) => {
                let id = tab_map
                    .get("id")
                    .or_else(|| tab_map.get("uuid"))
                    .or_else(|| tab_map.get("key"))
                    .or_else(|| tab_map.get("identifier"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("tab-{}", idx.saturating_add(1)));

                let title = tab_map
                    .get("title")
                    .or_else(|| tab_map.get("name"))
                    .or_else(|| tab_map.get("label"))
                    .or_else(|| tab_map.get("description"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "Untitled".to_string());

                let session_type = tab_map
                    .get("session_type")
                    .or_else(|| tab_map.get("type"))
                    .or_else(|| tab_map.get("kind"))
                    .or_else(|| tab_map.get("mode"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string)
                    .unwrap_or_else(default_tab_type);

                let left_path = tab_map
                    .get("left_path")
                    .or_else(|| tab_map.get("left"))
                    .or_else(|| tab_map.get("path_left"))
                    .or_else(|| tab_map.get("lhs"))
                    .or_else(|| tab_map.get("source"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                let right_path = tab_map
                    .get("right_path")
                    .or_else(|| tab_map.get("right"))
                    .or_else(|| tab_map.get("path_right"))
                    .or_else(|| tab_map.get("rhs"))
                    .or_else(|| tab_map.get("target"))
                    .or_else(|| tab_map.get("dest"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                let base_path = tab_map
                    .get("base_path")
                    .or_else(|| tab_map.get("base"))
                    .or_else(|| tab_map.get("ancestor"))
                    .or_else(|| tab_map.get("center"))
                    .or_else(|| tab_map.get("path_base"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                let output_path = tab_map
                    .get("output_path")
                    .or_else(|| tab_map.get("output"))
                    .or_else(|| tab_map.get("out"))
                    .or_else(|| tab_map.get("result"))
                    .or_else(|| tab_map.get("merged"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                let read_only = tab_map
                    .get("read_only")
                    .or_else(|| tab_map.get("readonly"))
                    .or_else(|| tab_map.get("ro"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let locked = tab_map
                    .get("locked")
                    .or_else(|| tab_map.get("is_locked"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let mut options = HashMap::new();
                let raw_options = tab_map
                    .get("options")
                    .or_else(|| tab_map.get("opts"))
                    .or_else(|| tab_map.get("settings"))
                    .or_else(|| tab_map.get("params"));

                if let Some(serde_json::Value::Object(opts_map)) = raw_options {
                    for (k, v) in opts_map {
                        if let Some(s) = v.as_str() {
                            options.insert(k.clone(), s.to_string());
                        } else if let Some(n) = v.as_i64() {
                            options.insert(k.clone(), n.to_string());
                        } else if let Some(b) = v.as_bool() {
                            options.insert(k.clone(), b.to_string());
                        }
                    }
                }

                // Check top-level legacy option keys that may have been placed directly on the tab
                check_and_insert_legacy_option(
                    tab_map,
                    &["sync_mode", "sync.mode"],
                    "sync.mode",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["folder_depth_limit", "depth_limit", "depth"],
                    "folder.depth_limit",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["content_compare", "folder.content_compare"],
                    "folder.content_compare",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["case_insensitive", "folder.case_insensitive"],
                    "folder.case_insensitive",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["include", "folder.include"],
                    "folder.include",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["exclude", "folder.exclude"],
                    "folder.exclude",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["quick_compare", "folder.quick_compare"],
                    "folder.quick_compare",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["follow_symlinks", "folder.follow_symlinks"],
                    "folder.follow_symlinks",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["archive_traversal", "folder.archive_traversal"],
                    "folder.archive_traversal",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["ignore_whitespace", "text.ignore_whitespace"],
                    "text.ignore_whitespace",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["ignore_case", "text.ignore_case"],
                    "text.ignore_case",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["ignore_space_change", "text.ignore_space_change"],
                    "text.ignore_space_change",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["ignore_blank_lines", "text.ignore_blank_lines"],
                    "text.ignore_blank_lines",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["ignore_line_endings", "text.ignore_line_endings"],
                    "text.ignore_line_endings",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["ignore_comments", "text.ignore_comments"],
                    "text.ignore_comments",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["grammar_preset", "grammar", "syntax", "grammar.preset"],
                    "grammar.preset",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["delimiter", "table.delimiter"],
                    "table.delimiter",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["header", "table.header"],
                    "table.header",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["direction", "sync.direction"],
                    "sync.direction",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["prune", "sync.prune"],
                    "sync.prune",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["heatmap", "heatmap_path", "picture.heatmap"],
                    "picture.heatmap",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["tolerance", "picture.tolerance"],
                    "picture.tolerance",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["conflict_strategy", "merge.conflict_strategy"],
                    "merge.conflict_strategy",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["auto_merge", "merge.auto_merge"],
                    "merge.auto_merge",
                    &mut options,
                );
                check_and_insert_legacy_option(
                    tab_map,
                    &["alignment", "merge.alignment"],
                    "merge.alignment",
                    &mut options,
                );

                tabs.push(SessionTab {
                    id,
                    title,
                    session_type,
                    left_path,
                    right_path,
                    base_path,
                    output_path,
                    read_only,
                    locked,
                    options,
                });
            }
            serde_json::Value::String(path_str) => {
                tabs.push(SessionTab {
                    id: format!("tab-{}", idx.saturating_add(1)),
                    title: path_str.clone(),
                    session_type: default_tab_type(),
                    left_path: Some(path_str.clone()),
                    right_path: None,
                    base_path: None,
                    output_path: None,
                    read_only: false,
                    locked: false,
                    options: HashMap::new(),
                });
            }
            serde_json::Value::Array(items) => {
                if items.len() >= 4 {
                    let base = items
                        .first()
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string);
                    let left = items
                        .get(1)
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string);
                    let right = items
                        .get(2)
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string);
                    let output = items
                        .get(3)
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string);
                    let title = match (&left, &right) {
                        (Some(l), Some(r)) => format!("{l} vs {r} (merge)"),
                        _ => format!("tab-{}", idx.saturating_add(1)),
                    };
                    tabs.push(SessionTab {
                        id: format!("tab-{}", idx.saturating_add(1)),
                        title,
                        session_type: "merge".to_string(),
                        left_path: left,
                        right_path: right,
                        base_path: base,
                        output_path: output,
                        read_only: false,
                        locked: false,
                        options: HashMap::new(),
                    });
                } else {
                    let left = items
                        .first()
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string);
                    let right = items
                        .get(1)
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string);
                    let title = match (&left, &right) {
                        (Some(l), Some(r)) => format!("{l} vs {r}"),
                        (Some(l), None) => l.clone(),
                        (None, Some(r)) => r.clone(),
                        (None, None) => format!("tab-{}", idx.saturating_add(1)),
                    };
                    tabs.push(SessionTab {
                        id: format!("tab-{}", idx.saturating_add(1)),
                        title,
                        session_type: default_tab_type(),
                        left_path: left,
                        right_path: right,
                        base_path: None,
                        output_path: None,
                        read_only: false,
                        locked: false,
                        options: HashMap::new(),
                    });
                }
            }
            _ => {
                return Err(SessionError::CorruptInput(
                    "Tab entry must be an object, string path, or path pair array".to_string(),
                ));
            }
        }
    }

    Ok(tabs)
}

fn check_and_insert_legacy_option(
    map: &serde_json::Map<String, serde_json::Value>,
    aliases: &[&str],
    canonical_key: &str,
    options: &mut HashMap<String, String>,
) {
    if options.contains_key(canonical_key) {
        return;
    }
    for alias in aliases {
        if let Some(val) = map.get(*alias) {
            if let Some(s) = val.as_str() {
                options.insert(canonical_key.to_string(), s.to_string());
                break;
            } else if let Some(n) = val.as_i64() {
                options.insert(canonical_key.to_string(), n.to_string());
                break;
            } else if let Some(b) = val.as_bool() {
                options.insert(canonical_key.to_string(), b.to_string());
                break;
            }
        }
    }
}

/// Migrates an arbitrary JSON string into a current [`SessionConfig`].
pub fn migrate_session_config(old_json: &str) -> Result<SessionConfig, SessionError> {
    migrate(old_json)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    #[test]
    fn test_session_tab_builder() {
        let tab = SessionTab::new("tab-1", "My Tab")
            .with_paths("/a", "/b")
            .with_session_type("hex")
            .with_locked(true)
            .with_read_only(true);
        assert_eq!(tab.id, "tab-1");
        assert_eq!(tab.title, "My Tab");
        assert_eq!(tab.session_type, "hex");
        assert_eq!(tab.left_path.as_deref(), Some("/a"));
        assert_eq!(tab.right_path.as_deref(), Some("/b"));
        assert!(tab.read_only);
        assert!(tab.locked);
    }

    #[test]
    fn test_session_document_roundtrip_unit() {
        let tab = SessionTab::new("t1", "Tab 1").with_paths("/left", "/right");
        let doc = SessionDocument::with_tabs(vec![tab])
            .with_name("My Session")
            .with_locked(true);
        assert!(doc.is_locked());
        assert_eq!(doc.tab_count(), 1);
        assert!(!doc.is_empty());
        assert_eq!(doc.name.as_deref(), Some("My Session"));
        let json = doc.to_json().expect("to_json");
        let parsed = SessionDocument::from_json(&json).expect("from_json");
        assert_eq!(doc, parsed);
        assert!(parsed.is_locked());
    }

    #[test]
    fn test_session_document_tab_management() {
        let mut doc = SessionDocument::new();
        assert!(doc.is_empty());
        let t1 = SessionTab::new_text("t1", "Text Tab", "/a.txt", "/b.txt");
        let t2 = SessionTab::new_folder("t2", "Folder Tab", "/dirA", "/dirB");
        assert_eq!(doc.add_tab(t1), 0);
        assert_eq!(doc.add_tab(t2), 1);
        assert_eq!(doc.tab_count(), 2);
        assert_eq!(doc.active_tab().map(|t| t.id.as_str()), Some("t1"));

        assert!(doc.set_active_tab_index(1));
        assert_eq!(doc.active_tab().map(|t| t.id.as_str()), Some("t2"));
        assert!(!doc.set_active_tab_index(99));

        assert!(doc.find_tab("t1").is_some());
        assert!(doc.find_tab("t99").is_none());

        let removed = doc.remove_tab(0).expect("remove t1");
        assert_eq!(removed.id, "t1");
        assert_eq!(doc.tab_count(), 1);
        assert_eq!(doc.active_tab().map(|t| t.id.as_str()), Some("t2"));
    }

    #[test]
    fn test_v0_migration_unit() {
        let v0 = r#"{"tabs": [{"id": "t", "title": "T", "locked": true}], "locked": true}"#;
        let doc = migrate(v0).expect("migrate");
        assert_eq!(doc.schema_version, CURRENT_SESSION_SCHEMA_VERSION);
        assert_eq!(doc.tabs.len(), 1);
        assert!(doc.locked);
        let tab0 = doc.tabs.first().expect("tab 0");
        assert!(tab0.locked);
    }

    #[test]
    fn test_future_version_unit() {
        let v2 = r#"{"schema_version": 2, "tabs": []}"#;
        assert!(migrate(v2).is_err());
    }

    #[test]
    fn test_corrupt_unit() {
        assert!(migrate("not json").is_err());
    }

    #[test]
    fn test_locked_save_guard() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_locked_guard_{}.json", std::process::id()));
        let mut doc = SessionDocument::new();
        doc.set_locked(true);
        assert!(doc.save_to_file(&path).is_err());
        assert!(doc.save_to_file_force(&path).is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_folder_sync_options_roundtrip() {
        let mut tab = SessionTab::new("f1", "Folder")
            .with_paths("/left", "/right")
            .with_session_type("folder")
            .with_sync_mode("mirror");
        assert_eq!(tab.sync_mode(), Some("mirror"));
        assert_eq!(tab.folder_depth_limit(), None);
        assert!(!tab.folder_content_compare());
        assert!(!tab.folder_case_insensitive());
        assert_eq!(tab.folder_include(), None);
        assert_eq!(tab.folder_exclude(), None);

        tab.set_folder_depth_limit(4);
        tab.set_folder_content_compare(true);
        tab.set_folder_case_insensitive(true);
        tab.set_folder_include("*.txt");
        tab.set_folder_exclude("*.log");
        assert_eq!(tab.folder_depth_limit(), Some(4));
        assert!(tab.folder_content_compare());
        assert!(tab.folder_case_insensitive());
        assert_eq!(tab.folder_include(), Some("*.txt"));
        assert_eq!(tab.folder_exclude(), Some("*.log"));

        // Clearing a glob removes the key; JSON keeps everything.
        tab.set_folder_include("");
        assert_eq!(tab.folder_include(), None);
        let doc = SessionDocument::with_tabs(vec![tab]);
        let json = doc.to_json().expect("to_json");
        let parsed = SessionDocument::from_json(&json).expect("from_json");
        assert_eq!(doc, parsed);
        let back = parsed.tabs.first().expect("one tab");
        assert_eq!(back.sync_mode(), Some("mirror"));
        assert_eq!(back.folder_depth_limit(), Some(4));
        assert!(back.folder_content_compare());
    }

    #[test]
    fn test_folder_options_corrupt_values_fall_back() {
        let mut tab = SessionTab::new("f2", "Folder");
        tab.options
            .insert("folder.depth_limit".to_string(), "deep".to_string());
        tab.options.insert(
            "folder.content_compare".to_string(),
            "yes-please".to_string(),
        );
        assert_eq!(tab.folder_depth_limit(), None);
        assert!(!tab.folder_content_compare());
        assert!(!tab.folder_case_insensitive());
    }

    #[test]
    fn test_text_and_table_compare_options_depth() {
        let mut tab = SessionTab::new_text("t1", "Diff", "a.rs", "b.rs");
        tab.set_text_ignore_case(true);
        tab.set_text_ignore_whitespace(true);
        tab.set_text_ignore_space_change(true);
        tab.set_text_ignore_blank_lines(true);
        tab.set_text_ignore_line_endings(true);
        tab.set_text_ignore_comments(true);
        tab.set_grammar_preset("rust");

        assert!(tab.text_ignore_case());
        assert!(tab.text_ignore_whitespace());
        assert!(tab.text_ignore_space_change());
        assert!(tab.text_ignore_blank_lines());
        assert!(tab.text_ignore_line_endings());
        assert!(tab.text_ignore_comments());
        assert_eq!(tab.grammar_preset(), Some("rust"));

        let text_opts = tab.text_options();
        assert!(text_opts.ignore_case);
        assert_eq!(text_opts.grammar_preset.as_deref(), Some("rust"));

        let mut tbl_tab = SessionTab::new_table("tbl1", "Table", "a.csv", "b.csv");
        tbl_tab.set_table_delimiter('\t');
        tbl_tab.set_table_header(false);
        assert_eq!(tbl_tab.table_delimiter(), Some('\t'));
        assert!(!tbl_tab.table_header());

        let tbl_opts = tbl_tab.table_options();
        assert_eq!(tbl_opts.delimiter, Some('\t'));
        assert!(!tbl_opts.header);
    }

    #[test]
    fn test_merge_tab_builder_and_options() {
        let mut tab = SessionTab::new_merge(
            "m1",
            "3-Way Merge Tab",
            "/base.rs",
            "/left.rs",
            "/right.rs",
            "/out.rs",
        )
        .with_locked(true);
        assert_eq!(tab.session_type, "merge");
        assert_eq!(tab.base_path.as_deref(), Some("/base.rs"));
        assert_eq!(tab.left_path.as_deref(), Some("/left.rs"));
        assert_eq!(tab.right_path.as_deref(), Some("/right.rs"));
        assert_eq!(tab.output_path.as_deref(), Some("/out.rs"));
        assert!(tab.locked);

        tab.set_merge_auto_merge(true);
        tab.set_merge_conflict_strategy("manual");
        assert!(tab.merge_auto_merge());
        assert_eq!(tab.merge_conflict_strategy(), Some("manual"));

        let opts = tab.merge_options();
        assert!(opts.auto_merge);
        assert_eq!(opts.conflict_strategy.as_deref(), Some("manual"));

        let doc = SessionDocument::with_tabs(vec![tab]);
        let json = doc.to_json_pretty().expect("to_json_pretty");
        let parsed = SessionDocument::from_json(&json).expect("from_json");
        assert_eq!(doc, parsed);
        let tab_back = parsed.tabs.first().expect("tab back");
        assert_eq!(tab_back.base_path.as_deref(), Some("/base.rs"));
        assert_eq!(tab_back.output_path.as_deref(), Some("/out.rs"));
        assert!(tab_back.merge_auto_merge());
    }

    #[test]
    fn test_v1_dangling_active_index_clamps_to_zero() {
        let empty = r#"{"schema_version": 1, "tabs": [], "active_tab_index": 7}"#;
        let doc = migrate(empty).expect("migrate empty");
        assert_eq!(doc.active_tab_index, 0);
        assert!(doc.active_tab().is_none());

        let one =
            r#"{"schema_version": 1, "tabs": [{"id": "t1", "title": "T"}], "active_tab_index": 5}"#;
        let doc = migrate(one).expect("migrate one tab");
        assert_eq!(doc.active_tab_index, 0);
        assert_eq!(doc.active_tab().expect("active").id, "t1");
    }

    #[test]
    fn test_v0_dangling_active_index_clamps_to_zero() {
        let v0 = r#"{"tabs": [{"id": "t1", "title": "T"}], "active": 99}"#;
        let doc = migrate(v0).expect("migrate v0");
        assert_eq!(doc.active_tab_index, 0);
        assert_eq!(doc.active_tab().expect("active").id, "t1");
    }

    #[test]
    fn test_atomic_save_leaves_no_tmp_sibling() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_sess_atomic_{}.json", std::process::id()));
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp_path = std::path::PathBuf::from(tmp);
        let _ = std::fs::remove_file(&tmp_path);

        let doc = SessionDocument::with_tabs(vec![SessionTab::new("t1", "T")]);
        doc.save_to_file_force(&path).expect("atomic save");
        assert!(!tmp_path.exists(), "no .tmp sibling may remain");
        let loaded = SessionDocument::load_from_file(&path).expect("reload");
        assert_eq!(doc, loaded);
        let _ = std::fs::remove_file(path);
    }
}
