//! App state and the engine-driving logic.
//!
//! One tab is active at a time; every tab keeps its own path strings
//! (so switching tabs preserves what was typed) and its own model.
//! The Compare button runs the engine call for the active tab through
//! the same `autocompare-compare` functions the CLI uses; results
//! replace that tab's model, failures become the red error panel and
//! clear the model. Nothing here panics.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};

use eframe::egui::{self, RichText, TextureHandle};

use autocompare_compare::{FolderOptions, SyncAction, TextOptions};
use autocompare_compare::{
    JobRunner, PictureOptions, SyncOptions, SyncPlan, TableOptions, TreeMergeAction, TreeMergePlan,
    compare_audio_files, compare_binary_files_aligned, compare_folders, compare_picture_files,
    compare_table_files, compare_table_files_by_key, compare_text_files, load_lines, plan_sync,
    plan_tree_merge,
};
use autocompare_diff::{AudioDiff, PictureDiff};

pub use crate::viewmodel::Tab;
use crate::viewmodel::{self, FolderRow, MergeSummary, TableModel};
use crate::views;
use crate::{home, tree, views_hex, views_table};

/// The two path strings of one tab, persisted across tab switches.
#[derive(Debug, Default, Clone)]
struct Paths {
    left: String,
    right: String,
}

/// The three path strings of a three-way tab.
#[derive(Debug, Default, Clone)]
struct TriplePaths {
    base: String,
    left: String,
    right: String,
}

enum CompareResult {
    Text(Result<viewmodel::TextRows, String>),
    Folder(Result<(Vec<FolderRow>, String), String>),
    Hex(Result<(Vec<String>, views_hex::HexCompareView), String>),
    Merge(Result<MergeSummary, String>),
    Table(Result<(TableModel, views_table::TableKeySummary), String>),
    Sync(Result<SyncChecklist, String>),
    Picture(
        Result<PictureCompareModel, String>,
        Option<(Vec<u8>, u32, u32)>,
    ),
    FolderMerge(Result<FolderMergeChecklist, String>),
    Audio(Result<AudioChecklist, String>),
}

/// The GUI's whole state.
pub struct App {
    pub tab: Tab,
    text: Paths,
    folder: Paths,
    hex: Paths,
    merge: TriplePaths,
    table: Paths,
    sync: Paths,
    picture: Paths,
    folder_merge: TriplePaths,
    audio: Paths,
    pub text_model: Option<viewmodel::TextRows>,
    pub folder_model: Option<(Vec<FolderRow>, String)>,
    pub hex_model: Option<Vec<String>>,
    pub hex_view: views_hex::HexCompareView,
    pub merge_model: Option<MergeSummary>,
    pub table_model: Option<TableModel>,
    pub table_view: views_table::TableKeyView,
    pub table_tolerance_text: String,
    pub sync_model: Option<SyncChecklist>,
    pub picture_model: Option<PictureCompareModel>,
    pub picture_view: PictureViewState,
    pub folder_merge_model: Option<FolderMergeChecklist>,
    pub audio_model: Option<AudioChecklist>,
    pub error: Option<String>,
    heatmap: Option<TextureHandle>,
    pub dark_mode: bool,
    pub folder_expanded: BTreeSet<String>,
    pub folder_session: tree::FolderSessionState,
    pub word_wrap: bool,
    pub show_line_numbers: bool,
    pub syntax_highlight: bool,
    pub status: String,
    rx: Option<Receiver<CompareResult>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            tab: Tab::Home,
            text: Paths::default(),
            folder: Paths::default(),
            hex: Paths::default(),
            merge: TriplePaths::default(),
            table: Paths::default(),
            sync: Paths::default(),
            picture: Paths::default(),
            folder_merge: TriplePaths::default(),
            audio: Paths::default(),
            text_model: None,
            folder_model: None,
            hex_model: None,
            hex_view: views_hex::HexCompareView::empty(
                views_hex::HexCompareMode::default(),
                views_hex::HEX_DEFAULT_MAX_HUNKS,
            ),
            merge_model: None,
            table_model: None,
            table_view: views_table::TableKeyView::new(),
            table_tolerance_text: String::new(),
            sync_model: None,
            picture_model: None,
            picture_view: PictureViewState::default(),
            folder_merge_model: None,
            audio_model: None,
            error: None,
            heatmap: None,
            dark_mode: true,
            folder_expanded: BTreeSet::new(),
            folder_session: tree::FolderSessionState::default(),
            word_wrap: false,
            show_line_numbers: true,
            syntax_highlight: true,
            status: String::from("ready: pick a session to begin"),
            rx: None,
        }
    }
}

impl App {
    /// Both path strings of the active two-path tab, trimmed.
    fn paths(&self) -> (String, String) {
        let (left, right) = match &self.tab {
            Tab::Text => (&self.text.left, &self.text.right),
            Tab::Folder => (&self.folder.left, &self.folder.right),
            Tab::Hex => (&self.hex.left, &self.hex.right),
            Tab::Table => (&self.table.left, &self.table.right),
            Tab::Sync => (&self.sync.left, &self.sync.right),
            Tab::Picture => (&self.picture.left, &self.picture.right),
            Tab::Audio => (&self.audio.left, &self.audio.right),
            Tab::Home | Tab::Merge | Tab::FolderMerge => return (String::new(), String::new()),
        };
        (left.trim().to_string(), right.trim().to_string())
    }

    /// All three path strings of the active three-way tab, trimmed.
    fn triple_paths(&self) -> (String, String, String) {
        let (base, left, right) = match &self.tab {
            Tab::Merge => (&self.merge.base, &self.merge.left, &self.merge.right),
            Tab::FolderMerge => (
                &self.folder_merge.base,
                &self.folder_merge.left,
                &self.folder_merge.right,
            ),
            _ => return (String::new(), String::new(), String::new()),
        };
        (
            base.trim().to_string(),
            left.trim().to_string(),
            right.trim().to_string(),
        )
    }

    /// Sets text paths (useful for testing and automation).
    pub fn set_text_paths(&mut self, left: impl Into<String>, right: impl Into<String>) {
        self.text.left = left.into();
        self.text.right = right.into();
    }

    /// Returns current text paths.
    pub fn text_paths(&self) -> (String, String) {
        (self.text.left.clone(), self.text.right.clone())
    }

    /// Sets folder paths.
    pub fn set_folder_paths(&mut self, left: impl Into<String>, right: impl Into<String>) {
        self.folder.left = left.into();
        self.folder.right = right.into();
    }

    /// Returns current folder paths.
    pub fn folder_paths(&self) -> (String, String) {
        (self.folder.left.clone(), self.folder.right.clone())
    }

    /// Sets merge triple paths.
    pub fn set_merge_paths(
        &mut self,
        base: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) {
        self.merge.base = base.into();
        self.merge.left = left.into();
        self.merge.right = right.into();
    }

    /// Returns current merge triple paths.
    pub fn merge_paths(&self) -> (String, String, String) {
        (
            self.merge.base.clone(),
            self.merge.left.clone(),
            self.merge.right.clone(),
        )
    }

    /// Sets the active tab.
    pub fn set_tab(&mut self, tab: Tab) {
        self.tab = tab;
    }

    /// Returns the active tab.
    pub fn current_tab(&self) -> Tab {
        self.tab
    }

    /// Human identity of the active session for tab/window titles.
    /// Format mirrors Beyond Compare: "left <--> right - Text Compare".
    /// Empty sides render as "(empty)" so the title never lies.
    pub fn session_identity(&self) -> String {
        let label = self.tab.title();
        match self.tab {
            Tab::Home => String::from("Home - autocompare"),
            Tab::Merge | Tab::FolderMerge => {
                let (base, left, right) = self.triple_paths();
                let b = if base.is_empty() {
                    "(empty)"
                } else {
                    base.as_str()
                };
                let l = if left.is_empty() {
                    "(empty)"
                } else {
                    left.as_str()
                };
                let r = if right.is_empty() {
                    "(empty)"
                } else {
                    right.as_str()
                };
                format!("{b} | {l} <--> {r} - {label}")
            }
            _ => {
                let (left, right) = self.paths();
                let l = if left.is_empty() {
                    "(empty)"
                } else {
                    left.as_str()
                };
                let r = if right.is_empty() {
                    "(empty)"
                } else {
                    right.as_str()
                };
                format!("{l} <--> {r} - {label}")
            }
        }
    }

    /// Viewport window title carrying the session identity.
    pub fn window_title(&self) -> String {
        format!("{} - autocompare", self.session_identity())
    }

    /// Swaps left/right sides of the active session.
    /// Returns true when a swap happened; Home never swaps.
    pub fn swap_sides(&mut self) -> bool {
        let swapped = match self.tab {
            Tab::Text => {
                std::mem::swap(&mut self.text.left, &mut self.text.right);
                self.text_model = None;
                true
            }
            Tab::Folder => {
                std::mem::swap(&mut self.folder.left, &mut self.folder.right);
                self.folder_model = None;
                true
            }
            Tab::Hex => {
                std::mem::swap(&mut self.hex.left, &mut self.hex.right);
                self.hex_model = None;
                let max_hunks = self.hex_view.max_hunks;
                self.hex_view = views_hex::HexCompareView::empty(self.hex_view.mode, max_hunks);
                true
            }
            Tab::Merge => {
                std::mem::swap(&mut self.merge.left, &mut self.merge.right);
                self.merge_model = None;
                true
            }
            Tab::Table => {
                std::mem::swap(&mut self.table.left, &mut self.table.right);
                self.table_model = None;
                self.table_view.summary = views_table::TableKeySummary::default();
                true
            }
            Tab::Sync => {
                std::mem::swap(&mut self.sync.left, &mut self.sync.right);
                self.sync_model = None;
                true
            }
            Tab::Picture => {
                std::mem::swap(&mut self.picture.left, &mut self.picture.right);
                self.picture_model = None;
                self.picture_view = PictureViewState::default();
                self.heatmap = None;
                true
            }
            Tab::FolderMerge => {
                std::mem::swap(&mut self.folder_merge.left, &mut self.folder_merge.right);
                self.folder_merge_model = None;
                true
            }
            Tab::Audio => {
                std::mem::swap(&mut self.audio.left, &mut self.audio.right);
                self.audio_model = None;
                true
            }
            Tab::Home => false,
        };
        if swapped {
            self.error = None;
            self.status = format!("swapped sides: {}", self.session_identity());
        } else {
            self.status = String::from("swap needs an active compare session");
        }
        swapped
    }

    /// Re-runs the compare for the active tab. Returns true when a job
    /// was started or validation already reported the problem.
    pub fn run_active_tab(&mut self) -> bool {
        let started = match self.tab {
            Tab::Home => {
                self.status = String::from("pick a session before comparing");
                return false;
            }
            Tab::Text => {
                self.run_text();
                true
            }
            Tab::Folder => {
                self.run_folder();
                true
            }
            Tab::Hex => {
                self.run_hex();
                true
            }
            Tab::Merge => {
                self.run_merge();
                true
            }
            Tab::Table => {
                self.run_table();
                true
            }
            Tab::Sync => {
                self.run_sync();
                true
            }
            Tab::Picture => {
                self.run_picture();
                true
            }
            Tab::FolderMerge => {
                self.run_folder_merge();
                true
            }
            Tab::Audio => {
                self.run_audio();
                true
            }
        };
        if self.rx.is_some() {
            self.status = format!("comparing: {}", self.session_identity());
        }
        started
    }

    /// View toggle setters. Each persists in app state and notes status.
    pub fn set_word_wrap(&mut self, on: bool) {
        self.word_wrap = on;
        self.status = if on {
            String::from("word wrap on: panes cap at 80 chars")
        } else {
            String::from("word wrap off: full-width panes")
        };
    }

    /// Line-number gutter toggle.
    pub fn set_line_numbers(&mut self, on: bool) {
        self.show_line_numbers = on;
        self.status = if on {
            String::from("line numbers on")
        } else {
            String::from("line numbers off")
        };
    }

    /// Syntax-highlight (inline diff accent) toggle.
    pub fn set_syntax_highlight(&mut self, on: bool) {
        self.syntax_highlight = on;
        self.status = if on {
            String::from("syntax highlighting on")
        } else {
            String::from("syntax highlighting off")
        };
    }

