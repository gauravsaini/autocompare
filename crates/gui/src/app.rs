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
            return false;
        }
        true
    }

    /// Rejects an empty side of a three-way tab before the engine runs.
    fn require_triple(&mut self, base: &str, left: &str, right: &str, noun: &str) -> bool {
        if base.is_empty() || left.is_empty() || right.is_empty() {
            self.error = Some(format!("type the base and both {noun} paths first"));
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
        // Drag and drop handling
        let dropped_files = ui.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            let paths: Vec<String> = dropped_files
                .into_iter()
                .map(|d| d.path().display().to_string())
                .collect();
            self.apply_dropped_paths(&paths);
        }

        // Process background comparison completion
        if let Some(rx) = &self.rx {
            if let Ok(res) = rx.try_recv() {
                match res {
                    CompareResult::Text(res) => match res {
                        Ok(model) => {
                            self.text_model = Some(model);
                            self.error = None;
                        }
                        Err(err) => {
                            self.text_model = None;
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Folder(res) => match res {
                        Ok(model) => {
                            self.folder_model = Some(model);
                            self.error = None;
                        }
                        Err(err) => {
                            self.folder_model = None;
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Hex(res) => match res {
                        Ok(model) => {
                            self.hex_model = Some(model);
                            self.error = None;
                        }
                        Err(err) => {
                            self.hex_model = None;
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Merge(res) => match res {
                        Ok(model) => {
                            self.merge_model = Some(model);
                            self.error = None;
                        }
                        Err(err) => {
                            self.merge_model = None;
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Table(res) => match res {
                        Ok(model) => {
                            self.table_model = Some(model);
                            self.error = None;
                        }
                        Err(err) => {
                            self.table_model = None;
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Sync(res) => match res {
                        Ok(model) => {
                            self.sync_model = Some(model);
                            self.error = None;
                        }
                        Err(err) => {
                            self.sync_model = None;
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Picture(res, hm) => {
                        match res {
                            Ok(model) => {
                                self.picture_model = Some(model);
                                self.error = None;
                            }
                            Err(err) => {
                                self.picture_model = None;
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
                            self.folder_merge_model = Some(model);
                            self.error = None;
                        }
                        Err(err) => {
                            self.folder_merge_model = None;
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Audio(res) => match res {
                        Ok(model) => {
                            self.audio_model = Some(model);
                            self.error = None;
                        }
                        Err(err) => {
                            self.audio_model = None;
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

            let tab = self.tab;
            let mut compare_clicked = false;
            match tab {
                Tab::Home => views::paint_home(ui, &mut self.tab),
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
                match tab {
                    Tab::Home => {}
                    Tab::Text => self.run_text(),
                    Tab::Folder => self.run_folder(),
                    Tab::Hex => self.run_hex(),
                    Tab::Merge => self.run_merge(),
                    Tab::Table => self.run_table(),
                    Tab::Sync => self.run_sync(),
                    Tab::Picture => self.run_picture(),
                    Tab::FolderMerge => self.run_folder_merge(),
                    Tab::Audio => self.run_audio(),
                }
            }

            if let Some(err) = self.error.as_deref() {
                views::paint_error(ui, err);
            }
            ui.add_space(4.0);

            match tab {
                Tab::Home => {}
                Tab::Text => {
                    if let Some(model) = self.text_model.as_mut() {
                        views::paint_text(ui, model);
                    }
                }
                Tab::Folder => {
                    if let Some((rows, summary)) = self.folder_model.as_ref() {
                        views::paint_folder(ui, rows, summary, &mut self.folder_expanded);
                    }
                }
                Tab::Hex => {
                    if let Some(lines) = self.hex_model.as_ref() {
                        views::paint_lines(ui, lines);
                    }
                }
                Tab::Merge => {
                    if let Some(model) = self.merge_model.as_mut() {
                        views::paint_merge(ui, model);
                    }
                }
                Tab::Table => {
                    if let Some(model) = self.table_model.as_ref() {
                        views::paint_table(ui, model);
                    }
                }
                Tab::Sync => {
                    if let Some(lines) = self.sync_model.as_ref() {
                        views::paint_lines(ui, lines);
                    }
                }
                Tab::Picture => {
                    if let Some(lines) = self.picture_model.as_ref() {
                        views::paint_picture(ui, lines, self.heatmap.as_ref());
                    }
                }
                Tab::FolderMerge => {
                    if let Some(lines) = self.folder_merge_model.as_ref() {
                        views::paint_lines(ui, lines);
                    }
                }
                Tab::Audio => {
                    if let Some(lines) = self.audio_model.as_ref() {
                        views::paint_lines(ui, lines);
                    }
                }
            }
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
}
