//! App state and the engine-driving logic.
//!
//! One tab is active at a time; every tab keeps its own two path
//! strings (so switching tabs preserves what was typed) and its own
//! model. The Compare button runs the engine call for the active tab
//! through the same `autocompare-compare` functions the CLI uses;
//! results replace that tab's model, failures become the red error
//! panel and clear the model. Nothing here panics.

use std::path::Path;

use eframe::egui::{self, TextureHandle};

use autocompare_compare::{
    FolderOptions, PictureOptions, TextOptions, compare_folders, compare_picture_files,
    compare_text_files,
};

use crate::viewmodel::{self, FolderRow};
use crate::views;

/// The three comparison tabs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Text,
    Folder,
    Picture,
}

/// The two path strings of one tab, persisted across tab switches.
#[derive(Debug, Default)]
struct Paths {
    left: String,
    right: String,
}

/// The GUI's whole state.
#[derive(Default)]
pub struct App {
    tab: Tab,
    text: Paths,
    folder: Paths,
    picture: Paths,
    text_model: Option<viewmodel::TextRows>,
    folder_model: Option<(Vec<FolderRow>, String)>,
    picture_model: Option<Vec<String>>,
    error: Option<String>,
    heatmap: Option<TextureHandle>,
}

impl App {
    /// Both path strings of the active tab, trimmed.
    fn paths(&self) -> (String, String) {
        let (left, right) = match self.tab {
            Tab::Text => (&self.text.left, &self.text.right),
            Tab::Folder => (&self.folder.left, &self.folder.right),
            Tab::Picture => (&self.picture.left, &self.picture.right),
        };
        (left.trim().to_string(), right.trim().to_string())
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
                Tab::Picture => views::paint_paths(
                    ui,
                    "picture",
                    &mut self.picture.left,
                    &mut self.picture.right,
                    &mut compare_clicked,
                ),
            }
            if compare_clicked {
                match tab {
                    Tab::Text => self.run_text(),
                    Tab::Folder => self.run_folder(),
                    Tab::Picture => self.run_picture(ui.ctx()),
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
                Tab::Picture => {
                    if let Some(lines) = self.picture_model.as_ref() {
                        views::paint_picture(ui, lines, self.heatmap.as_ref());
                    }
                }
            }
        });
    }
}