    /// Records user-visible status feedback.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
    }

    /// Sets the hex alignment mode, clearing stale hunks so the next
    /// Compare runs with the new strategy.
    pub fn set_hex_mode(&mut self, mode: views_hex::HexCompareMode) {
        if self.hex_view.mode == mode {
            return;
        }
        let max_hunks = self.hex_view.max_hunks;
        self.hex_view = views_hex::HexCompareView::empty(mode, max_hunks);
        self.hex_model = None;
        self.status = format!("hex mode: {}", mode.label());
    }

    /// Moves the hex hunk selection forward, wrapping like a find loop.
    pub fn hex_select_next(&mut self) {
        self.hex_view.select_next_hunk();
        self.status = format!("hex: {}", self.hex_view.status_line());
    }

    /// Moves the hex hunk selection backward, wrapping like a find loop.
    pub fn hex_select_prev(&mut self) {
        self.hex_view.select_prev_hunk();
        self.status = format!("hex: {}", self.hex_view.status_line());
    }

    /// Selects the hunk at the first difference, if any compare ran.
    pub fn hex_goto_first_diff(&mut self) -> bool {
        match self.hex_view.first_diff {
            Some(off) => self.hex_view.navigate_to_offset(off).is_some(),
            None => false,
        }
    }

    /// Toggles one table key column, clearing stale results.
    pub fn toggle_table_key_column(&mut self, column: usize) {
        self.table_view.toggle_key_column(column);
        self.clear_table_results();
        self.status = format!("table keys: {:?}", self.table_view.key_columns);
    }

    /// Clears every table key column back to positional compare.
    pub fn clear_table_keys(&mut self) {
        self.table_view.key_columns.clear();
        self.clear_table_results();
        self.status = String::from("table keys cleared: positional compare");
    }

    /// Sets the numeric tolerance, clearing stale results.
    pub fn set_table_tolerance(&mut self, tolerance: Option<f64>) {
        self.table_view.set_tolerance(tolerance);
        self.clear_table_results();
        self.status = match self.table_view.numeric_tolerance {
            Some(tol) => format!("table tolerance: {tol}"),
            None => String::from("table tolerance: exact text"),
        };
    }

    /// Sets case folding, clearing stale results.
    pub fn set_table_ignore_case(&mut self, on: bool) {
        self.table_view.ignore_case = on;
        self.clear_table_results();
        self.status = if on {
            String::from("table: ignore case on")
        } else {
            String::from("table: ignore case off")
        };
    }

    /// Sets space trimming, clearing stale results.
    pub fn set_table_trim_space(&mut self, on: bool) {
        self.table_view.trim_space = on;
        self.clear_table_results();
        self.status = if on {
            String::from("table: trim space on")
        } else {
            String::from("table: trim space off")
        };
    }

    /// Clears table results after an option change.
    fn clear_table_results(&mut self) {
        self.table_model = None;
        self.table_view.summary = views_table::TableKeySummary::default();
    }

    /// Honest over-cap view when an input exceeds the aligned hex cap.
    /// `None` when both sides fit (or cannot be measured), so callers
    /// only swap the view in for the genuinely skipped case.
    fn hex_over_cap_view(&self) -> Option<views_hex::HexCompareView> {
        let (left, right) = self.paths();
        if left.is_empty() || right.is_empty() {
            return None;
        }
        let left_len = std::fs::metadata(Path::new(&left))
            .map(|meta| meta.len())
            .unwrap_or(0);
        let right_len = std::fs::metadata(Path::new(&right))
            .map(|meta| meta.len())
            .unwrap_or(0);
        let left_fits = usize::try_from(left_len).unwrap_or(usize::MAX);
        let right_fits = usize::try_from(right_len).unwrap_or(usize::MAX);
        if views_hex::HexCompareView::is_over_cap(left_fits, right_fits) {
            Some(views_hex::HexCompareView {
                mode: self.hex_view.mode,
                max_hunks: self.hex_view.max_hunks,
                left_size: left_len,
                right_size: right_len,
                first_diff: None,
                hunks: Vec::new(),
                truncated: false,
                over_cap: true,
                selected: None,
            })
        } else {
            None
        }
    }

    /// Folder rows behind the active model, if compared.
    fn folder_rows(&self) -> Option<&Vec<FolderRow>> {
        self.folder_model.as_ref().map(|(rows, _)| rows)
    }

    /// Honest counts line for the Folder tab via [`tree::FolderCounts`].
    /// Empty model reports no results yet so the view never lies.
    pub fn folder_counts_summary(&self) -> String {
        match self.folder_rows() {
            Some(rows) => self.folder_session.counts(rows).summary(),
            None => String::from("folder: no results yet"),
        }
    }

    /// Toggles one directory in the folder tree. Keeps the legacy
    /// `folder_expanded` set mirrored so older callers stay correct.
    pub fn toggle_folder_dir(&mut self, dir: &str) {
        self.folder_session.toggle(dir);
        tree::toggle_expanded(&mut self.folder_expanded, dir);
    }

    /// Rows the folder tree paints: filtered, then hierarchical/flat.
    pub fn folder_visible_tree(&self) -> Vec<tree::FolderTreeNode> {
        match self.folder_rows() {
            Some(rows) => self.folder_session.visible_tree(rows),
            None => Vec::new(),
        }
    }

    /// Stages a double-clicked node and routes via
    /// [`home::resolve_compare_tab`]. Directory clicks stage nothing
    /// and keep the tab; file clicks stage the path and move to Text.
    /// Returns the tab the host should show.
    pub fn stage_folder_double_click(&mut self, node: &tree::FolderTreeNode) -> Tab {
        let staged_file = self.folder_session.stage_double_click(node);
        let target = home::resolve_compare_tab(&self.folder_session.staged);
        if staged_file {
            self.tab = target;
            if let Some(rel) = self.folder_session.staged.clone() {
                self.status = format!("staged {rel} for text compare");
            }
        }
        target
    }

    /// Switches the folder sync preview mode (drops Apply consent).
    pub fn set_folder_sync_mode(&mut self, mode: tree::SyncModeChoice) {
        self.folder_session.set_sync_mode(mode);
        self.status = format!("sync preview: {}", mode.label());
    }

    /// Sets explicit Apply consent for the folder sync preview.
    pub fn set_folder_consent(&mut self, consented: bool) {
        self.folder_session.consented = consented;
        self.status = if consented {
            String::from("sync consent on: Apply enabled when steps exist")
        } else {
            String::from("sync consent off: preview only")
        };
    }

    /// Dry-run preview for the current folder sync mode over compared rows.
    pub fn folder_sync_preview(&self) -> tree::SyncPreview {
        match self.folder_rows() {
            Some(rows) => self.folder_session.preview(rows),
            None => tree::SyncPreview::default(),
        }
    }

    /// Whether folder Apply may run now: consented and steps planned.
    pub fn folder_can_apply(&self) -> bool {
        self.folder_session
            .can_apply_now(&self.folder_sync_preview())
    }

    /// Engine sync options behind the current preview mode (same mapping
    /// `plan_sync` uses: Both = NewerWins/no-prune, Mirror =
    /// LeftToRight/prune). Shown as dry-run text; preview itself never
    /// touches the filesystem.
    pub fn folder_sync_options_label(&self) -> String {
        let opts = tree::sync_options_for(self.folder_session.sync_mode);
        format!("options: {opts:?} (plan_sync parity, dry-run only)")
    }

    /// Width panes paint at. Word wrap caps the model width at 80 so
    /// long lines stay readable without horizontal scrolling.
    pub fn effective_pane_chars(&self, pane_chars: usize) -> usize {
        if self.word_wrap {
            pane_chars.min(80)
        } else {
            pane_chars
        }
    }

    /// Returns a paint-ready copy of text rows with the current view
    /// prefs applied: line numbers hidden when off, inline accents
    /// cleared when syntax highlighting is off, pane width capped
    /// when word wrap is on. Pure and headless-testable.
    pub fn view_text_for_paint(&self, model: &viewmodel::TextRows) -> viewmodel::TextRows {
        let mut out = viewmodel::TextRows {
            rows: model.rows.clone(),
            truncated: model.truncated,
            coarse: model.coarse,
            pane_chars: self.effective_pane_chars(model.pane_chars),
            gutter_chars: if self.show_line_numbers {
                model.gutter_chars
            } else {
                1
            },
            selected: model.selected,
            filter: model.filter,
        };
        if !self.show_line_numbers {
            for row in &mut out.rows {
                row.left_no = None;
                row.right_no = None;
            }
        }
        if !self.syntax_highlight {
            for row in &mut out.rows {
                row.accent_a.clear();
                row.accent_b.clear();
            }
        }
        out
    }

    /// Honest one-line counts for the active tab plus last status.
    pub fn status_line(&self) -> String {
        let counts = match self.tab {
            Tab::Text => match self.text_model.as_ref() {
                Some(m) => {
                    let (eq, ch, del, ins) = text_kind_counts(m);
                    format!(
                        "{} rows: {eq} equal, {ch} changed, {del} deleted, {ins} inserted",
                        m.rows.len()
                    )
                }
                None => String::from("text: no results yet"),
            },
            Tab::Folder => match self.folder_model.as_ref() {
                Some((_, summary)) => {
                    if let Some(first) = summary.lines().next() {
                        format!("folder: {first}")
                    } else {
                        String::from("folder: no results yet")
                    }
                }
                None => String::from("folder: no results yet"),
            },
            Tab::Hex => {
                if self.hex_view.over_cap {
                    self.hex_view.status_line()
                } else {
                    match self.hex_model.as_ref() {
                        Some(_) => self.hex_view.status_line(),
                        None => String::from("hex: no results yet"),
                    }
                }
            }
            Tab::Merge => match self.merge_model.as_ref() {
                Some(m) => format!(
                    "merge: {} line(s){}",
                    m.lines.len(),
                    if m.conflicted { ", conflicted" } else { "" }
                ),
                None => String::from("merge: no results yet"),
            },
            Tab::Table => match self.table_model.as_ref() {
                Some(m) => format!(
                    "table: {} row(s), keyed {} | {}",
                    m.rows.len(),
                    self.table_view.summary.total(),
                    self.table_view.status_line()
                ),
                None => String::from("table: no results yet"),
            },
            Tab::Sync => match self.sync_model.as_ref() {
                Some(list) => format!("sync: {}", list.summary()),
                None => String::from("sync: no results yet"),
            },
            Tab::Picture => match self.picture_model.as_ref() {
                Some(model) => format!("picture: {}", model.status_summary()),
                None => String::from("picture: no results yet"),
            },
            Tab::FolderMerge => match self.folder_merge_model.as_ref() {
                Some(list) => format!("folder-merge: {}", list.summary()),
                None => String::from("folder-merge: no results yet"),
            },
            Tab::Audio => match self.audio_model.as_ref() {
                Some(list) => format!("audio: {}", list.summary()),
                None => String::from("audio: no results yet"),
            },
            Tab::Home => String::from("home: pick Text/Folder/Hex to begin"),
        };
        format!("{} | {}", counts, self.status)
    }

    /// Populates active tab paths when files or folders are dragged and dropped.
    pub fn apply_dropped_paths(&mut self, paths: &[String]) {
        match self.tab {
            Tab::Merge | Tab::FolderMerge => {
                let target = match self.tab {
                    Tab::Merge => &mut self.merge,
                    _ => &mut self.folder_merge,
                };
                if let Some(b) = paths.first() {
                    target.base = b.clone();
                }
                if let Some(l) = paths.get(1) {
                    target.left = l.clone();
                }
                if let Some(r) = paths.get(2) {
                    target.right = r.clone();
                }
            }
            Tab::Home => {
                if let Some(first) = paths.first() {
                    self.tab = Tab::Text;
                    self.text.left = first.clone();
                    if let Some(second) = paths.get(1) {
                        self.text.right = second.clone();
                    }
                }
            }
            _ => {
                let target = match self.tab {
                    Tab::Text => &mut self.text,
                    Tab::Folder => &mut self.folder,
                    Tab::Hex => &mut self.hex,
                    Tab::Table => &mut self.table,
                    Tab::Sync => &mut self.sync,
                    Tab::Picture => &mut self.picture,
                    Tab::Audio => &mut self.audio,
                    _ => return,
                };
                if let Some(first) = paths.first() {
                    if paths.len() >= 2 {
                        target.left = first.clone();
                        if let Some(second) = paths.get(1) {
                            target.right = second.clone();
                        }
                    } else if target.left.is_empty() {
                        target.left = first.clone();
                    } else {
                        target.right = first.clone();
                    }
                }
            }
        }
    }

    /// Rejects an empty side with a friendly message before the
    /// engine ever runs.
    fn require_paths(&mut self, left: &str, right: &str, noun: &str) -> bool {
        if left.is_empty() || right.is_empty() {
            self.error = Some(format!("type both {noun} paths first"));
            self.status = format!("needs both {noun} paths");
            return false;
        }
        true
    }

    /// Rejects an empty side of a three-way tab before the engine runs.
    fn require_triple(&mut self, base: &str, left: &str, right: &str, noun: &str) -> bool {
        if base.is_empty() || left.is_empty() || right.is_empty() {
            self.error = Some(format!("type the base and both {noun} paths first"));
            self.status = format!("needs base and both {noun} paths");
            return false;
        }
        true
    }

    fn run_text(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "file") {
            self.text_model = None;
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let _ = JobRunner::spawn(move |_cancel, _progress| {
            let res =
                compare_text_files(Path::new(&left), Path::new(&right), &TextOptions::default())
                    .map(|cmp| viewmodel::build_text_rows(&cmp))
                    .map_err(|err| err.to_string());
            let _ = tx.send(CompareResult::Text(res));
            Ok(())
        });
    }

    fn run_folder(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "folder") {
            self.folder_model = None;
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let _ = JobRunner::spawn(move |_cancel, _progress| {
            let res = compare_folders(
                Path::new(&left),
                Path::new(&right),
                &FolderOptions::default(),
            )
            .map(|cmp| viewmodel::build_folder_rows(&cmp, cmp.truncated))
            .map_err(|err| err.to_string());
            let _ = tx.send(CompareResult::Folder(res));
            Ok(())
        });
    }

    fn run_hex(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "file") {
            self.hex_model = None;
            let max_hunks = self.hex_view.max_hunks;
            self.hex_view = views_hex::HexCompareView::empty(self.hex_view.mode, max_hunks);
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let mode = self.hex_view.mode;
        let max_hunks = self.hex_view.max_hunks;
        let _ = JobRunner::spawn(move |_cancel, _progress| {
            let res = compare_binary_files_aligned(
                Path::new(&left),
                Path::new(&right),
                max_hunks,
                mode.to_alignment(),
            )
            .map(|cmp| {
                let view = views_hex::HexCompareView::from_comparison(&cmp, mode, max_hunks);
                let lines = viewmodel::build_hex_summary(&cmp);
                (lines, view)
            })
            .map_err(|err| err.to_string());
            let _ = tx.send(CompareResult::Hex(res));
            Ok(())
        });
    }

    fn run_merge(&mut self) {
        let (base, left, right) = self.triple_paths();
        if !self.require_triple(&base, &left, &right, "file") {
            self.merge_model = None;
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let _ = JobRunner::spawn(move |_cancel, _progress| {
            let base_lines = match load_lines(Path::new(&base)) {
                Ok(lines) => lines,
                Err(err) => {
                    let _ = tx.send(CompareResult::Merge(Err(err.to_string())));
                    return Ok(());
                }
            };
            let left_lines = match load_lines(Path::new(&left)) {
                Ok(lines) => lines,
                Err(err) => {
                    let _ = tx.send(CompareResult::Merge(Err(err.to_string())));
                    return Ok(());
                }
            };
            let right_lines = match load_lines(Path::new(&right)) {
                Ok(lines) => lines,
                Err(err) => {
                    let _ = tx.send(CompareResult::Merge(Err(err.to_string())));
                    return Ok(());
                }
            };
            let base_refs: Vec<&str> = base_lines.iter().map(String::as_str).collect();
            let left_refs: Vec<&str> = left_lines.iter().map(String::as_str).collect();
            let right_refs: Vec<&str> = right_lines.iter().map(String::as_str).collect();
            let merged = autocompare_diff::merge3(&base_refs, &left_refs, &right_refs);
            let summary = viewmodel::build_merge_summary(&merged);
            let _ = tx.send(CompareResult::Merge(Ok(summary)));
            Ok(())
        });
    }

    fn run_table(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "table") {
            self.table_model = None;
            self.table_view.summary = views_table::TableKeySummary::default();
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let key_opts = self.table_view.to_key_options();
        let positional = self.table_view.is_positional();
        let _ = JobRunner::spawn(move |_cancel, _progress| {
            let rows = compare_table_files(
                Path::new(&left),
                Path::new(&right),
                &TableOptions::default(),
            )
            .map(|cmp| viewmodel::build_table_rows(&cmp))
            .map_err(|err| err.to_string());
            // Key-aligned tolerant summary over the same inputs; shares
            // the plain loader, so a keyed-only failure is not expected.
            let summary = compare_table_files_by_key(
                Path::new(&left),
                Path::new(&right),
                &TableOptions::default(),
                &key_opts,
            )
            .map(|cmp| views_table::summarize(&cmp.diff, positional))
            .map_err(|err| err.to_string());
            let combined = match (rows, summary) {
                (Ok(model), Ok(sum)) => Ok((model, sum)),
                (Ok(model), Err(_)) => Ok((model, views_table::TableKeySummary::default())),
                (Err(err), _) => Err(err),
            };
            let _ = tx.send(CompareResult::Table(combined));
            Ok(())
        });
    }

    fn run_sync(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "folder") {
            self.sync_model = None;
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let _ = JobRunner::spawn(move |_cancel, _progress| {
            let res = plan_sync(Path::new(&left), Path::new(&right), &SyncOptions::default())
                .map(|plan| SyncChecklist::from_plan(&plan))
                .map_err(|err| err.to_string());
            let _ = tx.send(CompareResult::Sync(res));
            Ok(())
        });
    }

    fn run_picture(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "picture") {
            self.picture_model = None;
            self.heatmap = None;
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let _ = JobRunner::spawn(move |_cancel, _progress| {
            let opts = PictureOptions::default();
            let res = compare_picture_files(Path::new(&left), Path::new(&right), &opts)
                .map(|cmp| PictureCompareModel::from_diff(&cmp.diff))
                .map_err(|err| err.to_string());
            let hm = load_heatmap_rgba(Path::new(&left), Path::new(&right)).ok();
            let _ = tx.send(CompareResult::Picture(res, hm));
            Ok(())
        });
    }

    fn run_folder_merge(&mut self) {
        let (base, left, right) = self.triple_paths();
        if !self.require_triple(&base, &left, &right, "folder") {
            self.folder_merge_model = None;
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let _ = JobRunner::spawn(move |_cancel, _progress| {
            let res = plan_tree_merge(Path::new(&base), Path::new(&left), Path::new(&right))
                .map(|plan| FolderMergeChecklist::from_plan(&plan))
                .map_err(|err| err.to_string());
            let _ = tx.send(CompareResult::FolderMerge(res));
            Ok(())
        });
    }

    fn run_audio(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "audio") {
            self.audio_model = None;
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let _ = JobRunner::spawn(move |_cancel, _progress| {
            let res = compare_audio_files(Path::new(&left), Path::new(&right))
                .map(|cmp| AudioChecklist::from_diff(&cmp.diff))
                .map_err(|err| err.to_string());
            let _ = tx.send(CompareResult::Audio(res));
            Ok(())
        });
    }
}

/// Counts text rows by kind: (equal, changed, deleted, inserted).
fn text_kind_counts(model: &viewmodel::TextRows) -> (usize, usize, usize, usize) {
    let mut eq = 0usize;
    let mut ch = 0usize;
    let mut del = 0usize;
    let mut ins = 0usize;
    for row in &model.rows {
        match row.kind {
            viewmodel::TextRowKind::Equal => eq = eq.saturating_add(1),
            viewmodel::TextRowKind::Changed => ch = ch.saturating_add(1),
            viewmodel::TextRowKind::Deleted => del = del.saturating_add(1),
            viewmodel::TextRowKind::Inserted => ins = ins.saturating_add(1),
        }
    }
    (eq, ch, del, ins)
}

/// Decodes both PNGs and builds the display heatmap bytes. Every
/// failure mode (unreadable file, undecodable bytes) is a message,
/// never a panic.
fn load_heatmap_rgba(left: &Path, right: &Path) -> Result<(Vec<u8>, u32, u32), String> {
    let a_bytes =
        std::fs::read(left).map_err(|err| format!("reading {} failed: {err}", left.display()))?;
    let b_bytes =
        std::fs::read(right).map_err(|err| format!("reading {} failed: {err}", right.display()))?;
    let a = autocompare_diff::decode_png(&a_bytes)
        .ok_or_else(|| format!("{} is not a decodable PNG", left.display()))?;
    let b = autocompare_diff::decode_png(&b_bytes)
        .ok_or_else(|| format!("{} is not a decodable PNG", right.display()))?;
    let heatmap = autocompare_diff::picture_heatmap(&a, &b);
    Ok(viewmodel::build_heatmap_rgba(&heatmap))
}

// ----------------------------------------------------------------------------
// Picture / sync / folder-merge / audio structured models (Wave J-B)
// ----------------------------------------------------------------------------
//
// The run paths for these tabs return structured models instead of
// plain summary strings: the picture tab keeps its compare result
// plus view params (mode/zoom/pan/overlay), and the sync,
// folder-merge and audio tabs keep per-file action rows. Everything
// here is pure and headless-testable; `views.rs` only paints it.

/// Smallest picture zoom the slider offers, in percent.
pub const PICTURE_ZOOM_MIN: u32 = 25;
/// Largest picture zoom the slider offers, in percent.
pub const PICTURE_ZOOM_MAX: u32 = 400;
/// Largest overlay mix, in percent (0 = full left, 100 = full right).
pub const PICTURE_OVERLAY_MAX: u8 = 100;
/// One pan nudge step, in pixels.
pub const PICTURE_PAN_STEP: i32 = 32;
/// Pan clamp, in pixels, so nudges never run away.
pub const PICTURE_PAN_LIMIT: i32 = 4096;

/// How the picture tab shows the two images.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PictureMode {
    /// Left and right images in two columns.
    #[default]
    SideBySide,
    /// One heatmap frame with a left/right mix slider.
    Overlay,
}

