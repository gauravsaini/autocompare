//! App state and the engine-driving logic.
//!
//! One tab is active at a time; every tab keeps its own path strings
//! (so switching tabs preserves what was typed) and its own model.
//! The Compare button runs the engine call for the active tab through
//! the same `autocompare-compare` functions the CLI uses; results
//! replace that tab's model, failures become the red error panel and
//! clear the model. Nothing here panics.

use std::path::Path;

use eframe::egui::{self, TextureHandle};

use autocompare_compare::{FolderOptions, TextOptions};
use autocompare_compare::{
    PictureOptions, SyncOptions, TableOptions, compare_audio_files, compare_binary_files,
    compare_folders, compare_picture_files, compare_table_files, compare_text_files, load_lines,
    plan_sync, plan_tree_merge,
};

use crate::viewmodel::{self, FolderRow, MergeSummary, TableModel};
use crate::views;

/// The nine comparison tabs, matching the nine CLI commands and the
/// original's nine session types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Text,
    Folder,
    Hex,
    Merge,
    Table,
    Sync,
    Picture,
    FolderMerge,
    Audio,
}

/// The two path strings of one tab, persisted across tab switches.
#[derive(Debug, Default)]
struct Paths {
    left: String,
    right: String,
}

/// The three path strings of a three-way tab.
#[derive(Debug, Default)]
struct TriplePaths {
    base: String,
    left: String,
    right: String,
}

