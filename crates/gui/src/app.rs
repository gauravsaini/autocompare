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

use autocompare_compare::{FolderOptions, TextOptions};
use autocompare_compare::{
    JobRunner, PictureOptions, SyncOptions, TableOptions, compare_audio_files,
    compare_binary_files, compare_folders, compare_picture_files, compare_table_files,
    compare_text_files, load_lines, plan_sync, plan_tree_merge,
};

pub use crate::viewmodel::Tab;
use crate::viewmodel::{self, FolderRow, MergeSummary, TableModel};
use crate::views;

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
    Hex(Result<Vec<String>, String>),
    Merge(Result<MergeSummary, String>),
    Table(Result<TableModel, String>),
    Sync(Result<Vec<String>, String>),
    Picture(Result<Vec<String>, String>, Option<(Vec<u8>, u32, u32)>),
    FolderMerge(Result<Vec<String>, String>),
    Audio(Result<Vec<String>, String>),
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
    pub merge_model: Option<MergeSummary>,
    pub table_model: Option<TableModel>,
    pub sync_model: Option<Vec<String>>,
    pub picture_model: Option<Vec<String>>,
    pub folder_merge_model: Option<Vec<String>>,
    pub audio_model: Option<Vec<String>>,
    pub error: Option<String>,
    heatmap: Option<TextureHandle>,
    pub dark_mode: bool,
    pub folder_expanded: BTreeSet<String>,
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
            merge_model: None,
            table_model: None,
            sync_model: None,
            picture_model: None,
            folder_merge_model: None,
            audio_model: None,
            error: None,
            heatmap: None,
            dark_mode: true,
            folder_expanded: BTreeSet::new(),
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
            Tab::Hex => match self.hex_model.as_ref() {
                Some(lines) => format!("hex: {} line(s)", lines.len()),
                None => String::from("hex: no results yet"),
            },
            Tab::Merge => match self.merge_model.as_ref() {
                Some(m) => format!(
                    "merge: {} line(s){}",
                    m.lines.len(),
                    if m.conflicted { ", conflicted" } else { "" }
                ),
                None => String::from("merge: no results yet"),
            },
            Tab::Table => match self.table_model.as_ref() {
                Some(m) => format!("table: {} row(s)", m.rows.len()),
                None => String::from("table: no results yet"),
            },
            Tab::Sync => match self.sync_model.as_ref() {
                Some(lines) => format!("sync: {} line(s)", lines.len()),
                None => String::from("sync: no results yet"),
            },
            Tab::Picture => match self.picture_model.as_ref() {
                Some(lines) => {
                    if let Some(first) = lines.first() {
                        format!("picture: {first}")
                    } else {
                        String::from("picture: no results yet")
                    }
                }
                None => String::from("picture: no results yet"),
            },
            Tab::FolderMerge => match self.folder_merge_model.as_ref() {
                Some(lines) => format!("folder-merge: {} line(s)", lines.len()),
                None => String::from("folder-merge: no results yet"),
            },
            Tab::Audio => match self.audio_model.as_ref() {
                Some(lines) => format!("audio: {} line(s)", lines.len()),
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
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let _ = JobRunner::spawn(move |_cancel, _progress| {
            let res = compare_binary_files(Path::new(&left), Path::new(&right), 50)
                .map(|cmp| viewmodel::build_hex_summary(&cmp))
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
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let _ = JobRunner::spawn(move |_cancel, _progress| {
            let res = compare_table_files(
                Path::new(&left),
                Path::new(&right),
                &TableOptions::default(),
            )
            .map(|cmp| viewmodel::build_table_rows(&cmp))
            .map_err(|err| err.to_string());
            let _ = tx.send(CompareResult::Table(res));
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
                .map(|plan| viewmodel::build_sync_summary(&plan))
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
                .map(|cmp| viewmodel::build_picture_summary(&cmp.diff))
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
                .map(|plan| viewmodel::build_folder_merge_summary(&plan))
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
                .map(|cmp| viewmodel::build_audio_summary(&cmp.diff))
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
                        Ok(model) => {
                            let n = model.len();
                            self.hex_model = Some(model);
                            self.error = None;
                            self.status = format!("hex compare done: {n} line(s)");
                        }
                        Err(err) => {
                            self.hex_model = None;
                            self.status = format!("hex compare failed: {err}");
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
                        Ok(model) => {
                            let n = model.rows.len();
                            self.table_model = Some(model);
                            self.error = None;
                            self.status = format!("table compare done: {n} row(s)");
                        }
                        Err(err) => {
                            self.table_model = None;
                            self.status = format!("table compare failed: {err}");
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Sync(res) => match res {
                        Ok(model) => {
                            let n = model.len();
                            self.sync_model = Some(model);
                            self.error = None;
                            self.status = format!("sync plan done: {n} line(s)");
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
                                let n = model.len();
                                self.picture_model = Some(model);
                                self.error = None;
                                self.status = format!("picture compare done: {n} line(s)");
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
                            let n = model.len();
                            self.folder_merge_model = Some(model);
                            self.error = None;
                            self.status = format!("folder merge done: {n} line(s)");
                        }
                        Err(err) => {
                            self.folder_merge_model = None;
                            self.status = format!("folder merge failed: {err}");
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Audio(res) => match res {
                        Ok(model) => {
                            let n = model.len();
                            self.audio_model = Some(model);
                            self.error = None;
                            self.status = format!("audio compare done: {n} line(s)");
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

        // Central panel
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            views::paint_header(ui, &mut self.dark_mode);
            views::paint_tabs(ui, &mut self.tab);
            ui.label(RichText::new(self.session_identity()).small().weak());
            ui.add_space(2.0);

            // Session toolbar: Swap Sides + Reload/Recompare.
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
            });
            // View toggles: persist in app state, affect text panes.
            ui.horizontal(|ui| {
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
            ui.add_space(4.0);

            let tab = self.tab;
            let mut compare_clicked = false;
            match tab {
                Tab::Home => {
                    views::paint_home(ui, &mut self.tab);
                    ui.add_space(8.0);
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
            ui.add_space(4.0);

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
                    if let Some((rows, summary)) = self.folder_model.as_ref() {
                        views::paint_folder(ui, rows, summary, &mut self.folder_expanded);
                    } else {
                        ui.label(
                            RichText::new(
                                "no folder results yet: type both folder paths and Compare.",
                            )
                            .weak(),
                        );
                    }
                }
                Tab::Hex => {
                    if let Some(lines) = self.hex_model.as_ref() {
                        views::paint_lines(ui, lines);
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
                    if let Some(model) = self.table_model.as_ref() {
                        views::paint_table(ui, model);
                    } else {
                        ui.label(
                            RichText::new("no table results yet: type both table paths and Compare.")
                                .weak(),
                        );
                    }
                }
                Tab::Sync => {
                    if let Some(lines) = self.sync_model.as_ref() {
                        views::paint_lines(ui, lines);
                    } else {
                        ui.label(
                            RichText::new("no sync plan yet: type both folder paths and Compare.")
                                .weak(),
                        );
                    }
                }
                Tab::Picture => {
                    if let Some(lines) = self.picture_model.as_ref() {
                        views::paint_picture(ui, lines, self.heatmap.as_ref());
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
                    if let Some(lines) = self.folder_merge_model.as_ref() {
                        views::paint_lines(ui, lines);
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
                    if let Some(lines) = self.audio_model.as_ref() {
                        views::paint_lines(ui, lines);
                    } else {
                        ui.label(
                            RichText::new("no audio results yet: type both audio paths and Compare.")
                                .weak(),
                        );
                    }
                }
            }
            ui.add_space(6.0);
            ui.separator();
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
}