impl PictureMode {
    /// All modes in tab order, for the paint radio row.
    pub fn all() -> [PictureMode; 2] {
        [PictureMode::SideBySide, PictureMode::Overlay]
    }

    /// Short label for the paint radio row.
    pub fn label(self) -> &'static str {
        match self {
            PictureMode::SideBySide => "side-by-side",
            PictureMode::Overlay => "overlay",
        }
    }
}

/// View params of the picture tab: layout mode, overlay mix, zoom and
/// pan. All setters clamp, so paint never sees a wild value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureViewState {
    /// Layout mode.
    pub mode: PictureMode,
    /// Overlay mix in percent (0 = full left, 100 = full right).
    pub overlay_mix: u8,
    /// Zoom in percent (25..=400).
    pub zoom_pct: u32,
    /// Horizontal pan in pixels.
    pub pan_x: i32,
    /// Vertical pan in pixels.
    pub pan_y: i32,
}

impl Default for PictureViewState {
    fn default() -> Self {
        Self {
            mode: PictureMode::SideBySide,
            overlay_mix: 50,
            zoom_pct: 100,
            pan_x: 0,
            pan_y: 0,
        }
    }
}

impl PictureViewState {
    /// Sets the overlay mix, clamped to 0..=100.
    pub fn set_overlay_mix(&mut self, mix: u8) {
        self.overlay_mix = mix.min(PICTURE_OVERLAY_MAX);
    }

    /// Sets the zoom percent, clamped to 25..=400.
    pub fn set_zoom_pct(&mut self, pct: u32) {
        self.zoom_pct = pct.clamp(PICTURE_ZOOM_MIN, PICTURE_ZOOM_MAX);
    }

    /// Moves the pan by `(dx, dy)` pixels, clamped to ±4096.
    pub fn nudge_pan(&mut self, dx: i32, dy: i32) {
        self.pan_x = self
            .pan_x
            .saturating_add(dx)
            .clamp(-PICTURE_PAN_LIMIT, PICTURE_PAN_LIMIT);
        self.pan_y = self
            .pan_y
            .saturating_add(dy)
            .clamp(-PICTURE_PAN_LIMIT, PICTURE_PAN_LIMIT);
    }