/// The GUI's whole state.
#[derive(Default)]
pub struct App {
    tab: Tab,
    text: Paths,
    folder: Paths,
    hex: Paths,
    merge: TriplePaths,
    table: Paths,
    sync: Paths,
    picture: Paths,
    folder_merge: TriplePaths,
    audio: Paths,
    text_model: Option<viewmodel::TextRows>,
    folder_model: Option<(Vec<FolderRow>, String)>,
    hex_model: Option<Vec<String>>,
    merge_model: Option<MergeSummary>,
    table_model: Option<TableModel>,
    sync_model: Option<Vec<String>>,
    picture_model: Option<Vec<String>>,
    folder_merge_model: Option<Vec<String>>,
    audio_model: Option<Vec<String>>,
    error: Option<String>,
    heatmap: Option<TextureHandle>,
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
            Tab::Merge | Tab::FolderMerge => return (String::new(), String::new()),
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
        match compare_text_files(Path::new(&left), Path::new(&right), &TextOptions::default()) {
            Ok(cmp) => {
                self.error = None;
                self.text_model = Some(viewmodel::build_text_rows(&cmp));
            }
            Err(err) => {
                self.text_model = None;
                self.error = Some(err.to_string());
            }
        }
    }

    fn run_folder(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "folder") {
            self.folder_model = None;
            return;
        }
        match compare_folders(
            Path::new(&left),
            Path::new(&right),
            &FolderOptions::default(),
        ) {
            Ok(cmp) => {
                self.error = None;
                self.folder_model = Some(viewmodel::build_folder_rows(&cmp, cmp.truncated));
            }
            Err(err) => {
                self.folder_model = None;
                self.error = Some(err.to_string());
            }
        }
    }

    fn run_hex(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "file") {
            self.hex_model = None;
            return;
        }
        match compare_binary_files(Path::new(&left), Path::new(&right), 50) {
            Ok(cmp) => {
                self.error = None;
                self.hex_model = Some(viewmodel::build_hex_summary(&cmp));
            }
            Err(err) => {
                self.hex_model = None;
                self.error = Some(err.to_string());
            }
        }
    }

    fn run_merge(&mut self) {
        let (base, left, right) = self.triple_paths();
        if !self.require_triple(&base, &left, &right, "file") {
            self.merge_model = None;
            return;
        }
        let base_lines = match load_lines(Path::new(&base)) {
            Ok(lines) => lines,
            Err(err) => {
                self.merge_model = None;
                self.error = Some(err.to_string());
                return;
            }
        };
        let left_lines = match load_lines(Path::new(&left)) {
            Ok(lines) => lines,
            Err(err) => {
                self.merge_model = None;
                self.error = Some(err.to_string());
                return;
            }
        };
        let right_lines = match load_lines(Path::new(&right)) {
            Ok(lines) => lines,
            Err(err) => {
                self.merge_model = None;
                self.error = Some(err.to_string());
                return;
            }
        };
        let base_refs: Vec<&str> = base_lines.iter().map(String::as_str).collect();
        let left_refs: Vec<&str> = left_lines.iter().map(String::as_str).collect();
        let right_refs: Vec<&str> = right_lines.iter().map(String::as_str).collect();
        let merged = autocompare_diff::merge3(&base_refs, &left_refs, &right_refs);
        self.error = None;
        self.merge_model = Some(viewmodel::build_merge_summary(&merged));
    }

    fn run_table(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "table") {
            self.table_model = None;
            return;
        }
        match compare_table_files(
            Path::new(&left),
            Path::new(&right),
            &TableOptions::default(),
        ) {
            Ok(cmp) => {
                self.error = None;
                self.table_model = Some(viewmodel::build_table_rows(&cmp));
            }
            Err(err) => {
                self.table_model = None;
                self.error = Some(err.to_string());
            }
        }
    }

    fn run_sync(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "folder") {
            self.sync_model = None;
            return;
        }
        match plan_sync(Path::new(&left), Path::new(&right), &SyncOptions::default()) {
            Ok(plan) => {
                self.error = None;
                self.sync_model = Some(viewmodel::build_sync_summary(&plan));
            }
            Err(err) => {
                self.sync_model = None;
                self.error = Some(err.to_string());
            }
        }
    }

    fn run_picture(&mut self, ctx: &egui::Context) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "picture") {
            self.picture_model = None;
            self.heatmap = None;
            return;
        }
        // Stats first. The engine's heatmap-file option stays off:
        // the GUI never writes files; pixels are built in memory
        // below.
        let opts = PictureOptions::default();
        match compare_picture_files(Path::new(&left), Path::new(&right), &opts) {
            Ok(cmp) => {
                self.error = None;
                self.picture_model = Some(viewmodel::build_picture_summary(&cmp.diff));
            }
            Err(err) => {
                self.picture_model = None;
                self.heatmap = None;
                self.error = Some(err.to_string());
                return;
            }
        }
        // Best-effort heatmap: decode both PNGs again (the engine
        // already decoded them internally; the duplicate decode is
        // the accepted price of getting pixels into the GUI without
        // an engine API change - file sizes are modest) and turn the
        // overlap heatmap into a texture.
        match load_heatmap_rgba(Path::new(&left), Path::new(&right)) {
            Ok((bytes, width, height)) => match views::heatmap_color_image(&bytes, width, height) {
                Some(image) => {
                    // Replacing the Option drops the old handle, so
                    // repeated compares reuse no stale texture.
                    self.heatmap = Some(ctx.load_texture(
                        "autocompare-heatmap",
                        image,
                        egui::TextureOptions::default(),
                    ));
                }
                None => {
                    self.heatmap = None;
                    self.error =
                        Some("the heatmap is empty; there is nothing to display".to_string());
                }
            },
            Err(msg) => {
                self.heatmap = None;
                self.error = Some(format!("heatmap unavailable: {msg}"));
            }
        }
    }

    fn run_folder_merge(&mut self) {
        let (base, left, right) = self.triple_paths();
        if !self.require_triple(&base, &left, &right, "folder") {
            self.folder_merge_model = None;
            return;
        }
        match plan_tree_merge(Path::new(&base), Path::new(&left), Path::new(&right)) {
            Ok(plan) => {
                self.error = None;
                self.folder_merge_model = Some(viewmodel::build_folder_merge_summary(&plan));
            }
            Err(err) => {
                self.folder_merge_model = None;
                self.error = Some(err.to_string());
            }
        }
    }

    fn run_audio(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "audio") {
            self.audio_model = None;
            return;
        }
        match compare_audio_files(Path::new(&left), Path::new(&right)) {
            Ok(cmp) => {
                self.error = None;
                self.audio_model = Some(viewmodel::build_audio_summary(&cmp.diff));
            }
            Err(err) => {
                self.audio_model = None;
                self.error = Some(err.to_string());
            }
        }
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
        // The framework hands over the central ui without a
        // background; wrap in the standard central-panel frame so the
        // window paints like a native one.
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            views::paint_header(ui);
            views::paint_tabs(ui, &mut self.tab);

            let tab = self.tab;
            let mut compare_clicked = false;
            match tab {
                Tab::Text => views::paint_paths(
                    ui,
                    "file",
                    &mut self.text.left,
                    &mut self.text.right,
                    &mut compare_clicked,
                ),
                Tab::Folder => views::paint_paths(
                    ui,
                    "folder",
                    &mut self.folder.left,
                    &mut self.folder.right,
                    &mut compare_clicked,
                ),
                Tab::Hex => views::paint_paths(
                    ui,
                    "file",
                    &mut self.hex.left,
                    &mut self.hex.right,
                    &mut compare_clicked,
                ),
                Tab::Merge => views::paint_triple_paths(
                    ui,
                    "file",
                    &mut self.merge.base,
                    &mut self.merge.left,
                    &mut self.merge.right,
                    &mut compare_clicked,
                ),
                Tab::Table => views::paint_paths(
                    ui,
                    "table",
                    &mut self.table.left,
                    &mut self.table.right,
                    &mut compare_clicked,
                ),
                Tab::Sync => views::paint_paths(
                    ui,
                    "folder",
                    &mut self.sync.left,
                    &mut self.sync.right,
                    &mut compare_clicked,
                ),
                Tab::Picture => views::paint_paths(
                    ui,
                    "picture",
                    &mut self.picture.left,
                    &mut self.picture.right,
                    &mut compare_clicked,
                ),
                Tab::FolderMerge => views::paint_triple_paths(
                    ui,
                    "folder",
                    &mut self.folder_merge.base,
                    &mut self.folder_merge.left,
                    &mut self.folder_merge.right,
                    &mut compare_clicked,
                ),
                Tab::Audio => views::paint_paths(
                    ui,
                    "audio",
                    &mut self.audio.left,
                    &mut self.audio.right,
                    &mut compare_clicked,
                ),
            }
            if compare_clicked {
                match tab {
                    Tab::Text => self.run_text(),
                    Tab::Folder => self.run_folder(),
                    Tab::Hex => self.run_hex(),
                    Tab::Merge => self.run_merge(),
                    Tab::Table => self.run_table(),
                    Tab::Sync => self.run_sync(),
                    Tab::Picture => self.run_picture(ui.ctx()),
                    Tab::FolderMerge => self.run_folder_merge(),
                    Tab::Audio => self.run_audio(),
                }
            }
            if let Some(err) = self.error.as_deref() {
                views::paint_error(ui, err);
            }
            ui.add_space(4.0);
            match tab {
                Tab::Text => {
                    if let Some(model) = self.text_model.as_ref() {
                        views::paint_text(ui, model);
                    }
                }
                Tab::Folder => {
                    if let Some((rows, summary)) = self.folder_model.as_ref() {
                        views::paint_folder(ui, rows, summary);
                    }
                }
                Tab::Hex => {
                    if let Some(lines) = self.hex_model.as_ref() {
                        views::paint_lines(ui, lines);
                    }
                }
                Tab::Merge => {
                    if let Some(model) = self.merge_model.as_ref() {
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