    /// Back to side-by-side at full scale without pan.
    pub fn reset_view(&mut self) {
        *self = PictureViewState::default();
    }

    /// Overlay mix as a 0.0..=1.0 ratio.
    pub fn overlay_ratio(&self) -> f32 {
        f32::from(self.overlay_mix) / 100.0
    }

    /// Zoom as a 0.25..=4.0 scale factor.
    pub fn zoom_factor(&self) -> f32 {
        (self.zoom_pct.max(PICTURE_ZOOM_MIN) as f32) / 100.0
    }

    /// One-line view description for the status bar.
    pub fn describe(&self) -> String {
        format!(
            "{} view, overlay {}%, zoom {}%, pan ({},{})",
            self.mode.label(),
            self.overlay_mix,
            self.zoom_pct,
            self.pan_x,
            self.pan_y
        )
    }
}

/// Structured result of one picture compare: the diff fields paint
/// needs plus the CLI-worded summary lines.
#[derive(Debug, Clone, Default)]
pub struct PictureCompareModel {
    /// Left image width.
    pub width_l: u32,
    /// Left image height.
    pub height_l: u32,
    /// Right image width.
    pub width_r: u32,
    /// Right image height.
    pub height_r: u32,
    /// Overlap width paint compares over.
    pub overlap_w: u32,
    /// Overlap height paint compares over.
    pub overlap_h: u32,
    /// Differing pixels in the overlap.
    pub differing: u64,
    /// Total pixels in the overlap.
    pub total: u64,
    /// Differing-pixel bounds, when any differ.
    pub bbox: Option<(u32, u32, u32, u32)>,
    /// Same dimensions and no differing pixel anywhere.
    pub identical: bool,
    /// CLI-worded summary lines, for the details section.
    pub lines: Vec<String>,
}

impl PictureCompareModel {
    /// Builds the model from an engine diff, reusing the CLI-worded
    /// summary so both clients tell the same story.
    pub fn from_diff(diff: &PictureDiff) -> Self {
        Self {
            width_l: diff.width_l,
            height_l: diff.height_l,
            width_r: diff.width_r,
            height_r: diff.height_r,
            overlap_w: diff.overlap_w,
            overlap_h: diff.overlap_h,
            differing: diff.differing,
            total: diff.total,
            bbox: diff.bbox,
            identical: diff.identical,
            lines: viewmodel::build_picture_summary(diff),
        }
    }

    /// Short verdict for the status bar.
    pub fn status_summary(&self) -> String {
        if self.identical {
            String::from("no differences")
        } else {
            let pct = if self.total == 0 {
                0.0
            } else {
                self.differing as f64 / self.total as f64 * 100.0
            };
            format!(
                "{} of {} pixels differ ({pct:.2}%)",
                self.differing, self.total
            )
        }
    }

    /// First summary line, or a no-results note when empty.
    pub fn summary_first_line(&self) -> &str {
        self.lines.first().map_or("no results yet", String::as_str)
    }
}

/// One planned sync step as a checklist row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncCheckRow {
    /// Path relative to the compared roots, '/'-separated.
    pub rel_path: String,
    /// Whether the subject is a directory.
    pub is_dir: bool,
    /// What the plan would do.
    pub action: SyncAction,
    /// Whether the step stays selected for Apply.
    pub checked: bool,
}

impl SyncCheckRow {
    /// Status glyph, like the CLI's: `>` rightwards, `<` leftwards,
    /// `x` for prune deletes.
    pub fn glyph(&self) -> char {
        match self.action {
            SyncAction::CopyToRight => '>',
            SyncAction::CopyToLeft => '<',
            SyncAction::DeleteRight | SyncAction::DeleteLeft => 'x',
        }
    }

    /// Plain-English action label for the checklist row.
    pub fn action_label(&self) -> &'static str {
        match self.action {
            SyncAction::CopyToRight => "copy to right",
            SyncAction::CopyToLeft => "copy to left",
            SyncAction::DeleteRight => "delete right",
            SyncAction::DeleteLeft => "delete left",
        }
    }

    /// Paint-ready row text: glyph plus path with the directory mark.
    pub fn display_line(&self) -> String {
        let mark = if self.is_dir { "/" } else { "" };
        format!("{} {}{mark}", self.glyph(), self.rel_path)
    }
}

/// The sync tab model: per-file action rows plus skips.
#[derive(Debug, Clone, Default)]
pub struct SyncChecklist {
    /// Planned steps as checklist rows, parents before children.
    pub rows: Vec<SyncCheckRow>,
    /// Entries left alone: (relative path, plain-English reason).
    pub skipped: Vec<(String, String)>,
    /// Whether the walk stopped at the depth limit.
    pub truncated: bool,
}

impl SyncChecklist {
    /// Builds the checklist from an engine plan. Rows start checked;
    /// skips stay informational and are never rows.
    pub fn from_plan(plan: &SyncPlan) -> Self {
        Self {
            rows: plan
                .steps
                .iter()
                .map(|step| SyncCheckRow {
                    rel_path: step.rel_path.clone(),
                    is_dir: step.is_dir,
                    action: step.action,
                    checked: true,
                })
                .collect(),
            skipped: plan.skipped.clone(),
            truncated: plan.truncated,
        }
    }

    /// Honest counts line, mirroring the CLI's planned/skipped wording.
    pub fn summary(&self) -> String {
        let mut out = format!(
            "{} step(s) planned, {} skipped",
            self.rows.len(),
            self.skipped.len()
        );
        if self.truncated {
            out.push_str(" (truncated at the depth limit)");
        }
        out
    }

    /// Rows still selected for Apply.
    pub fn checked_count(&self) -> usize {
        self.rows.iter().filter(|row| row.checked).count()
    }

    /// Flips one row; false when the index names no row.
    pub fn set_checked(&mut self, idx: usize, on: bool) -> bool {
        match self.rows.get_mut(idx) {
            Some(row) => {
                row.checked = on;
                true
            }
            None => false,
        }
    }

    /// Selects or clears every row.
    pub fn set_all(&mut self, on: bool) {
        for row in &mut self.rows {
            row.checked = on;
        }
    }
}

/// One planned folder-merge step as a checklist row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderMergeCheckRow {
    /// Path relative to the merged roots, '/'-separated.
    pub rel_path: String,
    /// What the plan would do.
    pub action: TreeMergeAction,
    /// Whether the step stays selected for Apply.
    pub checked: bool,
}

impl FolderMergeCheckRow {
    /// Status glyph: `L`/`R` take a side, `M` clean merge,
    /// `C` conflict, `+` new directory.
    pub fn glyph(&self) -> char {
        match self.action {
            TreeMergeAction::TakeLeft => 'L',
            TreeMergeAction::TakeRight => 'R',
            TreeMergeAction::MergedText => 'M',
            TreeMergeAction::Conflict => 'C',
            TreeMergeAction::MakeDir => '+',
        }
    }

    /// Plain-English action label for the checklist row.
    pub fn action_label(&self) -> &'static str {
        match self.action {
            TreeMergeAction::TakeLeft => "take left",
            TreeMergeAction::TakeRight => "take right",
            TreeMergeAction::MergedText => "merged text",
            TreeMergeAction::Conflict => "conflict",
            TreeMergeAction::MakeDir => "make dir",
        }
    }

    /// Whether this row needs a manual decision.
    pub fn is_conflict(&self) -> bool {
        matches!(self.action, TreeMergeAction::Conflict)
    }

    /// Paint-ready row text: glyph plus path with the directory mark.
    pub fn display_line(&self) -> String {
        let mark = if matches!(self.action, TreeMergeAction::MakeDir) {
            "/"
        } else {
            ""
        };
        format!("{} {}{mark}", self.glyph(), self.rel_path)
    }
}

/// The folder-merge tab model: per-file action rows plus conflicts.
#[derive(Debug, Clone, Default)]
pub struct FolderMergeChecklist {
    /// Planned steps as checklist rows, sorted by relative path.
    pub rows: Vec<FolderMergeCheckRow>,
    /// Relative paths with conflicts, sorted.
    pub conflicts: Vec<String>,
    /// Entries left out of the output, with reasons.
    pub skipped: Vec<(String, String)>,
}

impl FolderMergeChecklist {
    /// Builds the checklist from an engine plan. Rows start checked;
    /// conflict rows stay checked so Apply reviews them, never skips
    /// them silently.
    pub fn from_plan(plan: &TreeMergePlan) -> Self {
        Self {
            rows: plan
                .steps
                .iter()
                .map(|step| FolderMergeCheckRow {
                    rel_path: step.rel_path.clone(),
                    action: step.action,
                    checked: true,
                })
                .collect(),
            conflicts: plan.conflicts.clone(),
            skipped: plan.skipped.clone(),
        }
    }

    /// Honest counts line, mirroring the CLI's wording.
    pub fn summary(&self) -> String {
        format!(
            "{} step(s) planned, {} omitted, {} conflict(s)",
            self.rows.len(),
            self.skipped.len(),
            self.conflicts.len()
        )
    }

    /// Rows still selected for Apply.
    pub fn checked_count(&self) -> usize {
        self.rows.iter().filter(|row| row.checked).count()
    }

    /// Flips one row; false when the index names no row.
    pub fn set_checked(&mut self, idx: usize, on: bool) -> bool {
        match self.rows.get_mut(idx) {
            Some(row) => {
                row.checked = on;
                true
            }
            None => false,
        }
    }

    /// Selects or clears every row.
    pub fn set_all(&mut self, on: bool) {
        for row in &mut self.rows {
            row.checked = on;
        }
    }
}

/// Which audio fact one checklist row carries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AudioCheckKind {
    /// Left stream configuration.
    #[default]
    LeftConfig,
    /// Right stream configuration.
    RightConfig,
    /// The compare verdict.
    Verdict,
}

/// One audio fact as a checklist row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioCheckRow {
    /// Which fact this row carries.
    pub kind: AudioCheckKind,
    /// Short row label.
    pub label: String,
    /// Row detail (config text or verdict text).
    pub detail: String,
    /// Whether the row stays selected.
    pub checked: bool,
}

/// The audio tab model: per-stream config rows plus a verdict row.
#[derive(Debug, Clone, Default)]
pub struct AudioChecklist {
    /// Config rows for both streams plus the verdict row.
    pub rows: Vec<AudioCheckRow>,
    /// Same configuration and no differing sample anywhere.
    pub identical: bool,
    /// Sample rates or channel counts differ; samples are not comparable.
    pub config_mismatch: bool,
}

impl AudioChecklist {
    /// Builds the checklist from an engine diff: one row per stream
    /// config plus one verdict row.
    pub fn from_diff(diff: &AudioDiff) -> Self {
        let left = AudioCheckRow {
            kind: AudioCheckKind::LeftConfig,
            label: String::from("left"),
            detail: format!(
                "{:.3}s, {} Hz, {} channel(s)",
                diff.duration_l, diff.sample_rate_l, diff.channels_l
            ),
            checked: true,
        };
        let right = AudioCheckRow {
            kind: AudioCheckKind::RightConfig,
            label: String::from("right"),
            detail: format!(
                "{:.3}s, {} Hz, {} channel(s)",
                diff.duration_r, diff.sample_rate_r, diff.channels_r
            ),
            checked: true,
        };
        let verdict = if diff.config_mismatch {
            AudioCheckRow {
                kind: AudioCheckKind::Verdict,
                label: String::from("verdict"),
                detail: String::from(
                    "not comparable: stream configurations differ; sample comparison skipped",
                ),
                checked: true,
            }
        } else if diff.identical {
            AudioCheckRow {
                kind: AudioCheckKind::Verdict,
                label: String::from("verdict"),
                detail: String::from("no differences"),
                checked: true,
            }
        } else {
            let at = diff.first_diff_at.map_or_else(
                || String::from("first difference: -"),
                |at| format!("first difference at {at:.3}s"),
            );
            let pct = if diff.windows_total > 0 {
                diff.windows_differing as f64 / diff.windows_total as f64 * 100.0
            } else {
                0.0
            };
            AudioCheckRow {
                kind: AudioCheckKind::Verdict,
                label: String::from("verdict"),
                detail: format!(
                    "{at}; differing 10ms windows: {}/{} ({pct:.2}%)",
                    diff.windows_differing, diff.windows_total
                ),
                checked: true,
            }
        };
        Self {
            rows: vec![left, right, verdict],
            identical: diff.identical,
            config_mismatch: diff.config_mismatch,
        }
    }

    /// Honest one-line verdict for the status bar.
    pub fn summary(&self) -> String {
        if self.config_mismatch {
            String::from("not comparable: config mismatch")
        } else if self.identical {
            String::from("no differences")
        } else {
            self.rows
                .iter()
                .find(|row| row.kind == AudioCheckKind::Verdict)
                .map_or_else(|| String::from("compared"), |row| row.detail.clone())
        }
    }

    /// Flips one row; false when the index names no row.
    pub fn set_checked(&mut self, idx: usize, on: bool) -> bool {
        match self.rows.get_mut(idx) {
            Some(row) => {
                row.checked = on;
                true
            }
            None => false,
        }
    }
}

impl App {
    /// Switches the picture layout mode, keeping zoom/pan/overlay.
    pub fn set_picture_mode(&mut self, mode: PictureMode) {
        self.picture_view.mode = mode;
        self.status = format!("picture: {} view", mode.label());
    }

    /// Sets the overlay mix (0 = full left, 100 = full right).
    pub fn set_picture_overlay(&mut self, mix: u8) {
        self.picture_view.set_overlay_mix(mix);
        self.status = format!("picture overlay: {}%", self.picture_view.overlay_mix);
    }

    /// Sets the picture zoom percent (clamped 25..=400).
    pub fn set_picture_zoom(&mut self, pct: u32) {
        self.picture_view.set_zoom_pct(pct);
        self.status = format!("picture zoom: {}%", self.picture_view.zoom_pct);
    }

    /// Nudges the picture pan by one step in a direction.
    pub fn nudge_picture_pan(&mut self, dx: i32, dy: i32) {
        self.picture_view.nudge_pan(
            dx.saturating_mul(PICTURE_PAN_STEP),
            dy.saturating_mul(PICTURE_PAN_STEP),
        );
        self.status = format!(
            "picture pan: ({},{})",
            self.picture_view.pan_x, self.picture_view.pan_y
        );
    }

    /// Resets the picture view to side-by-side at full scale.
    pub fn reset_picture_view(&mut self) {
        self.picture_view.reset_view();
        self.status = String::from("picture view reset");
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Title(self.window_title()));
        // Drag and drop handling
        let dropped_files = ui.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            let paths: Vec<String> = dropped_files
                .into_iter()
                .map(|d| d.path().display().to_string())
                .collect();
            self.apply_dropped_paths(&paths);
            self.status = format!("dropped {} path(s)", paths.len());
        }

        // Process background comparison completion
        if let Some(rx) = &self.rx {
            if let Ok(res) = rx.try_recv() {
                match res {
                    CompareResult::Text(res) => match res {
                        Ok(model) => {
                            let n = model.rows.len();
                            self.text_model = Some(model);
                            self.error = None;
                            self.status = format!("text compare done: {n} row(s)");
                        }
                        Err(err) => {
                            self.text_model = None;
                            self.status = format!("text compare failed: {err}");
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Folder(res) => match res {
                        Ok(model) => {
                            let n = model.0.len();
                            self.folder_model = Some(model);
                            self.error = None;
                            self.status = format!("folder compare done: {n} entries");
                        }
                        Err(err) => {
                            self.folder_model = None;
                            self.status = format!("folder compare failed: {err}");
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Hex(res) => match res {
                        Ok((lines, view)) => {
                            let n = view.hunks.len();
                            self.hex_model = Some(lines);
                            self.hex_view = view;
                            self.error = None;
                            self.status = format!("hex compare done: {n} hunk(s)");
                        }
                        Err(err) => {
                            self.hex_model = None;
                            if let Some(view) = self.hex_over_cap_view() {
                                self.hex_view = view;
                                self.status = self.hex_view.status_line();
                            } else {
                                let max_hunks = self.hex_view.max_hunks;
                                self.hex_view =
                                    views_hex::HexCompareView::empty(self.hex_view.mode, max_hunks);
                                self.status = format!("hex compare failed: {err}");
                            }
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Merge(res) => match res {
                        Ok(model) => {
                            let n = model.lines.len();
                            let conflicted = model.conflicted;
                            self.merge_model = Some(model);
                            self.error = None;
                            self.status = format!(
                                "merge done: {n} line(s){}",
                                if conflicted { ", conflicted" } else { "" }
                            );
                        }
                        Err(err) => {
                            self.merge_model = None;
                            self.status = format!("merge failed: {err}");
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Table(res) => match res {
                        Ok((model, summary)) => {
                            let n = model.rows.len();
                            self.table_model = Some(model);
                            self.table_view.summary = summary;
                            self.error = None;
                            self.status = format!("table compare done: {n} row(s)");
                        }
                        Err(err) => {
                            self.table_model = None;
                            self.table_view.summary = views_table::TableKeySummary::default();
                            self.status = format!("table compare failed: {err}");
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Sync(res) => match res {
                        Ok(model) => {
                            let n = model.rows.len();
                            self.sync_model = Some(model);
                            self.error = None;
                            self.status = format!("sync plan done: {n} step(s)");
                        }
                        Err(err) => {
                            self.sync_model = None;
                            self.status = format!("sync plan failed: {err}");
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Picture(res, hm) => {
                        match res {
                            Ok(model) => {
                                let summary = model.status_summary();
                                self.picture_model = Some(model);
                                self.error = None;
                                self.status = format!("picture compare done: {summary}");
                            }
                            Err(err) => {
                                self.picture_model = None;
                                self.status = format!("picture compare failed: {err}");
                                self.error = Some(err);
                            }
                        }
                        if let Some((bytes, w, h)) = hm {
                            if let Some(image) = views::heatmap_color_image(&bytes, w, h) {
                                self.heatmap = Some(ui.ctx().load_texture(
                                    "autocompare-heatmap",
                                    image,
                                    egui::TextureOptions::default(),
                                ));
                            } else {
                                self.heatmap = None;
                            }
                        } else {
                            self.heatmap = None;
                        }
                    }
                    CompareResult::FolderMerge(res) => match res {
                        Ok(model) => {
                            let n = model.rows.len();
                            self.folder_merge_model = Some(model);
                            self.error = None;
                            self.status = format!("folder merge done: {n} step(s)");
                        }
                        Err(err) => {
                            self.folder_merge_model = None;
                            self.status = format!("folder merge failed: {err}");
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Audio(res) => match res {
                        Ok(model) => {
                            let n = model.rows.len();
                            self.audio_model = Some(model);
                            self.error = None;
                            self.status = format!("audio compare done: {n} row(s)");
                        }
                        Err(err) => {
                            self.audio_model = None;
                            self.status = format!("audio compare failed: {err}");
                            self.error = Some(err);
                        }
                    },
                }
                self.rx = None;
            } else {
                ui.ctx().request_repaint();
            }
        }

        // Central panel with unified outer margin.
        egui::Frame::central_panel(ui.style())
            .inner_margin(egui::Margin::same(views::OUTER_MARGIN))
            .show(ui, |ui| {
            views::paint_header(ui, &mut self.dark_mode);
            views::paint_tabs(ui, &mut self.tab);
            ui.label(RichText::new(self.session_identity()).small().weak());
            ui.add_space(views::TAB_BOTTOM_GAP);

            // Single aligned toolbar row: actions + view toggles.
            let toolbar_active = self.tab != Tab::Home;
            ui.horizontal(|ui| {
                let swap_resp = ui.add_enabled(
                    toolbar_active,
                    egui::Button::new("Swap Sides"),
                );
                if swap_resp.clicked() {
                    // Swap, then re-run so both sides refresh like
                    // Beyond Compare does; empty sides report validation.
                    if self.swap_sides() {
                        self.run_active_tab();
                    }
                }
                let reload_resp = ui.add_enabled(
                    toolbar_active,
                    egui::Button::new("Reload / Recompare"),
                );
                if reload_resp.clicked() {
                    self.run_active_tab();
                }
                ui.separator();
                let mut wrap = self.word_wrap;
                if ui.checkbox(&mut wrap, "Word Wrap").changed() {
                    self.set_word_wrap(wrap);
                }
                let mut numbers = self.show_line_numbers;
                if ui.checkbox(&mut numbers, "Line Numbers").changed() {
                    self.set_line_numbers(numbers);
                }
                let mut syntax = self.syntax_highlight;
                if ui.checkbox(&mut syntax, "Syntax Highlighting").changed() {
                    self.set_syntax_highlight(syntax);
                }
            });
            ui.add_space(views::TIGHT_GAP);

            let tab = self.tab;
            let mut compare_clicked = false;
            match tab {
                Tab::Home => {
                    views::paint_home(ui, &mut self.tab);
                    ui.add_space(views::SECTION_GAP);
                    ui.label(RichText::new("Start here: open Text or Folder above, type both paths, then Compare. Swap Sides flips inputs; Reload reruns the same compare.").weak());
                }
                Tab::Text => views::paint_paths(
                    ui,
                    "file",
                    false,
                    &mut self.text.left,
                    &mut self.text.right,
                    &mut compare_clicked,
                ),
                Tab::Folder => views::paint_paths(
                    ui,
                    "folder",
                    true,
                    &mut self.folder.left,
                    &mut self.folder.right,
                    &mut compare_clicked,
                ),
                Tab::Hex => views::paint_paths(
                    ui,
                    "file",
                    false,
                    &mut self.hex.left,
                    &mut self.hex.right,
                    &mut compare_clicked,
                ),
                Tab::Merge => views::paint_triple_paths(
                    ui,
                    "file",
                    false,
                    &mut self.merge.base,
                    &mut self.merge.left,
                    &mut self.merge.right,
                    &mut compare_clicked,
                ),
                Tab::Table => views::paint_paths(
                    ui,
                    "table",
                    false,
                    &mut self.table.left,
                    &mut self.table.right,
                    &mut compare_clicked,
                ),
                Tab::Sync => views::paint_paths(
                    ui,
                    "folder",
                    true,
                    &mut self.sync.left,
                    &mut self.sync.right,
                    &mut compare_clicked,
                ),
                Tab::Picture => views::paint_paths(
                    ui,
                    "picture",
                    false,
                    &mut self.picture.left,
                    &mut self.picture.right,
                    &mut compare_clicked,
                ),
                Tab::FolderMerge => views::paint_triple_paths(
                    ui,
                    "folder",
                    true,
                    &mut self.folder_merge.base,
                    &mut self.folder_merge.left,
                    &mut self.folder_merge.right,
                    &mut compare_clicked,
                ),
                Tab::Audio => views::paint_paths(
                    ui,
                    "audio",
                    false,
                    &mut self.audio.left,
                    &mut self.audio.right,
                    &mut compare_clicked,
                ),
            }

            if self.rx.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("Comparing on background thread...").weak());
                });
            }

            if compare_clicked {
                self.run_active_tab();
            }

            if let Some(err) = self.error.clone() {
                views::paint_error(ui, err.as_str());
            }
            ui.add_space(views::TIGHT_GAP);

            match tab {
                Tab::Home => {}
                Tab::Text => {
                    if let Some(snapshot) = self.text_model.clone() {
                        ui.style_mut().wrap_mode = Some(if self.word_wrap {
                            egui::TextWrapMode::Wrap
                        } else {
                            egui::TextWrapMode::Extend
                        });
                        let mut view = self.view_text_for_paint(&snapshot);
                        views::paint_text(ui, &mut view);
                        // paint_text works on the view clone (toggles are
                        // view-only): propagate navigation selection and
                        // display-filter choices back so they stick.
                        if view.selected != snapshot.selected
                            && let Some(real) = self.text_model.as_mut()
                        {
                            real.selected = view.selected;
                        }
                        if view.filter != snapshot.filter {
                            if let Some(real) = self.text_model.as_mut() {
                                real.filter = view.filter;
                            }
                            let name = match view.filter {
                                viewmodel::TextFilter::All => "all rows",
                                viewmodel::TextFilter::Differences => "differences only",
                                viewmodel::TextFilter::Same => "same rows only",
                                viewmodel::TextFilter::Context => "differences with context",
                            };
                            self.status = format!("text filter: {name}");
                        }
                        // The copy buttons inside paint_text mutate the
                        // view clone; propagate one copy back to the real
                        // model so the buttons work under any toggle
                        // combination, and report it in the status bar.
                        let mut copied = None;
                        if let Some(real) = self.text_model.as_ref() {
                            for (idx, view_row) in view.rows.iter().enumerate() {
                                let Some(real_row) = real.rows.get(idx) else {
                                    break;
                                };
                                if real_row.kind != viewmodel::TextRowKind::Equal
                                    && view_row.kind == viewmodel::TextRowKind::Equal
                                {
                                    if view_row.right == real_row.left
                                        && view_row.left == real_row.left
                                    {
                                        copied = Some((
                                            idx,
                                            viewmodel::CopyDirection::LeftToRight,
                                        ));
                                        break;
                                    }
                                    if view_row.left == real_row.right
                                        && view_row.right == real_row.right
                                    {
                                        copied = Some((
                                            idx,
                                            viewmodel::CopyDirection::RightToLeft,
                                        ));
                                        break;
                                    }
                                }
                            }
                        }
                        if let Some((idx, dir)) = copied {
                            if let Some(real) = self.text_model.as_mut() {
                                let _ = viewmodel::apply_copy(real, idx, dir);
                            }
                            let arrow = match dir {
                                viewmodel::CopyDirection::LeftToRight => "left->right",
                                viewmodel::CopyDirection::RightToLeft => "right->left",
                            };
                            self.status =
                                format!("copied row {} {arrow}", idx.saturating_add(1));
                        }
                    } else {
                        ui.label(
                            RichText::new("no text results yet: type both file paths and Compare.")
                                .weak(),
                        );
                    }
                }
                Tab::Folder => {
                    let has_rows = self.folder_rows().is_some_and(|r| !r.is_empty());
                    let has_error = self.error.is_some();
                    if self.folder_model.is_none() {
                        ui.label(
                            RichText::new(home::folder_empty_hint(false, has_error)).weak(),
                        );
                    } else {
                        // 4 display-filter checkboxes from FolderDisplay.
                        let mut display = self.folder_session.display;
                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(&mut display.hide_orphans, "Hide orphans");
                            ui.checkbox(&mut display.left_newer_only, "Left newer only");
                            ui.checkbox(&mut display.right_newer_only, "Right newer only");
                            ui.checkbox(&mut display.flatten, "Flatten");
                        });
                        if display != self.folder_session.display {
                            self.folder_session.display = display;
                            self.status = String::from("folder filter updated");
                        }
                        // Honest counts line via FolderCounts::summary.
                        ui.label(RichText::new(self.folder_counts_summary()).monospace().weak());
                        let nodes = self.folder_visible_tree();
                        if nodes.is_empty() {
                            ui.label(
                                RichText::new(home::folder_empty_hint(
                                    has_rows, has_error,
                                ))
                                .weak(),
                            );
                        } else {
                            let mut toggle_dir: Option<String> = None;
                            let mut double_clicked: Vec<tree::FolderTreeNode> = Vec::new();
                            for node in &nodes {
                                ui.horizontal(|ui| {
                                    if node.depth > 0 {
                                        ui.add_space(views::tree_indent(node.depth));
                                    }
                                    if node.row.is_dir && node.has_children {
                                        let icon =
                                            if node.is_expanded { "▼" } else { "▶" };
                                        if ui.small_button(icon).clicked() {
                                            toggle_dir = Some(node.row.rel_path.clone());
                                        }
                                    } else if node.row.is_dir {
                                        ui.label(RichText::new("  ").monospace());
                                    }
                                    let glyph = node.row.glyph.to_string();
                                    ui.label(RichText::new(glyph).monospace().weak());
                                    let mark =
                                        if node.row.is_dir { "/" } else { "" };
                                    let text =
                                        format!("{}{}", node.row.rel_path, mark);
                                    let resp = ui.selectable_label(false, text);
                                    if resp.double_clicked() {
                                        double_clicked.push(node.clone());
                                    }
                                });
                            }
                            if let Some(dir) = toggle_dir {
                                self.toggle_folder_dir(&dir);
                            }
                            for node in &double_clicked {
                                self.stage_folder_double_click(node);
                            }
                        }
                        // Sync preview: mode select, dry-run options,
                        // consent gate, Apply button.
                        ui.add_space(views::TIGHT_GAP);
                        ui.separator();
                        ui.label(RichText::new("Sync preview (dry-run)").strong());
                        let mode = self.folder_session.sync_mode;
                        ui.horizontal(|ui| {
                            let mut next = mode;
                            ui.radio_value(
                                &mut next,
                                tree::SyncModeChoice::UpdateBoth,
                                tree::SyncModeChoice::UpdateBoth.label(),
                            );
                            ui.radio_value(
                                &mut next,
                                tree::SyncModeChoice::Mirror,
                                tree::SyncModeChoice::Mirror.label(),
                            );
                            if next != mode {
                                self.set_folder_sync_mode(next);
                            }
                        });
                        ui.label(
                            RichText::new(self.folder_sync_options_label()).small().weak(),
                        );
                        let preview = self.folder_sync_preview();
                        ui.label(
                            RichText::new(preview.summary()).monospace().weak(),
                        );
                        for step in preview.steps.iter().take(8) {
                            ui.label(
                                RichText::new(format!(
                                    "{:?} {}",
                                    step.action, step.rel_path
                                ))
                                .monospace()
                                .small(),
                            );
                        }
                        let mut consented = self.folder_session.consented;
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut consented, "I understand Apply runs the plan");
                            ui.label(
                                RichText::new(home::sync_consent_hint(
                                    consented,
                                    preview.steps.len(),
                                ))
                                .small()
                                .weak(),
                            );
                        });
                        if consented != self.folder_session.consented {
                            self.set_folder_consent(consented);
                        }
                        let can_apply = self.folder_can_apply();
                        let apply = ui.add_enabled(can_apply, egui::Button::new("Apply sync plan"));
                        if apply.clicked() {
                            let n = preview.steps.len();
                            self.status = format!(
                                "sync apply approved: {n} step(s) (preview only, no filesystem writes)"
                            );
                        }
                    }
                }
                Tab::Hex => {
                    let mut mode = self.hex_view.mode;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Alignment:").small().weak());
                        for candidate in views_hex::HexCompareMode::all() {
                            ui.radio_value(&mut mode, candidate, candidate.label());
                        }
                    });
                    if mode != self.hex_view.mode {
                        let had_results = self.hex_model.is_some();
                        self.set_hex_mode(mode);
                        if had_results {
                            self.run_hex();
                        }
                    }
                    if self.hex_model.is_some() {
                        if let Some(lines) = self.hex_model.as_ref() {
                            views::paint_hex(ui, &mut self.hex_view, lines);
                        }
                    } else if self.hex_view.over_cap {
                        views::paint_hex_over_cap(ui, &self.hex_view);
                    } else {
                        ui.label(
                            RichText::new("no hex results yet: type both file paths and Compare.")
                                .weak(),
                        );
                    }
                }
                Tab::Merge => {
                    if let Some(model) = self.merge_model.as_mut() {
                        views::paint_merge(ui, model);
                    } else {
                        ui.label(
                            RichText::new("no merge results yet: type base + both sides and Compare.")
                                .weak(),
                        );
                    }
                }
                Tab::Table => {
                    let had_results = self.table_model.is_some();
                    let mut rerun = false;
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Keys:").small().weak());
                        let mut toggled: Option<usize> = None;
                        for col in 0..8usize {
                            let selected = self.table_view.key_columns.contains(&col);
                            if ui
                                .selectable_label(selected, col.to_string())
                                .on_hover_text("toggle key column")
                                .clicked()
                            {
                                toggled = Some(col);
                            }
                        }
                        if let Some(col) = toggled {
                            self.toggle_table_key_column(col);
                            rerun = had_results;
                        }
                        if ui.small_button("Clear keys").clicked()
                            && !self.table_view.key_columns.is_empty()
                        {
                            self.clear_table_keys();
                            rerun = had_results;
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Tolerance:").small().weak());
                        let mut tol = self.table_view.numeric_tolerance;
                        ui.radio_value(&mut tol, None, "exact");
                        ui.radio_value(&mut tol, Some(0.01), "0.01");
                        ui.radio_value(&mut tol, Some(0.1), "0.10");
                        ui.radio_value(&mut tol, Some(1.0), "1.00");
                        if tol != self.table_view.numeric_tolerance {
                            self.set_table_tolerance(tol);
                            self.table_tolerance_text.clear();
                            rerun = had_results;
                        }
                        ui.label(RichText::new("custom:").small().weak());
                        let mut custom = self.table_tolerance_text.clone();
                        let custom_resp = ui.add(
                            egui::TextEdit::singleline(&mut custom)
                                .desired_width(60.0)
                                .hint_text("e.g. 0.05"),
                        );
                        if custom_resp.changed() {
                            self.table_tolerance_text = custom;
                        }
                        if ui.small_button("Apply").clicked() {
                            match views::parse_tolerance_text(&self.table_tolerance_text) {
                                Some(value)
                                    if Some(value) != self.table_view.numeric_tolerance =>
                                {
                                    self.set_table_tolerance(Some(value));
                                    rerun = had_results;
                                }
                                Some(_) => {}
                                None => {
                                    self.status = String::from(
                                        "table tolerance: type a finite number >= 0, or use exact",
                                    );
                                }
                            }
                        }
                        let mut ignore_case = self.table_view.ignore_case;
                        if ui.checkbox(&mut ignore_case, "Ignore case").changed()
                            && ignore_case != self.table_view.ignore_case
                        {
                            self.set_table_ignore_case(ignore_case);
                            rerun = had_results;
                        }
                        let mut trim_space = self.table_view.trim_space;
                        if ui.checkbox(&mut trim_space, "Trim space").changed()
                            && trim_space != self.table_view.trim_space
                        {
                            self.set_table_trim_space(trim_space);
                            rerun = had_results;
                        }
                    });
                    if rerun {
                        self.run_table();
                    }
                    if let Some(model) = self.table_model.as_ref() {
                        views::paint_table_keyed(ui, &self.table_view, model);
                    } else {
                        ui.label(
                            RichText::new("no table results yet: type both table paths and Compare.")
                                .weak(),
                        );
                    }
                }
                Tab::Sync => {
                    if let Some(list) = self.sync_model.as_mut() {
                        views::paint_sync_checklist(ui, list);
                    } else {
                        ui.label(
                            RichText::new("no sync plan yet: type both folder paths and Compare.")
                                .weak(),
                        );
                    }
                }
                Tab::Picture => {
                    if self.picture_model.is_some() {
                        let mut view = self.picture_view;
                        let mut changed = false;
                        if let Some(model) = self.picture_model.as_ref() {
                            views::paint_picture_compare(
                                ui,
                                model,
                                &mut view,
                                self.heatmap.as_ref(),
                            );
                        }
                        if view != self.picture_view {
                            self.picture_view = view;
                            changed = true;
                        }
                        if changed {
                            self.status = format!(
                                "picture view: {}",
                                self.picture_view.describe()
                            );
                        }
                    } else {
                        ui.label(
                            RichText::new(
                                "no picture results yet: type both image paths and Compare.",
                            )
                            .weak(),
                        );
                    }
                }
                Tab::FolderMerge => {
                    if let Some(list) = self.folder_merge_model.as_mut() {
                        views::paint_folder_merge_checklist(ui, list);
                    } else {
                        ui.label(
                            RichText::new(
                                "no folder-merge plan yet: type base + both sides and Compare.",
                            )
                            .weak(),
                        );
                    }
                }
                Tab::Audio => {
                    if let Some(list) = self.audio_model.as_mut() {
                        views::paint_audio_checklist(ui, list);
                    } else {
                        ui.label(
                            RichText::new("no audio results yet: type both audio paths and Compare.")
                                .weak(),
                        );
                    }
                }
            }
            ui.add_space(views::SECTION_GAP);
            ui.separator();
            ui.add_space(views::FOOTER_GAP);
            ui.label(RichText::new(self.status_line()).small().weak());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_tabs_preserves_paths() {
        let mut app = App::default();
        assert_eq!(app.current_tab(), Tab::Home);

        app.set_tab(Tab::Text);
        app.set_text_paths("left.txt", "right.txt");
        assert_eq!(
            app.text_paths(),
            ("left.txt".to_string(), "right.txt".to_string())
        );

        app.set_tab(Tab::Folder);
        app.set_folder_paths("dir_a", "dir_b");
        assert_eq!(
            app.folder_paths(),
            ("dir_a".to_string(), "dir_b".to_string())
        );

        app.set_tab(Tab::Merge);
        app.set_merge_paths("base.txt", "left_merge.txt", "right_merge.txt");
        assert_eq!(
            app.merge_paths(),
            (
                "base.txt".to_string(),
                "left_merge.txt".to_string(),
                "right_merge.txt".to_string(),
            )
        );

        // Switch back to Text: paths are preserved!
        app.set_tab(Tab::Text);
        assert_eq!(
            app.text_paths(),
            ("left.txt".to_string(), "right.txt".to_string())
        );

        // Switch back to Folder: paths are preserved!
        app.set_tab(Tab::Folder);
        assert_eq!(
            app.folder_paths(),
            ("dir_a".to_string(), "dir_b".to_string())
        );

        // Switch back to Merge: triple paths are preserved!
        app.set_tab(Tab::Merge);
        assert_eq!(
            app.merge_paths(),
            (
                "base.txt".to_string(),
                "left_merge.txt".to_string(),
                "right_merge.txt".to_string(),
            )
        );
    }

    #[test]
    fn dropped_paths_populate_active_tab() {
        let mut app = App::default();
        app.set_tab(Tab::Text);
        app.apply_dropped_paths(&["file_a.txt".to_string(), "file_b.txt".to_string()]);
        assert_eq!(
            app.text_paths(),
            ("file_a.txt".to_string(), "file_b.txt".to_string())
        );
    }

    #[test]
    fn swap_sides_flips_text_paths_and_clears_model() {
        let mut app = App::default();
        app.set_tab(Tab::Text);
        app.set_text_paths("a.txt", "b.txt");
        assert!(app.swap_sides());
        assert_eq!(app.text_paths(), ("b.txt".to_string(), "a.txt".to_string()));
        assert!(app.text_model.is_none());
        assert!(app.status.contains("swapped sides"));
        assert!(app.session_identity().contains("b.txt"));
    }

    #[test]
    fn swap_sides_text_exchanges_paths_and_clears_model() {
        use crate::viewmodel::{TextFilter, TextRow, TextRowKind, TextRows};
        let mut app = App::default();
        app.set_tab(Tab::Text);
        app.set_text_paths("l.txt", "r.txt");
        app.text_model = Some(TextRows {
            rows: vec![TextRow {
                kind: TextRowKind::Equal,
                ..Default::default()
            }],
            truncated: false,
            coarse: false,
            pane_chars: 5,
            gutter_chars: 4,
            selected: None,
            filter: TextFilter::All,
        });
        assert!(app.swap_sides());
        assert_eq!(app.text_paths(), ("r.txt".to_string(), "l.txt".to_string()));
        assert!(app.text_model.is_none());
        assert!(app.error.is_none());
        assert!(app.status.contains("swapped sides"));
        assert!(app.session_identity().contains("r.txt"));
    }

    #[test]
    fn swap_sides_on_home_reports_without_swapping() {
        let mut app = App::default();
        assert_eq!(app.current_tab(), Tab::Home);
        assert!(!app.swap_sides());
        assert!(app.status.contains("swap needs"));
    }

    #[test]
    fn view_toggles_persist_and_note_status() {
        let mut app = App::default();
        assert!(!app.word_wrap);
        assert!(app.show_line_numbers);
        assert!(app.syntax_highlight);
        app.set_word_wrap(true);
        assert!(app.word_wrap);
        assert!(app.status.contains("word wrap on"));
        app.set_line_numbers(false);
        assert!(!app.show_line_numbers);
        app.set_syntax_highlight(false);
        assert!(!app.syntax_highlight);
        assert!(app.status.contains("syntax highlighting off"));
    }

    #[test]
    fn view_prefs_hide_line_numbers_and_accents() {
        use crate::viewmodel::{TextFilter, TextRow, TextRowKind, TextRows};
        let mut app = App::default();
        let model = TextRows {
            rows: vec![TextRow {
                kind: TextRowKind::Changed,
                left_no: Some(3),
                right_no: Some(3),
                left: "alpha".to_string(),
                right: "alphi".to_string(),
                accent_a: vec![(4, 5)],
                accent_b: vec![(4, 5)],
            }],
            truncated: false,
            coarse: false,
            pane_chars: 200,
            gutter_chars: 4,
            selected: None,
            filter: TextFilter::All,
        };
        app.set_line_numbers(false);
        app.set_syntax_highlight(false);
        app.set_word_wrap(true);
        let view = app.view_text_for_paint(&model);
        let Some(row) = view.rows.first() else {
            return;
        };
        assert_eq!(row.left_no, None);
        assert_eq!(row.right_no, None);
        assert!(row.accent_a.is_empty());
        assert!(row.accent_b.is_empty());
        assert_eq!(view.pane_chars, 80);
        assert_eq!(view.gutter_chars, 1);
    }

    #[test]
    fn view_clone_preserves_selection_and_filter() {
        use crate::viewmodel::{TextFilter, TextRows};
        let app = App::default();
        let model = TextRows {
            rows: Vec::new(),
            truncated: false,
            coarse: false,
            pane_chars: 10,
            gutter_chars: 4,
            selected: Some(2),
            filter: TextFilter::Differences,
        };
        let view = app.view_text_for_paint(&model);
        assert_eq!(view.selected, Some(2));
        assert_eq!(view.filter, TextFilter::Differences);
        assert!(view.rows.is_empty());
    }

    #[test]
    fn session_identity_carries_paths_and_tab() {
        let mut app = App::default();
        assert!(app.session_identity().contains("Home"));
        app.set_tab(Tab::Text);
        app.set_text_paths("l.txt", "r.txt");
        let id = app.session_identity();
        assert!(id.contains("l.txt"));
        assert!(id.contains("r.txt"));
        assert!(id.contains("Text"));
        assert!(app.window_title().contains("autocompare"));
    }

    #[test]
    fn status_line_reports_honest_counts() {
        use crate::viewmodel::{TextFilter, TextRow, TextRowKind, TextRows};
        let mut app = App::default();
        app.set_tab(Tab::Text);
        assert!(app.status_line().contains("no results yet"));
        app.text_model = Some(TextRows {
            rows: vec![
                TextRow {
                    kind: TextRowKind::Equal,
                    ..Default::default()
                },
                TextRow {
                    kind: TextRowKind::Deleted,
                    ..Default::default()
                },
            ],
            truncated: false,
            coarse: false,
            pane_chars: 10,
            gutter_chars: 4,
            selected: None,
            filter: TextFilter::All,
        });
        let line = app.status_line();
        assert!(line.contains("2 rows"));
        assert!(line.contains("1 equal"));
        assert!(line.contains("1 deleted"));
    }

    fn folder_row(path: &str, is_dir: bool, glyph: char) -> crate::viewmodel::FolderRow {
        crate::viewmodel::FolderRow {
            rel_path: path.to_string(),
            is_dir,
            glyph,
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: glyph != '=',
        }
    }

    fn folder_app_with_rows() -> App {
        let mut app = App::default();
        app.set_tab(Tab::Folder);
        app.folder_model = Some((
            vec![
                folder_row("src", true, '='),
                folder_row("src/a.txt", false, '>'),
                folder_row("orphan.txt", false, 'R'),
            ],
            String::from("3 entries"),
        ));
        app
    }

    #[test]
    fn folder_counts_line_uses_honest_summary() {
        let app = folder_app_with_rows();
        let line = app.folder_counts_summary();
        assert!(line.contains("3 entries"));
        assert!(line.contains("1 newer left"));
        assert!(line.contains("1 right-only"));
        let mut empty = App::default();
        empty.set_tab(Tab::Folder);
        assert!(empty.folder_counts_summary().contains("no results yet"));
    }

    #[test]
    fn folder_dir_toggle_expands_and_mirrors_legacy_set() {
        let mut app = folder_app_with_rows();
        assert!(app.folder_visible_tree().len() < 3);
        app.toggle_folder_dir("src");
        assert!(app.folder_session.expanded.contains("src"));
        assert!(app.folder_expanded.contains("src"));
        assert_eq!(app.folder_visible_tree().len(), 3);
        app.toggle_folder_dir("src");
        assert!(!app.folder_session.expanded.contains("src"));
    }

    #[test]
    fn folder_filters_narrow_visible_tree() {
        let mut app = folder_app_with_rows();
        app.toggle_folder_dir("src");
        assert_eq!(app.folder_visible_tree().len(), 3);
        app.folder_session.display.hide_orphans = true;
        assert_eq!(app.folder_visible_tree().len(), 2);
        app.folder_session.display.left_newer_only = true;
        assert_eq!(app.folder_visible_tree().len(), 1);
    }

    #[test]
    fn folder_double_click_stages_file_and_routes_to_text() {
        use crate::tree::FolderTreeNode;
        let mut app = folder_app_with_rows();
        let dir_node = FolderTreeNode {
            row: folder_row("src", true, '='),
            depth: 0,
            is_expanded: false,
            has_children: true,
        };
        assert_eq!(app.stage_folder_double_click(&dir_node), Tab::Folder);
        assert_eq!(app.current_tab(), Tab::Folder);
        let file_node = FolderTreeNode {
            row: folder_row("src/a.txt", false, '>'),
            depth: 1,
            is_expanded: false,
            has_children: false,
        };
        assert_eq!(app.stage_folder_double_click(&file_node), Tab::Text);
        assert_eq!(app.current_tab(), Tab::Text);
        assert!(app.status.contains("src/a.txt"));
    }

    #[test]
    fn hex_mode_change_clears_stale_results() {
        use crate::views_hex::HexCompareMode;
        let mut app = App::default();
        assert_eq!(app.hex_view.mode, HexCompareMode::Complete);
        app.hex_model = Some(vec![String::from("left: 1 bytes")]);
        app.set_hex_mode(HexCompareMode::Fast);
        assert_eq!(app.hex_view.mode, HexCompareMode::Fast);
        assert!(app.hex_model.is_none());
        assert!(app.hex_view.hunks.is_empty());
        assert!(app.status.contains("Fast"));
        // Same mode again: no wipe, no-op.
        app.hex_model = Some(vec![String::from("left: 1 bytes")]);
        app.set_hex_mode(HexCompareMode::Fast);
        assert!(app.hex_model.is_some());
    }

    #[test]
    fn hex_status_line_uses_view_counts() {
        let mut app = App::default();
        app.set_tab(Tab::Hex);
        assert!(app.status_line().contains("no results yet"));
        app.hex_model = Some(vec![String::from("left: 2 bytes")]);
        app.hex_view = crate::views_hex::HexCompareView::build_from_slices(
            &[1u8, 2],
            &[1u8, 3],
            crate::views_hex::HexCompareMode::None,
            8,
        );
        let line = app.status_line();
        assert!(line.contains("1 hunk(s)"), "unexpected: {line}");
        assert!(
            line.contains("first difference at offset"),
            "unexpected: {line}"
        );
    }

    #[test]
    fn hex_nav_helpers_move_selection() {
        let mut app = App {
            hex_view: crate::views_hex::HexCompareView::build_from_slices(
                &[0u8; 200],
                &[1u8; 200],
                crate::views_hex::HexCompareMode::None,
                8,
            ),
            ..Default::default()
        };
        app.hex_select_next();
        assert_eq!(app.hex_view.selected, Some(0));
        app.hex_select_prev();
        assert_eq!(
            app.hex_view.selected,
            Some(app.hex_view.hunks.len().saturating_sub(1))
        );
        assert!(app.hex_goto_first_diff());
        assert_eq!(app.hex_view.selected, Some(0));
    }

    #[test]
    fn hex_over_cap_view_only_for_big_inputs() {
        let mut app = App::default();
        app.set_tab(Tab::Hex);
        // Missing files measure as zero: no over-cap view.
        assert!(app.hex_over_cap_view().is_none());
    }

    #[test]
    fn table_option_changes_clear_stale_results() {
        let mut app = App {
            table_model: Some(crate::viewmodel::TableModel::default()),
            ..Default::default()
        };
        app.toggle_table_key_column(0);
        assert_eq!(app.table_view.key_columns, vec![0]);
        assert!(app.table_model.is_none());
        app.toggle_table_key_column(2);
        assert_eq!(app.table_view.key_columns, vec![0, 2]);
        app.clear_table_keys();
        assert!(app.table_view.key_columns.is_empty());
        assert!(app.table_view.is_positional());
        app.set_table_tolerance(Some(0.1));
        assert_eq!(app.table_view.numeric_tolerance, Some(0.1));
        app.set_table_tolerance(Some(f64::NAN));
        assert_eq!(app.table_view.numeric_tolerance, None);
        assert!(app.status.contains("exact text"));
        app.set_table_ignore_case(true);
        assert!(app.table_view.ignore_case);
        app.set_table_trim_space(true);
        assert!(app.table_view.trim_space);
    }

    #[test]
    fn swap_sides_resets_hex_and_table_views() {
        let mut app = App::default();
        app.set_tab(Tab::Hex);
        app.hex_model = Some(vec![String::from("x")]);
        app.hex_view.selected = Some(0);
        assert!(app.swap_sides());
        assert!(app.hex_model.is_none());
        assert_eq!(app.hex_view.selected, None);
        assert!(app.hex_view.hunks.is_empty());
        app.set_tab(Tab::Table);
        app.table_view.key_columns = vec![0];
        app.table_model = Some(crate::viewmodel::TableModel::default());
        assert!(app.swap_sides());
        assert!(app.table_model.is_none());
        // Options survive the swap; only the summary resets.
        assert_eq!(app.table_view.key_columns, vec![0]);
        assert_eq!(
            app.table_view.summary,
            crate::views_table::TableKeySummary::default()
        );
    }

    #[test]
    fn folder_sync_preview_consent_gates_apply() {
        use crate::tree::SyncModeChoice;
        let mut app = folder_app_with_rows();
        assert_eq!(app.folder_session.sync_mode, SyncModeChoice::UpdateBoth);
        assert!(!app.folder_can_apply());
        app.set_folder_consent(true);
        assert!(app.folder_can_apply());
        assert!(app.folder_sync_options_label().contains("plan_sync"));
        app.set_folder_sync_mode(SyncModeChoice::Mirror);
        assert!(!app.folder_can_apply());
        assert!(app.status.contains("Mirror"));
    }

    fn picture_diff(
        identical: bool,
        differing: u64,
        total: u64,
        bbox: Option<(u32, u32, u32, u32)>,
    ) -> autocompare_diff::PictureDiff {
        autocompare_diff::PictureDiff {
            width_l: 2,
            height_l: 2,
            width_r: 2,
            height_r: 2,
            overlap_w: 2,
            overlap_h: 2,
            differing,
            total,
            bbox,
            identical,
        }
    }

    #[test]
    fn picture_view_defaults_clamp_and_describe() {
        let mut view = PictureViewState::default();
        assert_eq!(view.mode, PictureMode::SideBySide);
        assert_eq!(view.overlay_mix, 50);
        assert_eq!(view.zoom_pct, 100);
        assert_eq!((view.pan_x, view.pan_y), (0, 0));
        assert_eq!(PictureMode::all().len(), 2);
        view.set_overlay_mix(200);
        assert_eq!(view.overlay_mix, 100);
        view.set_overlay_mix(30);
        assert_eq!(view.overlay_mix, 30);
        assert!((view.overlay_ratio() - 0.3).abs() < 0.001);
        view.set_zoom_pct(9);
        assert_eq!(view.zoom_pct, PICTURE_ZOOM_MIN);
        view.set_zoom_pct(900);
        assert_eq!(view.zoom_pct, PICTURE_ZOOM_MAX);
        assert!((view.zoom_factor() - 4.0).abs() < 0.001);
        view.nudge_pan(PICTURE_PAN_STEP, -PICTURE_PAN_STEP);
        assert_eq!((view.pan_x, view.pan_y), (32, -32));
        view.nudge_pan(1_000_000, 0);
        assert_eq!(view.pan_x, PICTURE_PAN_LIMIT);
        view.reset_view();
        assert_eq!(view, PictureViewState::default());
        assert!(view.describe().contains("side-by-side"));
    }

    #[test]
    fn picture_model_from_diff_maps_fields_and_verdict() {
        let model = PictureCompareModel::from_diff(&picture_diff(false, 1, 4, Some((1, 0, 1, 0))));
        assert_eq!((model.width_l, model.height_l), (2, 2));
        assert_eq!((model.overlap_w, model.overlap_h), (2, 2));
        assert_eq!((model.differing, model.total), (1, 4));
        assert_eq!(model.bbox, Some((1, 0, 1, 0)));
        assert!(!model.identical);
        assert!(model.status_summary().contains("1 of 4 pixels differ"));
        assert!(model.status_summary().contains("25.00%"));
        assert!(!model.lines.is_empty());
        assert!(model.summary_first_line().contains("2x2"));
        let clean = PictureCompareModel::from_diff(&picture_diff(true, 0, 4, None));
        assert!(clean.identical);
        assert_eq!(clean.status_summary(), "no differences");
        let empty = PictureCompareModel::from_diff(&picture_diff(false, 0, 0, None));
        assert!(empty.status_summary().contains("0.00%"));
    }

    #[test]
    fn picture_view_setters_report_status() {
        let mut app = App::default();
        app.set_tab(Tab::Picture);
        app.set_picture_mode(PictureMode::Overlay);
        assert_eq!(app.picture_view.mode, PictureMode::Overlay);
        assert!(app.status.contains("overlay view"));
        app.set_picture_overlay(200);
        assert_eq!(app.picture_view.overlay_mix, 100);
        app.set_picture_zoom(9);
        assert_eq!(app.picture_view.zoom_pct, PICTURE_ZOOM_MIN);
        assert!(app.status.contains("25%"));
        app.nudge_picture_pan(1, 0);
        assert_eq!(app.picture_view.pan_x, PICTURE_PAN_STEP);
        assert!(app.status.contains("picture pan"));
        app.reset_picture_view();
        assert_eq!(app.picture_view, PictureViewState::default());
        // Swap clears the model and resets the view.
        app.picture_model = Some(PictureCompareModel::from_diff(&picture_diff(
            true, 0, 1, None,
        )));
        app.set_picture_zoom(200);
        assert!(app.swap_sides());
        assert!(app.picture_model.is_none());
        assert_eq!(app.picture_view, PictureViewState::default());
    }

    #[test]
    fn picture_status_line_uses_model_verdict() {
        let mut app = App::default();
        app.set_tab(Tab::Picture);
        assert!(app.status_line().contains("no results yet"));
        app.picture_model = Some(PictureCompareModel::from_diff(&picture_diff(
            true, 0, 1, None,
        )));
        assert!(app.status_line().contains("no differences"));
        app.picture_model = Some(PictureCompareModel::from_diff(&picture_diff(
            false, 3, 4, None,
        )));
        assert!(app.status_line().contains("3 of 4 pixels differ"));
    }

    fn sync_plan_fixture() -> autocompare_compare::SyncPlan {
        autocompare_compare::SyncPlan {
            steps: vec![
                autocompare_compare::SyncStep {
                    rel_path: "new.txt".to_string(),
                    is_dir: false,
                    action: autocompare_compare::SyncAction::CopyToRight,
                },
                autocompare_compare::SyncStep {
                    rel_path: "old/".to_string(),
                    is_dir: true,
                    action: autocompare_compare::SyncAction::DeleteLeft,
                },
            ],
            skipped: vec![("same.txt".to_string(), "already in sync".to_string())],
            truncated: false,
        }
    }

    #[test]
    fn sync_checklist_from_plan_rows_glyphs_and_toggles() {
        let mut list = SyncChecklist::from_plan(&sync_plan_fixture());
        assert_eq!(list.rows.len(), 2);
        assert_eq!(list.checked_count(), 2);
        let Some(first) = list.rows.first() else {
            return;
        };
        assert_eq!(first.glyph(), '>');
        assert_eq!(first.action_label(), "copy to right");
        assert_eq!(first.display_line(), "> new.txt");
        let Some(second) = list.rows.get(1) else {
            return;
        };
        assert_eq!(second.glyph(), 'x');
        assert!(second.display_line().ends_with('/'));
        assert_eq!(list.summary(), "2 step(s) planned, 1 skipped");
        assert!(list.set_checked(0, false));
        assert_eq!(list.checked_count(), 1);
        assert!(!list.set_checked(9, true));
        list.set_all(true);
        assert_eq!(list.checked_count(), 2);
    }

    #[test]
    fn sync_status_line_uses_checklist_summary() {
        let mut app = App::default();
        app.set_tab(Tab::Sync);
        assert!(app.status_line().contains("no results yet"));
        app.sync_model = Some(SyncChecklist::from_plan(&sync_plan_fixture()));
        let line = app.status_line();
        assert!(line.contains("2 step(s) planned, 1 skipped"));
    }

    fn merge_plan_fixture() -> autocompare_compare::TreeMergePlan {
        autocompare_compare::TreeMergePlan {
            steps: vec![
                autocompare_compare::TreeMergeStep {
                    rel_path: "keep.txt".to_string(),
                    action: autocompare_compare::TreeMergeAction::TakeLeft,
                    merged: None,
                },
                autocompare_compare::TreeMergeStep {
                    rel_path: "clash.txt".to_string(),
                    action: autocompare_compare::TreeMergeAction::Conflict,
                    merged: None,
                },
            ],
            conflicts: vec!["clash.txt".to_string()],
            skipped: Vec::new(),
        }
    }

    #[test]
    fn folder_merge_checklist_flags_conflicts() {
        let mut list = FolderMergeChecklist::from_plan(&merge_plan_fixture());
        assert_eq!(list.rows.len(), 2);
        let Some(first) = list.rows.first() else {
            return;
        };
        assert_eq!(first.glyph(), 'L');
        assert_eq!(first.action_label(), "take left");
        assert!(!first.is_conflict());
        let Some(second) = list.rows.get(1) else {
            return;
        };
        assert!(second.is_conflict());
        assert_eq!(second.display_line(), "C clash.txt");
        assert_eq!(
            list.summary(),
            "2 step(s) planned, 0 omitted, 1 conflict(s)"
        );
        assert!(list.set_checked(1, false));
        assert_eq!(list.checked_count(), 1);
        assert!(!list.set_checked(7, false));
        list.set_all(false);
        assert_eq!(list.checked_count(), 0);
        let mut app = App::default();
        app.set_tab(Tab::FolderMerge);
        app.folder_merge_model = Some(list);
        assert!(app.status_line().contains("1 conflict(s)"));
    }

    fn audio_diff_fixture(
        identical: bool,
        mismatch: bool,
        first: Option<f64>,
    ) -> autocompare_diff::AudioDiff {
        autocompare_diff::AudioDiff {
            sample_rate_l: 44_100,
            sample_rate_r: if mismatch { 48_000 } else { 44_100 },
            channels_l: 1,
            channels_r: 1,
            duration_l: 1.0,
            duration_r: 1.0,
            config_mismatch: mismatch,
            first_diff_at: first,
            windows_total: 100,
            windows_differing: if identical { 0 } else { 1 },
            identical,
        }
    }

    #[test]
    fn audio_checklist_rows_cover_configs_and_verdict() {
        let list = AudioChecklist::from_diff(&audio_diff_fixture(true, false, None));
        assert_eq!(list.rows.len(), 3);
        assert!(list.identical);
        assert_eq!(list.summary(), "no differences");
        let Some(left) = list.rows.first() else {
            return;
        };
        assert_eq!(left.kind, AudioCheckKind::LeftConfig);
        assert!(left.detail.contains("44100 Hz"));
        let mismatch = AudioChecklist::from_diff(&audio_diff_fixture(false, true, None));
        assert!(mismatch.config_mismatch);
        assert!(mismatch.summary().contains("not comparable"));
        assert!(
            mismatch
                .rows
                .iter()
                .any(|row| row.detail.contains("not comparable"))
        );
        let diffed = AudioChecklist::from_diff(&audio_diff_fixture(false, false, Some(0.5)));
        assert!(diffed.summary().contains("0.500s"));
        assert!(diffed.summary().contains("1/100"));
        let mut toggled = diffed;
        assert!(toggled.set_checked(2, false));
        assert!(!toggled.set_checked(9, false));
        let mut app = App::default();
        app.set_tab(Tab::Audio);
        app.audio_model = Some(toggled);
        assert!(app.status_line().contains("0.500s"));
    }
}
