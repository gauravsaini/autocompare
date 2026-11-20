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
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, channel};

use eframe::egui::{self, RichText, TextureHandle};

use autocompare_compare::{FolderOptions, SyncAction, TextOptions};
use autocompare_compare::{
    JobHandle, JobProgress, JobRunner, PictureOptions, SyncDirection, SyncOptions, SyncPlan,
    SyncReport, TableOptions, TreeMergeAction, TreeMergePlan, apply_sync, compare_audio_files,
    compare_binary_files_aligned, compare_folders_cancellable, compare_picture_files,
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
    Hex(Result<(Vec<String>, views_hex::HexCompareView, Option<HexByteSnap>), String>),
    Merge(Result<MergeSummary, String>),
    Table(Result<(TableModel, views_table::TableKeySummary), String>),
    Sync(Result<SyncChecklist, String>),
    Picture(
        Result<PictureCompareModel, String>,
        Option<(Vec<u8>, u32, u32)>,
    ),
    FolderMerge(Result<FolderMergeOutcome, String>),
    Audio(Result<AudioChecklist, String>),
}

/// Merge Apply action ka progress/result state (Gap #8 state half).
/// Paint agle wave me isi se Apply section banayega: Idle se start,
/// har `run_merge_apply` branch ek honest terminal state deta hai.
/// `Copy` + `Default(Idle)` taaki headless tests saste rahen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MergeApplyState {
    /// Koi apply report abhi tak nahi (fresh compare / swap / error).
    #[default]
    Idle,
    /// Dry-run report bana, kuch likha nahi.
    DryRunReported,
    /// Real write ho gaya, `wrote_lines` summary me.
    Wrote,
    /// Dry-run off par output path khali tha.
    NeedsPath,
    /// Model bina compare ke apply manga gaya.
    NoModel,
    /// Resolve ya I/O fail, model untouched (retry safe).
    Failed,
}

/// Last merge apply ka structured summary (Gap #8 state half).
/// `merge_apply_last_report` string line ka judvaan: ye ginti rakhta
/// hai taaki paint agle wave me real numbers dikhaye, jhooth kabhi nahi.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeApplySummary {
    /// Dry-run tha (true) ya real write (false).
    pub dry_run: bool,
    /// Resolved output lines (markers samait jab pending ho).
    pub resolved_lines: usize,
    /// Staged picks se decided hunks.
    pub decided: usize,
    /// Live lines me kul conflict hunks.
    pub total: usize,
    /// Abhi bhi undecided hunks.
    pub pending: usize,
    /// Real write par likhi lines (dry-run par 0).
    pub wrote_lines: usize,
    /// Output path (dry-run/no-model par khali ho sakta hai).
    pub out: String,
}

/// App-owned hex byte snapshot behind the nibble-accent callsite.
///
/// Engine ka `BinaryComparison` sirf badle runs rakhta hai (offset/len),
/// isliye `views_hex::HexCompareView` me byte values nahi hain. Ye snap
/// dono inputs ko aligned engine cap tak rakhta hai taaki paint
/// `hex_diff_nibble_cells(row, left, right)` me REAL slices de sake
/// (absolute-offset indexing ke liye poori-file slices chahiye).
/// Caller-side ownership: App rakhta hai, `paint_hex` sirf udhaar leta
/// hai (`views_hex.rs` FROZEN, koi change nahi).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HexByteSnap {
    /// Left input bytes, aligned cap tak.
    pub left: Vec<u8>,
    /// Right input bytes, aligned cap tak.
    pub right: Vec<u8>,
}

impl HexByteSnap {
    /// Callsite slices: `paint_hex` inhe `hex_diff_nibble_cells` me deta
    /// hai. Khali snap par khali slices (purana offset-only behavior).
    /// Pure; unit-tested.
    pub fn as_slices(&self) -> (&[u8], &[u8]) {
        (self.left.as_slice(), self.right.as_slice())
    }
}

/// Ek hex input ko aligned engine cap tak padhta hai. Missing /
/// unreadable / over-cap input par khali (paint offset-only marks par
/// rehta hai — purana behavior, kabhi jhooth nahi). No panic.
fn read_hex_input_capped(path: &Path) -> Vec<u8> {
    let cap = views_hex::ALIGNED_HEX_MAX_BYTES;
    let too_big = std::fs::metadata(path)
        .map(|meta| meta.len() > cap as u64)
        .unwrap_or(false);
    if too_big {
        return Vec::new();
    }
    let mut bytes = std::fs::read(path).unwrap_or_default();
    bytes.truncate(cap);
    bytes
}

/// Dono hex inputs ka callsite snap. Compare safal hone par banta hai;
/// dobara padhne me nakam file khali rehti hai (TOCTOU par accent nahi,
/// grid marks rehte hain). No panic.
fn read_hex_inputs_capped(left: &Path, right: &Path) -> Option<HexByteSnap> {
    Some(HexByteSnap {
        left: read_hex_input_capped(left),
        right: read_hex_input_capped(right),
    })
}

/// The GUI's whole state.
pub struct App {
    pub tab: Tab,
    /// Live multi-session store (BW-SES-APP bridge): har khula compare ek
    /// session hai. `current_tab` active session ka kind deta hai, khaali
    /// par legacy `tab` (Home). Model API (`home::SessionList` T1-T7)
    /// read-only use hota hai, yahan sirf bridge + slots sync rehta hai.
    pub sessions: home::SessionList,
    /// Session-start recent list (Home): `open_home_session` yahan record
    /// karta hai, naya sabse upar, cap [`home::HOME_RECENT_MAX`] tak.
    pub home_recent: home::RecentSessions,
    /// Session restore history: last state (tab, paths, cursor) snapshots,
    /// naya sabse upar. `home_recent` titles paint-compat ke liye rehta hai;
    /// ye reopen ke liye full state deta hai. Empty/missing paths graceful.
    pub session_history: home::SessionHistory,
    /// Kya recent list clear hui hai (fresh start nahi). Dono clear actions
    /// ise true karte hain, naya record false karta hai. Paint isi se khali
    /// list ka sahi hint dikhata hai.
    pub recent_cleared: bool,
    text: Paths,
    folder: Paths,
    hex: Paths,
    merge: TriplePaths,
    table: Paths,
    sync: Paths,
    picture: Paths,
    folder_merge: TriplePaths,
    audio: Paths,
    /// Compare hone wale paths ka dropdown data (BU-PATHMETA App half):
    /// non-empty trimmed paths, naya sabse upar, dedupe + cap
    /// [`home::HOME_RECENT_MAX`] tak. Paint sirf labels dikhata hai,
    /// selection kabhi nahi badalti. Per-run only.
    pub path_history: Vec<String>,
    pub text_model: Option<viewmodel::TextRows>,
    pub folder_model: Option<(Vec<FolderRow>, String)>,
    pub hex_model: Option<Vec<String>>,
    pub hex_view: views_hex::HexCompareView,
    /// Nibble-accent callsite ka App-owned byte snap (`paint_hex` ko
    /// udhaar slices; `None` = offset-only marks, purana behavior).
    /// Naya compare / swap / mode / error par clear hota hai.
    pub hex_bytes: Option<HexByteSnap>,
    pub merge_model: Option<MergeSummary>,
    /// App-owned per-hunk staged merge picks (Gap #13 App half).
    /// AI-MERGEPICK temp-store intent ka App truth: `decide_merge_hunk`
    /// yahi persist karta hai, summary yahi se ginta hai. Naya compare /
    /// swap / error par clear hota hai; file apply (I/O) `run_merge_apply`
    /// se hota hai (dry-run default on).
    pub merge_picks: Vec<viewmodel::MergeHunkPick>,
    /// Merge Apply gate (Gap #7/#8 App half): dry-run default on (no
    /// writes), output path khali (user type kare), last report line
    /// `None` jab tak dry-run ya real apply report na de. Naya compare /
    /// swap / error par clear hota hai. Paint wiring agle wave ka hai.
    pub merge_dry_run: bool,
    /// Merge output file path (Apply yahi likhta hai).
    pub merge_output_path: String,
    /// Last merge dry-run ya real apply outcome line.
    /// `None` until a dry-run or real apply reports one.
    pub merge_apply_last_report: Option<String>,
    /// Apply button intent (Gap #8 state half): paint Apply click par
    /// `request_merge_apply` se true karta hai, `run_merge_apply` har
    /// branch me consume karke false karta hai. Naya compare / swap /
    /// error par clear hota hai. Paint adoption agle wave ka hai.
    pub merge_apply_requested: bool,
    /// Last apply action ka progress/result state. `Idle` jab tak koi
    /// report na bane; har `run_merge_apply` branch honest state deta hai.
    pub merge_apply_state: MergeApplyState,
    /// Last apply ka structured summary (real counts). `None` jab tak
    /// koi report na bane; naya compare / swap / error par clear.
    pub merge_apply_last_summary: Option<MergeApplySummary>,
    pub table_model: Option<TableModel>,
    pub table_view: views_table::TableKeyView,
    pub table_tolerance_text: String,
    pub sync_model: Option<SyncChecklist>,
    /// Sync-tab Apply gate: dry-run default on (no writes), explicit
    /// confirm off. One direction only (checked left->right copies).
    pub sync_dry_run: bool,
    pub sync_confirmed: bool,
    pub sync_last_report: Option<String>,
    pub picture_model: Option<PictureCompareModel>,
    pub picture_view: PictureViewState,
    pub folder_merge_model: Option<FolderMergeChecklist>,
    /// FolderMerge tree ke teen presence snapshots (base/left/right).
    /// Worker har safal plan ke saath banata hai; rows
    /// `tree::three_way_tree_rows` isi se bante hain. Naya compare /
    /// swap / error par clear hote hain (model ke saath).
    pub folder_merge_base: Vec<FolderRow>,
    /// Left root ka presence snapshot (tree rows ke liye).
    pub folder_merge_left: Vec<FolderRow>,
    /// Right root ka presence snapshot (tree rows ke liye).
    pub folder_merge_right: Vec<FolderRow>,
    /// Tree par staged Take decisions (last-wins). Naya compare / swap /
    /// error par clear hoti hain; paint `views::paint_three_way_tree`
    /// click par isi me likhta hai.
    pub folder_merge_decisions: Vec<tree::ThreeWayDecision>,
    pub audio_model: Option<AudioChecklist>,
    pub error: Option<String>,
    heatmap: Option<TextureHandle>,
    pub dark_mode: bool,
    pub folder_expanded: BTreeSet<String>,
    pub folder_session: tree::FolderSessionState,
    pub hex_pager: views_hex::HexPager,
    pub table_pager: views_table::TablePager,
    pub folder_pager: tree::RowPager,
    /// Shared select + scroll + stub-action state behind the dual-tree
    /// folder panes (`views::paint_folder_dual`). Page size mirrors
    /// `folder_pager` (20); totals sync from compare results.
    pub folder_dual: tree::DualTreeState,
    /// Dry-run gate behind Folder Apply (Gap #8 kadam 1): default true
    /// means Apply only reports what would copy, never writes. User
    /// must explicitly uncheck to allow real writes, plus keep the
    /// existing consent checkbox on. No destructive default.
    pub folder_sync_dry_run: bool,
    /// Last mirror-copy outcome line for the Folder Apply section.
    /// `None` until a dry-run or real apply reports one.
    pub folder_sync_last_report: Option<String>,
    /// Multi-select model behind folder sync direction actions: kaunsi
    /// rows par copy/prune direction lagna hai. Paint adoption agle
    /// wave me; is wave me sirf headless wiring + scoped dry-run.
    pub folder_sync_selection: tree::FolderSyncSelection,
    pub hex_gutter_row_text: String,
    pub table_find_needle: String,
    pub table_find_from: usize,
    /// Table find N/M cursor: `table_find_matches` list me index.
    /// Text `text_find_cursor` precedent mirror; legacy `table_find_from`
    /// (row-based) compat ke liye rehta hai.
    pub table_find_cursor: usize,
    pub folder_find_needle: String,
    pub folder_cursor: usize,
    pub text_find_needle: String,
    pub text_find_cursor: usize,
    pub text_find_ignore_case: bool,
    /// Find-box state behind the Text tab paint (`views::paint_text_with_find`).
    /// Legacy `text_find_needle` fields compat ke liye rehte hain; paint ka
    /// source of truth ye hai (N/M badge `count_label` se aata hai).
    pub text_find: viewmodel::FindState,
    pub hex_find_needle: String,
    pub hex_find_cursor: u64,
    pub word_wrap: bool,
    pub show_line_numbers: bool,
    pub syntax_highlight: bool,
    /// Font zoom step into `crate::theme::ZOOM_LEVELS` (per-run only).
    pub zoom_level: i8,
    pub status: String,
    /// Draggable pane split fraction (left share of the row).
    /// Clamped to 0.20..=0.80, default 0.50; per-run only.
    pub split_fraction: f32,
    /// Running background job behind `rx` (cancel + progress poll).
    /// `None` when idle; cleared when the result lands or is
    /// cancelled. The thread always ends via the channel send, so
    /// dropping the handle never leaks a live worker.
    job: Option<JobHandle<()>>,
    /// Last shown progress fraction 0.0..=1.0 (`None` = indeterminate
    /// or idle). Painted in the status bar; updates pass the
    /// [`App::progress_should_emit`] throttle so per-chunk worker
    /// writes never become per-byte repaints.
    pub progress_fraction: Option<f32>,
    /// Last shown worker message behind [`App::progress_fraction`].
    /// Display still reuses [`App::status`]; this only feeds the
    /// throttle gate so a repeated message never repaints.
    progress_message: Option<String>,
    rx: Option<Receiver<CompareResult>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            tab: Tab::Home,
            sessions: home::SessionList::new(),
            home_recent: home::RecentSessions::new(),
            session_history: home::SessionHistory::new(),
            recent_cleared: false,
            text: Paths::default(),
            folder: Paths::default(),
            hex: Paths::default(),
            merge: TriplePaths::default(),
            table: Paths::default(),
            sync: Paths::default(),
            picture: Paths::default(),
            folder_merge: TriplePaths::default(),
            audio: Paths::default(),
            path_history: Vec::new(),
            text_model: None,
            folder_model: None,
            hex_model: None,
            hex_view: views_hex::HexCompareView::empty(
                views_hex::HexCompareMode::default(),
                views_hex::HEX_DEFAULT_MAX_HUNKS,
            ),
            hex_bytes: None,
            merge_model: None,
            merge_picks: Vec::new(),
            merge_dry_run: true,
            merge_output_path: String::new(),
            merge_apply_last_report: None,
            merge_apply_requested: false,
            merge_apply_state: MergeApplyState::Idle,
            merge_apply_last_summary: None,
            table_model: None,
            table_view: views_table::TableKeyView::new(),
            table_tolerance_text: String::new(),
            sync_model: None,
            sync_dry_run: true,
            sync_confirmed: false,
            sync_last_report: None,
            picture_model: None,
            picture_view: PictureViewState::default(),
            folder_merge_model: None,
            folder_merge_base: Vec::new(),
            folder_merge_left: Vec::new(),
            folder_merge_right: Vec::new(),
            folder_merge_decisions: Vec::new(),
            audio_model: None,
            error: None,
            heatmap: None,
            dark_mode: true,
            folder_expanded: BTreeSet::new(),
            folder_session: tree::FolderSessionState::default(),
            hex_pager: views_hex::HexPager::new(0, 16),
            table_pager: views_table::TablePager::new(0, 20),
            folder_pager: tree::RowPager::new(0, 20),
            folder_dual: tree::DualTreeState::new(20),
            folder_sync_dry_run: true,
            folder_sync_last_report: None,
            folder_sync_selection: tree::FolderSyncSelection::new(),
            hex_gutter_row_text: String::new(),
            table_find_needle: String::new(),
            table_find_from: 0,
            table_find_cursor: 0,
            folder_find_needle: String::new(),
            folder_cursor: 0,
            text_find_needle: String::new(),
            text_find_cursor: 0,
            text_find_ignore_case: true,
            text_find: viewmodel::FindState::default(),
            hex_find_needle: String::new(),
            hex_find_cursor: 0,
            word_wrap: false,
            show_line_numbers: true,
            syntax_highlight: true,
            zoom_level: crate::theme::ZOOM_DEFAULT_LEVEL,
            status: String::from("ready: pick a session to begin"),
            split_fraction: views::SPLIT_DEFAULT_FRACTION,
            job: None,
            progress_fraction: None,
            progress_message: None,
            rx: None,
        }
    }
}

/// Font-zoom key decision behind Cmd/Ctrl + `=`/`-`/`0` (headless,
/// unit-tested). Reset wins jab `0` +/- ke saath chorded ho; kuch na
/// dabane par [`ZoomKey::None`]. `eframe::App::ui` isko `ui.input` se
/// feed karta hai; paint yahi se chalta hai.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomKey {
    /// Zoom in (Cmd/Ctrl + `=` / `+`).
    In,
    /// Zoom out (Cmd/Ctrl + `-`).
    Out,
    /// Reset to default (Cmd/Ctrl + `0`).
    Reset,
    /// No zoom key pressed.
    None,
}

/// CN-OPENER: non-interactive `--open <left> <right> [--tab <name>]`
/// launch request. Dono path session me preload hote hain aur tab khulta
/// hai taaki live captures me content rows photo-proof ho saken
/// (keyboard injection/drag-drop is env me block hai, ye flag hi
/// input-gate ka hal hai). Sirf wiring (`set_tab` + `run_active_tab`
/// reuse); colors/geometry/typo untouched. Headless-tested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRequest {
    /// Left input path.
    pub left: String,
    /// Right input path.
    pub right: String,
    /// Kaunsa tab khulna hai (`--tab` se, default Text).
    pub tab: Tab,
}

impl App {
    /// Resolves one zoom-key frame: reset sabse pehle, phir in, phir out.
    /// Pure; headless-tested.
    pub fn resolve_zoom_key(zoom_in: bool, zoom_out: bool, zoom_reset: bool) -> ZoomKey {
        if zoom_reset {
            ZoomKey::Reset
        } else if zoom_in {
            ZoomKey::In
        } else if zoom_out {
            ZoomKey::Out
        } else {
            ZoomKey::None
        }
    }

    /// Applies one resolved zoom key. Returns true jab level badla ho
    /// (In/Out/Reset), false par [`ZoomKey::None`]. Headless-testable
    /// core behind the paint-time Cmd/Ctrl + +/-/0 binding.
    pub fn apply_zoom_key(&mut self, action: ZoomKey) -> bool {
        match action {
            ZoomKey::In => {
                self.zoom_in();
                true
            }
            ZoomKey::Out => {
                self.zoom_out();
                true
            }
            ZoomKey::Reset => {
                self.zoom_reset();
                true
            }
            ZoomKey::None => false,
        }
    }

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
        self.save_slots_into_active();
    }

    /// Returns current text paths.
    pub fn text_paths(&self) -> (String, String) {
        (self.text.left.clone(), self.text.right.clone())
    }

    /// Sets folder paths.
    pub fn set_folder_paths(&mut self, left: impl Into<String>, right: impl Into<String>) {
        self.folder.left = left.into();
        self.folder.right = right.into();
        self.save_slots_into_active();
    }

    /// Returns current folder paths.
    pub fn folder_paths(&self) -> (String, String) {
        (self.folder.left.clone(), self.folder.right.clone())
    }

    /// Path history dropdown data: naya-sabse-pehle order me recent paths.
    /// Paint ([`views::paint_path_meta_row`]) sirf labels dikhata hai,
    /// selection kabhi nahi badalti. Pure; headless-tested.
    pub fn path_history_items(&self) -> &[String] {
        self.path_history.as_slice()
    }

    /// History me paths record karta hai: khali girte hain, same path
    /// dedupe hokar upar aata hai, cap [`home::HOME_RECENT_MAX`] tak.
    /// Pure state; headless-tested.
    pub fn record_path_history(&mut self, paths: &[&str]) {
        for path in paths {
            let trimmed = path.trim();
            if trimmed.is_empty() {
                continue;
            }
            self.path_history.retain(|existing| existing != trimmed);
            self.path_history.insert(0, trimmed.to_string());
        }
        self.path_history.truncate(home::HOME_RECENT_MAX);
    }

    /// File byte-size label: missing/khali/unreadable par khali (koi fake
    /// "0 B" nahi). No panic.
    pub fn path_byte_label(path: &str) -> String {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        match std::fs::metadata(trimmed) {
            Ok(meta) => format!("{} B", meta.len()),
            Err(_) => String::new(),
        }
    }

    /// File timestamp label: modified epoch secs, unknown par khali (koi
    /// fake "just now" nahi). No panic.
    pub fn path_time_label(path: &str) -> String {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        match std::fs::metadata(trimmed) {
            Ok(meta) => match meta.modified() {
                Ok(modified) => match modified.duration_since(std::time::UNIX_EPOCH) {
                    Ok(age) => format!("modified @{}s", age.as_secs()),
                    Err(_) => String::new(),
                },
                Err(_) => String::new(),
            },
            Err(_) => String::new(),
        }
    }

    /// Active tab ka rules label (BC meta row ka rules hissa, apne shabdon
    /// me). Home par khali. Pure; headless-tested.
    pub fn path_rules_label(&self) -> &'static str {
        match self.tab {
            Tab::Text => "Text: exact",
            Tab::Folder => "Folder: name+size",
            Tab::Hex => "Hex: aligned",
            Tab::Merge => "Merge: 3-way",
            Tab::Table => "Table: positional",
            Tab::Sync => "Sync: plan",
            Tab::Picture => "Picture: pixels",
            Tab::FolderMerge => "FolderMerge: 3-way",
            Tab::Audio => "Audio: fingerprint",
            Tab::Home => "",
        }
    }

    /// Active tab ke view toggles ka flags hissa. Home par khali. Pure;
    /// headless-tested.
    pub fn path_flags_label(&self) -> String {
        match self.tab {
            Tab::Home => String::new(),
            _ => {
                let wrap = if self.word_wrap { "on" } else { "off" };
                let lines = if self.show_line_numbers { "on" } else { "off" };
                format!("wrap:{wrap} lines:{lines}")
            }
        }
    }

    /// Ek path ka poora meta line: time | bytes | rules | flags. Khali
    /// parts [`views::format_path_meta`] girata hai. Pure; headless-tested.
    pub fn path_meta_for(&self, path: &str) -> String {
        views::format_path_meta(
            Self::path_time_label(path).as_str(),
            Self::path_byte_label(path).as_str(),
            self.path_rules_label(),
            self.path_flags_label().as_str(),
        )
    }

    /// Active session ke left/right meta lines. Triple tabs par left slot
    /// me base + left dono hain (left field ka meta kabhi nahi chhupta).
    /// Pure (fs read graceful); headless-tested.
    pub fn active_path_meta(&self) -> (String, String) {
        match self.tab {
            Tab::Merge | Tab::FolderMerge => {
                let (base, left, right) = self.triple_paths();
                let base_meta = self.path_meta_for(base.as_str());
                let left_meta = self.path_meta_for(left.as_str());
                let joined = [base_meta, left_meta]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<String>>()
                    .join(" || ");
                (joined, self.path_meta_for(right.as_str()))
            }
            Tab::Home => (String::new(), String::new()),
            _ => {
                let (left, right) = self.paths();
                (
                    self.path_meta_for(left.as_str()),
                    self.path_meta_for(right.as_str()),
                )
            }
        }
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
        self.save_slots_into_active();
    }

    /// Returns current merge triple paths.
    pub fn merge_paths(&self) -> (String, String, String) {
        (
            self.merge.base.clone(),
            self.merge.left.clone(),
            self.merge.right.clone(),
        )
    }

    /// Sets the active tab (legacy compat): same-kind session ho to switch,
    /// na ho to us kind ke slot paths se open (fresh kinds par khaali).
    /// Switch se pehle purane active ke slots save hote hain (T5/T6).
    pub fn set_tab(&mut self, tab: Tab) {
        if tab == Tab::Home {
            self.save_slots_into_active();
            self.tab = Tab::Home;
            return;
        }
        if self.sessions.active().map(|item| item.kind) == Some(tab) {
            self.tab = tab;
            return;
        }
        if let Some(id) = self
            .sessions
            .list()
            .iter()
            .find(|item| item.kind == tab)
            .map(|item| item.id)
        {
            self.switch_session(id);
            return;
        }
        self.save_slots_into_active();
        let (left, right, base) = self.slot_paths_for(tab);
        self.sessions.open(tab, left, right, base);
        self.tab = tab;
        self.load_active_into_slots();
    }

    /// Returns the active tab: active session ka kind, khaali par legacy
    /// `tab` (Default par Home). Pure; headless-tested.
    pub fn current_tab(&self) -> Tab {
        match self.sessions.active() {
            Some(item) => item.kind,
            None => self.tab,
        }
    }

    /// Us kind ke slots ke trimmed paths (session open/sync ke liye).
    /// Order `(left, right, base)` — `SessionList::open` jaisa. Pure.
    fn slot_paths_for(&self, kind: Tab) -> (String, String, String) {
        match kind {
            Tab::Text => (
                self.text.left.trim().to_string(),
                self.text.right.trim().to_string(),
                String::new(),
            ),
            Tab::Folder => (
                self.folder.left.trim().to_string(),
                self.folder.right.trim().to_string(),
                String::new(),
            ),
            Tab::Hex => (
                self.hex.left.trim().to_string(),
                self.hex.right.trim().to_string(),
                String::new(),
            ),
            Tab::Merge => (
                self.merge.left.trim().to_string(),
                self.merge.right.trim().to_string(),
                self.merge.base.trim().to_string(),
            ),
            Tab::Table => (
                self.table.left.trim().to_string(),
                self.table.right.trim().to_string(),
                String::new(),
            ),
            Tab::Sync => (
                self.sync.left.trim().to_string(),
                self.sync.right.trim().to_string(),
                String::new(),
            ),
            Tab::Picture => (
                self.picture.left.trim().to_string(),
                self.picture.right.trim().to_string(),
                String::new(),
            ),
            Tab::FolderMerge => (
                self.folder_merge.left.trim().to_string(),
                self.folder_merge.right.trim().to_string(),
                self.folder_merge.base.trim().to_string(),
            ),
            Tab::Audio => (
                self.audio.left.trim().to_string(),
                self.audio.right.trim().to_string(),
                String::new(),
            ),
            Tab::Home => (String::new(), String::new(), String::new()),
        }
    }

    /// Active session ke paths + cursor slots me load karta hai (switch ke
    /// baad; paint ke `paint_paths` signature bina tode). Khaali par no-op.
    fn load_active_into_slots(&mut self) {
        let snapshot = match self.sessions.active() {
            Some(item) => (
                item.kind,
                item.left.clone(),
                item.right.clone(),
                item.base.clone(),
                item.cursor,
            ),
            None => return,
        };
        let (kind, left, right, base, cursor) = snapshot;
        match kind {
            Tab::Text => {
                self.text.left = left;
                self.text.right = right;
            }
            Tab::Folder => {
                self.folder.left = left;
                self.folder.right = right;
            }
            Tab::Hex => {
                self.hex.left = left;
                self.hex.right = right;
            }
            Tab::Merge => {
                self.merge.left = left;
                self.merge.right = right;
                self.merge.base = base;
            }
            Tab::Table => {
                self.table.left = left;
                self.table.right = right;
            }
            Tab::Sync => {
                self.sync.left = left;
                self.sync.right = right;
            }
            Tab::Picture => {
                self.picture.left = left;
                self.picture.right = right;
            }
            Tab::FolderMerge => {
                self.folder_merge.left = left;
                self.folder_merge.right = right;
                self.folder_merge.base = base;
            }
            Tab::Audio => {
                self.audio.left = left;
                self.audio.right = right;
            }
            Tab::Home => return,
        }
        self.tab = kind;
        self.set_current_cursor(cursor);
    }

    /// Active session me current slots sync karta hai (T5 paths + T6
    /// cursor; type/compare/switch ke baad caller chalaye). Khaali ya
    /// unknown active par no-op. Trimmed likhta hai (model precedent).
    fn save_slots_into_active(&mut self) {
        let (id, kind) = match self.sessions.active() {
            Some(item) => (item.id, item.kind),
            None => return,
        };
        let (left, right, base) = self.slot_paths_for(kind);
        self.sessions.update_paths(id, left, right, base);
        self.sessions.update_cursor(id, self.cursor_for(kind));
    }

    /// Us kind ka find/scroll cursor. Hex `u64` saturate hokar aata hai
    /// (`as` cast, koi `unwrap` nahi). Pure; headless-tested.
    fn cursor_for(&self, kind: Tab) -> usize {
        match kind {
            Tab::Text => self.text_find_cursor,
            Tab::Folder => self.folder_cursor,
            Tab::Table => self.table_find_cursor,
            Tab::Hex => {
                let cursor = self.hex_find_cursor;
                let max = usize::MAX as u64;
                if cursor > max {
                    usize::MAX
                } else {
                    cursor as usize
                }
            }
            Tab::Home | Tab::Merge | Tab::Sync | Tab::Picture | Tab::FolderMerge | Tab::Audio => 0,
        }
    }

    /// Slots ko live session banata hai: active same-kind ho to update
    /// (T5/T6), na ho to open (cap/drop model rule se). Home par no-op.
    fn adopt_slots_as_session(&mut self) {
        let kind = self.tab;
        if kind == Tab::Home {
            return;
        }
        let (left, right, base) = self.slot_paths_for(kind);
        let cursor = self.cursor_for(kind);
        if let Some(id) = self.sessions.active_id() {
            let same = self.sessions.get(id).map(|item| item.kind) == Some(kind);
            if same {
                self.sessions.update_paths(id, left, right, base);
                self.sessions.update_cursor(id, cursor);
                return;
            }
        }
        self.sessions.open(kind, left, right, base);
    }

    /// Session switch (T3 bridge): pehle purane slots save, phir switch,
    /// phir naye slots load. Unknown id par false, `tab`/slots untouched.
    pub fn switch_session(&mut self, id: u64) -> bool {
        if self.sessions.active_id() == Some(id) {
            self.load_active_into_slots();
            return true;
        }
        self.save_slots_into_active();
        if !self.sessions.switch(id) {
            return false;
        }
        self.load_active_into_slots();
        self.error = None;
        self.status = format!("switched session: {}", self.tab.title());
        true
    }

    /// Session close (T2 bridge): band wahi hota hai; active band hua to
    /// naya active left-neighbour (na ho to right-neighbour); khaali par
    /// `tab` Home. Unknown id par false. Cap/drop model rule se.
    pub fn close_session(&mut self, id: u64) -> bool {
        if self.sessions.active_id() == Some(id) {
            self.save_slots_into_active();
        }
        if !self.sessions.close(id) {
            return false;
        }
        self.tab = match self.sessions.active() {
            Some(item) => item.kind,
            None => Tab::Home,
        };
        self.load_active_into_slots();
        self.error = None;
        if self.sessions.is_empty() {
            self.status = String::from("session closed: launcher ready");
        } else {
            self.status = format!("session closed: {}", self.tab.title());
        }
        true
    }

    /// Strip hit bridge: `Switch` par session badlo (paths load andar),
    /// `Close` par sirf hit band karo, `None` par no-op. Returns true jab
    /// koi switch/close hua ho. Paint ka ek-frame hit isi se chalta hai.
    /// Pure caller wiring; headless-tested.
    pub fn apply_session_action(&mut self, action: views::SessionAction) -> bool {
        match action {
            views::SessionAction::None => false,
            views::SessionAction::Switch(id) => self.switch_session(id),
            views::SessionAction::Close(id) => self.close_session(id),
        }
    }

    /// T7 bridge: session label rename karta hai (trimmed; blank ya
    /// unknown id par no-op false). Model `SessionList::rename` reuse,
    /// slots untouched. Pure caller wiring; headless-tested.
    pub fn rename_session(&mut self, id: u64, title: impl Into<String>) -> bool {
        if !self.sessions.rename(id, title) {
            return false;
        }
        self.error = None;
        self.status = match self.sessions.get(id) {
            Some(item) => format!("renamed session: {}", item.title),
            None => String::from("renamed session"),
        };
        true
    }

    /// Opens one Home session card: live session open + recent me record
    /// (Home khud record nahi hota). Returns true jab koi session khula ho.
    /// Pure state; paint card click isi ko chalata hai. Headless-tested.
    pub fn open_home_session(&mut self, tab: Tab) -> bool {
        if tab == Tab::Home {
            return false;
        }
        self.save_slots_into_active();
        let (left, right, base) = self.slot_paths_for(tab);
        self.sessions.open(tab, left, right, base);
        self.tab = tab;
        self.load_active_into_slots();
        self.home_recent.push(tab.title());
        let snapshot = self.current_snapshot();
        self.session_history.push(snapshot);
        self.recent_cleared = false;
        self.status = format!("opened {}", tab.title());
        true
    }

    /// Waqt-stamped open: snapshot `now_secs` ke saath record hota hai taaki
    /// row timestamp dikhe. Pure state; headless-tested.
    pub fn open_home_session_at(&mut self, tab: Tab, now_secs: u64) -> bool {
        if tab == Tab::Home {
            return false;
        }
        self.save_slots_into_active();
        let (left, right, base) = self.slot_paths_for(tab);
        self.sessions.open(tab, left, right, base);
        self.tab = tab;
        self.load_active_into_slots();
        self.home_recent.push(tab.title());
        let snapshot = self.current_snapshot();
        self.session_history.push_stamped(snapshot, now_secs);
        self.recent_cleared = false;
        self.status = format!("opened {}", tab.title());
        true
    }

    /// Session-start recent entries, naya-sabse-pehle order me.
    pub fn recent_sessions(&self) -> &[String] {
        self.home_recent.list()
    }

    /// Recent list saaf karta hai (cleared flag lagta hai taaki hint honest rahe).
    pub fn clear_recent_sessions(&mut self) {
        self.home_recent.clear();
        self.recent_cleared = true;
    }

    /// Khali recent list ka guidance: fresh start vs cleared.
    pub fn recent_empty_hint(&self) -> &'static str {
        home::recent_list_empty_hint(self.recent_cleared)
    }

    /// Current viewmodel tab ka home-tab roop (snapshot mapping).
    fn home_tab_of(tab: Tab) -> home::Tab {
        match tab {
            Tab::Home => home::Tab::Home,
            Tab::Text => home::Tab::Text,
            Tab::Folder => home::Tab::Folder,
            Tab::Hex => home::Tab::Hex,
            Tab::Merge => home::Tab::Merge,
            Tab::Table => home::Tab::Table,
            Tab::Sync => home::Tab::Sync,
            Tab::Picture => home::Tab::Picture,
            Tab::FolderMerge => home::Tab::FolderMerge,
            Tab::Audio => home::Tab::Audio,
        }
    }

    /// Home snapshot tab ka viewmodel roop (restore mapping).
    fn app_tab_of(tab: home::Tab) -> Tab {
        match tab {
            home::Tab::Home => Tab::Home,
            home::Tab::Text => Tab::Text,
            home::Tab::Folder => Tab::Folder,
            home::Tab::Hex => Tab::Hex,
            home::Tab::Merge => Tab::Merge,
            home::Tab::Table => Tab::Table,
            home::Tab::Sync => Tab::Sync,
            home::Tab::Picture => Tab::Picture,
            home::Tab::FolderMerge => Tab::FolderMerge,
            home::Tab::Audio => Tab::Audio,
        }
    }

    /// Active tab ka find/scroll cursor `usize` me. Hex `u64` yahan
    /// saturate hokar aata hai (`as` cast, koi `unwrap` nahi).
    fn current_cursor(&self) -> usize {
        self.cursor_for(self.tab)
    }

    /// Restore par cursor wapas lagata hai; doosre tabs untouched.
    fn set_current_cursor(&mut self, cursor: usize) {
        match self.tab {
            Tab::Text => {
                self.text_find_cursor = cursor;
            }
            Tab::Folder => {
                self.folder_cursor = cursor;
            }
            Tab::Table => {
                self.table_find_cursor = cursor;
                self.table_find_from = cursor;
            }
            Tab::Hex => {
                self.hex_find_cursor = cursor as u64;
            }
            Tab::Home | Tab::Merge | Tab::Sync | Tab::Picture | Tab::FolderMerge | Tab::Audio => {}
        }
    }

    /// Active session ka snapshot: tab + paths + cursor. Triple tabs par
    /// base samait, two-path par base khali. Pure; headless-tested.
    pub fn current_snapshot(&self) -> home::SessionSnapshot {
        let tab = Self::home_tab_of(self.tab);
        match self.tab {
            Tab::Merge | Tab::FolderMerge => {
                let (base, left, right) = self.triple_paths();
                home::SessionSnapshot::new(tab, left, right, base, self.current_cursor())
            }
            Tab::Home => home::SessionSnapshot::new(tab, "", "", "", 0),
            _ => {
                let (left, right) = self.paths();
                home::SessionSnapshot::new(tab, left, right, "", self.current_cursor())
            }
        }
    }

    /// Active session ko history me record karta hai (Home kabhi nahi).
    /// Returns true jab record hua. Compare/switch ke baad caller isi se
    /// last state bachata hai. Headless-tested.
    pub fn record_current_session(&mut self) -> bool {
        if self.tab == Tab::Home {
            return false;
        }
        self.home_recent.push(self.tab.title());
        let snapshot = self.current_snapshot();
        self.session_history.push(snapshot);
        self.recent_cleared = false;
        true
    }

    /// Waqt-stamped record: snapshot `now_secs` ke saath history me jata hai.
    /// Home par false. Pure state; headless-tested.
    pub fn record_current_session_at(&mut self, now_secs: u64) -> bool {
        if self.tab == Tab::Home {
            return false;
        }
        self.home_recent.push(self.tab.title());
        let snapshot = self.current_snapshot();
        self.session_history.push_stamped(snapshot, now_secs);
        self.recent_cleared = false;
        true
    }

    /// Restore history entries, naya-sabse-pehle order me.
    pub fn session_entries(&self) -> &[home::SessionSnapshot] {
        self.session_history.list()
    }

    /// Restore history ginti.
    pub fn session_count(&self) -> usize {
        self.session_history.len()
    }

    /// Restore history saaf karta hai (cleared flag lagta hai).
    pub fn clear_session_history(&mut self) {
        self.session_history.clear();
        self.recent_cleared = true;
    }

    /// Row timestamp label: `index` wale snapshot ka relative waqt, unknown
    /// ya out-of-range par `None` (paint timestamp chhupata hai). Koi panic nahi.
    pub fn session_age_label(&self, index: usize, now_secs: u64) -> Option<String> {
        self.session_history.get(index)?.age_label(now_secs)
    }

    /// Row meta line (status + timestamp): out-of-range par `None`.
    pub fn session_row_meta(&self, index: usize, now_secs: u64) -> Option<String> {
        self.session_history
            .get(index)
            .map(|snapshot| home::recent_row_meta_text(snapshot, now_secs))
    }

    /// Row status label (missing vs ready): out-of-range par `None`.
    pub fn session_row_status(&self, index: usize) -> Option<&'static str> {
        self.session_history
            .get(index)
            .map(|snapshot| home::recent_row_status_label(snapshot.has_empty_paths()))
    }

    /// Restore history se purana session reopen: tab + paths + cursor wapas.
    /// Stale model clear hota hai taaki purana result jhooth na bole.
    /// Empty paths par bhi session khulta hai (graceful hint + compare-first
    /// model), out-of-range index par false. Koi panic nahi. Headless-tested.
    pub fn reopen_session(&mut self, index: usize) -> bool {
        let snapshot = match self.session_history.get(index) {
            Some(entry) => entry.clone(),
            None => {
                self.status = String::from("no saved session at that position");
                return false;
            }
        };
        let tab = Self::app_tab_of(snapshot.tab);
        if tab == Tab::Home {
            self.status = String::from("no saved session at that position");
            return false;
        }
        self.tab = tab;
        if snapshot.is_three_way() {
            let (base_target, left_target, right_target) = match self.tab {
                Tab::Merge => (
                    &mut self.merge.base,
                    &mut self.merge.left,
                    &mut self.merge.right,
                ),
                _ => (
                    &mut self.folder_merge.base,
                    &mut self.folder_merge.left,
                    &mut self.folder_merge.right,
                ),
            };
            *base_target = snapshot.base.clone();
            *left_target = snapshot.left.clone();
            *right_target = snapshot.right.clone();
        } else {
            let (left_target, right_target) = match self.tab {
                Tab::Text => (&mut self.text.left, &mut self.text.right),
                Tab::Folder => (&mut self.folder.left, &mut self.folder.right),
                Tab::Hex => (&mut self.hex.left, &mut self.hex.right),
                Tab::Table => (&mut self.table.left, &mut self.table.right),
                Tab::Sync => (&mut self.sync.left, &mut self.sync.right),
                Tab::Picture => (&mut self.picture.left, &mut self.picture.right),
                Tab::Audio => (&mut self.audio.left, &mut self.audio.right),
                Tab::Home | Tab::Merge | Tab::FolderMerge => {
                    self.status = String::from("no saved session at that position");
                    return false;
                }
            };
            *left_target = snapshot.left.clone();
            *right_target = snapshot.right.clone();
        }
        self.clear_restored_tab_model();
        self.set_current_cursor(snapshot.cursor);
        self.error = None;
        self.adopt_slots_as_session();
        if snapshot.has_empty_paths() {
            self.status = format!(
                "reopened {}: saved paths empty, type both paths then Compare",
                tab.title()
            );
        } else {
            self.status = format!(
                "reopened {}: paths restored, Compare to refresh",
                tab.title()
            );
        }
        true
    }

    /// Reopen ke baad us tab ka stale model hatata hai. Report/consent bhi
    /// girta hai taaki purana result naye paths par jhooth na bole.
    fn clear_restored_tab_model(&mut self) {
        match self.tab {
            Tab::Text => {
                self.text_model = None;
            }
            Tab::Folder => {
                self.folder_model = None;
                self.folder_sync_last_report = None;
                self.folder_sync_selection.clear();
                self.reset_folder_dual();
            }
            Tab::Hex => {
                self.clear_hex_results();
            }
            Tab::Merge => {
                self.merge_model = None;
                self.clear_merge_picks();
                self.clear_merge_apply_state();
            }
            Tab::Table => {
                self.table_model = None;
                self.table_view.summary = views_table::TableKeySummary::default();
            }
            Tab::Sync => {
                self.sync_model = None;
                self.sync_last_report = None;
                self.sync_confirmed = false;
            }
            Tab::Picture => {
                self.picture_model = None;
                self.picture_view = PictureViewState::default();
                self.heatmap = None;
            }
            Tab::FolderMerge => {
                self.clear_folder_merge_results();
            }
            Tab::Audio => {
                self.audio_model = None;
            }
            Tab::Home => {}
        }
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

    /// CN-OPENER: tab naam se `Tab` (`--tab text` jaisa,
    /// case-insensitive). `home` reject hota hai (paths ke saath launcher
    /// kholna jhooth hota). Pure; headless-tested.
    pub fn tab_from_name(name: &str) -> Option<Tab> {
        let lowered = name.to_ascii_lowercase();
        match lowered.as_str() {
            "text" => Some(Tab::Text),
            "folder" => Some(Tab::Folder),
            "hex" => Some(Tab::Hex),
            "merge" => Some(Tab::Merge),
            "table" => Some(Tab::Table),
            "sync" => Some(Tab::Sync),
            "picture" => Some(Tab::Picture),
            "foldermerge" | "folder-merge" | "folder_merge" => Some(Tab::FolderMerge),
            "audio" => Some(Tab::Audio),
            _ => None,
        }
    }

    /// CN-OPENER: `argv` (program naam samait) se opener request nikalo.
    /// `None` jab `--open` na ho (existing behavior untouched). `Err` jab
    /// `--open` ke do path ya `--tab` ki value missing/unknown ho. Pure;
    /// headless-tested, `--help`-safe (GUI nahi khulta).
    pub fn parse_open_args(args: &[String]) -> Result<Option<OpenRequest>, String> {
        if args.is_empty() {
            return Ok(None);
        }
        let mut rest = match args.get(1..) {
            Some(rest) => rest,
            None => return Ok(None),
        };
        let mut opened: Option<(String, String)> = None;
        let mut tab = Tab::Text;
        while let Some((head, tail)) = rest.split_first() {
            match head.as_str() {
                "--open" => match tail {
                    [left, right, after @ ..]
                        if !left.starts_with("--") && !right.starts_with("--") =>
                    {
                        opened = Some((left.clone(), right.clone()));
                        rest = after;
                    }
                    _ => {
                        return Err(String::from(
                            "--open needs two paths: --open <left> <right>",
                        ));
                    }
                },
                "--tab" => match tail {
                    [name, after @ ..] => match Self::tab_from_name(name) {
                        Some(parsed) => {
                            tab = parsed;
                            rest = after;
                        }
                        None => {
                            return Err(format!(
                                "unknown --tab '{name}': text/folder/hex/merge/table/sync/picture/foldermerge/audio"
                            ));
                        }
                    },
                    _ => {
                        return Err(String::from("--tab needs a value: --tab text"));
                    }
                },
                _ => {
                    rest = tail;
                }
            }
        }
        match opened {
            Some((left, right)) => Ok(Some(OpenRequest { left, right, tab })),
            None => Ok(None),
        }
    }

    /// CN-OPENER: dono path slots me preload + tab khula, phir auto-compare
    /// taaki content rows bina typing ke banen. Missing paths par
    /// `require_paths` honest error panel deta hai (kabhi jhooth nahi).
    /// Home par false (kuch nahi khulta). Headless-tested.
    pub fn apply_open_request(&mut self, req: &OpenRequest) -> bool {
        match req.tab {
            Tab::Text => {
                self.text.left = req.left.clone();
                self.text.right = req.right.clone();
            }
            Tab::Folder => {
                self.folder.left = req.left.clone();
                self.folder.right = req.right.clone();
            }
            Tab::Hex => {
                self.hex.left = req.left.clone();
                self.hex.right = req.right.clone();
            }
            Tab::Merge => {
                self.merge.left = req.left.clone();
                self.merge.right = req.right.clone();
            }
            Tab::Table => {
                self.table.left = req.left.clone();
                self.table.right = req.right.clone();
            }
            Tab::Sync => {
                self.sync.left = req.left.clone();
                self.sync.right = req.right.clone();
            }
            Tab::Picture => {
                self.picture.left = req.left.clone();
                self.picture.right = req.right.clone();
            }
            Tab::FolderMerge => {
                self.folder_merge.left = req.left.clone();
                self.folder_merge.right = req.right.clone();
            }
            Tab::Audio => {
                self.audio.left = req.left.clone();
                self.audio.right = req.right.clone();
            }
            Tab::Home => return false,
        }
        self.set_tab(req.tab);
        self.run_active_tab()
    }

    /// CN-OPENER: `--help`/`-h` maujood ho to usage print karke GUI bina
    /// khole exit. Pure; headless-tested.
    pub fn help_requested(args: &[String]) -> bool {
        args.iter()
            .skip(1)
            .any(|arg| arg == "--help" || arg == "-h")
    }

    /// CN-OPENER: `--help` output (existing flags + opener flags).
    pub fn help_text() -> &'static str {
        "autocompare-gui: desktop client over the comparison engine.\n\
         \n\
         usage:\n\
         \x20 autocompare-gui [--open <left> <right> [--tab <name>]] [--screenshot <png>]\n\
         \n\
         flags:\n\
         \x20 --open <left> <right>  preload both paths and open the tab\n\
         \x20 --tab <name>           text/folder/hex/merge/table/sync/picture/foldermerge/audio (default: text)\n\
         \x20 --screenshot <png>     render the fixture PNG headlessly and exit\n\
         \x20 --help, -h             print this help and exit\n"
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
                self.folder_sync_last_report = None;
                self.folder_sync_selection.clear();
                self.reset_folder_dual();
                true
            }
            Tab::Hex => {
                std::mem::swap(&mut self.hex.left, &mut self.hex.right);
                self.clear_hex_results();
                true
            }
            Tab::Merge => {
                std::mem::swap(&mut self.merge.left, &mut self.merge.right);
                self.merge_model = None;
                self.merge_picks.clear();
                self.merge_apply_last_report = None;
                self.merge_apply_requested = false;
                self.merge_apply_state = MergeApplyState::Idle;
                self.merge_apply_last_summary = None;
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
                self.sync_last_report = None;
                self.sync_confirmed = false;
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
                self.clear_folder_merge_results();
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
            self.save_slots_into_active();
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
            let (base, left, right) = self.triple_paths();
            if base.is_empty() && left.is_empty() && right.is_empty() {
                let (left_path, right_path) = self.paths();
                self.record_path_history(&[left_path.as_str(), right_path.as_str()]);
            } else {
                self.record_path_history(&[base.as_str(), left.as_str(), right.as_str()]);
            }
            self.status = format!("comparing: {}", self.session_identity());
        }
        started
    }

    /// Fraction behind one worker update: `current/total` clamped to
    /// 0.0..=1.0, `None` while the total is unknown. Pure;
    /// headless-tested.
    pub fn progress_fraction_of(update: &JobProgress) -> Option<f32> {
        match update.total {
            Some(total) if total > 0 => {
                let done = update.current.min(total) as f32;
                Some((done / total as f32).clamp(0.0, 1.0))
            }
            _ => None,
        }
    }

    /// Throttle gate: repaint only when the message changed, the
    /// fraction first appears/disappears, jumps by >= 0.05, or hits
    /// done. Pure; headless-tested. This is what keeps per-chunk
    /// worker writes from becoming per-byte repaints.
    pub fn progress_should_emit(
        last_frac: Option<f32>,
        next_frac: Option<f32>,
        last_msg: Option<&str>,
        next_msg: &str,
    ) -> bool {
        if last_msg != Some(next_msg) {
            return true;
        }
        match (last_frac, next_frac) {
            (None, None) => false,
            (None, Some(_)) | (Some(_), None) => true,
            (Some(prev), Some(next)) => (next - prev).abs() >= 0.05 || next >= 1.0 || prev >= 1.0,
        }
    }

    /// Status-bar text behind one throttled update. Reuses the
    /// existing `status` field; no new paint state. Pure;
    /// headless-tested.
    pub fn progress_status_text(base: &str, fraction: Option<f32>, message: &str) -> String {
        match fraction {
            Some(frac) => {
                let pct = (frac * 100.0).round().clamp(0.0, 100.0) as u32;
                format!("{base} {pct}% ({message})")
            }
            None => format!("{base} ({message})"),
        }
    }

    /// Whether a background compare is still running.
    pub fn is_job_running(&self) -> bool {
        self.rx.is_some()
    }

    /// Requests cancellation of the running job, if any. The worker
    /// checks the flag at chunk boundaries and reports back through
    /// the channel, so the thread always joins via the normal result
    /// path and never leaks. Returns true when a job was signalled.
    pub fn cancel_active_job(&mut self) -> bool {
        match self.job.as_ref() {
            Some(handle) => {
                handle.cancel();
                self.status = format!("cancel requested: {}", self.session_identity());
                true
            }
            None => false,
        }
    }

    /// Polls the worker's latest progress and folds it into `status`
    /// through the throttle gate. Returns true when the status bar
    /// changed and needs a repaint. Headless-safe: no `ctx` use.
    pub fn poll_job_progress(&mut self) -> bool {
        let snapshot = match self.job.as_ref() {
            Some(handle) => handle.progress(),
            None => None,
        };
        let Some(update) = snapshot else {
            return false;
        };
        let next_frac = Self::progress_fraction_of(&update);
        if !Self::progress_should_emit(
            self.progress_fraction,
            next_frac,
            self.progress_message.as_deref(),
            update.message.as_str(),
        ) {
            return false;
        }
        let base = format!("comparing: {}", self.session_identity());
        self.status = Self::progress_status_text(base.as_str(), next_frac, update.message.as_str());
        self.progress_fraction = next_frac;
        self.progress_message = Some(update.message);
        true
    }

    /// Records a freshly spawned job: channel, handle, and the
    /// initial progress state behind one call so all nine `run_*`
    /// paths stay identical. Smallest-diff helper, not paint.
    fn note_job_started(&mut self, rx: Receiver<CompareResult>, handle: JobHandle<()>) {
        self.rx = Some(rx);
        self.job = Some(handle);
        self.progress_fraction = Some(0.0);
        self.progress_message = Some(String::new());
        self.status = format!("comparing: {}", self.session_identity());
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

    /// eframe Storage key jisme `dark_mode` per-run se aage persist hota hai.
    pub(crate) const DARK_MODE_STORAGE_KEY: &'static str = "dark_mode";

    /// Persisted dark_mode flag padhta hai (main.rs load path).
    /// `SessionSnapshot::new` trim precedent: aas-paas whitespace ignore hota hai;
    /// unknown/khali/missing par declared intent dark (true) — jhootha light kabhi nahi
    /// (`SessionSnapshot::age_secs` unknown→None precedent jaisa graceful default).
    pub(crate) fn dark_mode_from_storage(raw: Option<&str>) -> bool {
        !matches!(raw.map(str::trim), Some("0" | "false" | "light"))
    }

    /// Persisted dark_mode flag likhta hai (`App::save` se).
    pub(crate) fn dark_mode_to_storage(dark: bool) -> String {
        if dark {
            String::from("1")
        } else {
            String::from("0")
        }
    }

    /// Current font zoom step (clamped to `crate::theme::ZOOM_LEVELS`).
    pub fn zoom_level(&self) -> i8 {
        crate::theme::clamp_zoom_level(self.zoom_level)
    }

    /// Sets the font zoom step, clamped to MIN..=MAX.
    pub fn set_zoom_level(&mut self, level: i8) {
        self.zoom_level = crate::theme::clamp_zoom_level(level);
        self.status = format!("font zoom: {}", self.zoom_level());
    }

    /// Steps font zoom up one level (clamped at MAX).
    pub fn zoom_in(&mut self) {
        self.set_zoom_level(self.zoom_level().saturating_add(1));
    }

    /// Steps font zoom down one level (clamped at MIN).
    pub fn zoom_out(&mut self) {
        self.set_zoom_level(self.zoom_level().saturating_sub(1));
    }

    /// Resets font zoom to the default step.
    pub fn zoom_reset(&mut self) {
        self.set_zoom_level(crate::theme::ZOOM_DEFAULT_LEVEL);
    }

    /// Records user-visible status feedback.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
    }

    /// Current draggable pane split fraction (left share of the row).
    pub fn split_fraction(&self) -> f32 {
        views::clamp_split_fraction(self.split_fraction)
    }

    /// Sets the pane split fraction, clamped to 0.20..=0.80.
    pub fn set_split_fraction(&mut self, fraction: f32) {
        self.split_fraction = views::clamp_split_fraction(fraction);
        self.clamp_scroll_after_layout();
    }

    /// Drags the pane split by `dx` pixels over `avail` pixels of
    /// travel, clamped to 0.20..=0.80. Headless-testable core behind
    /// the paint-time drag handle.
    pub fn drag_split_by(&mut self, dx: f32, avail: f32) {
        self.split_fraction = views::split_apply_drag(self.split_fraction, dx, avail);
        self.clamp_scroll_after_layout();
    }

    /// Clamps every pager top after a layout change so a wider or
    /// narrower row never leaves a stale offset past the end. Reuses
    /// the existing per-tab sync (totals from the model, then clamp).
    pub fn clamp_scroll_after_layout(&mut self) {
        self.sync_hex_pager();
        self.sync_table_pager();
        self.sync_folder_pager();
        self.sync_folder_dual();
    }

    /// Clears hex results (model + view + byte snap), keeping mode and
    /// max_hunks. Har stale-wipe site yahi chalata hai taaki snap kabhi
    /// purane hunks ke saath na rahe. Pure state, no panic.
    fn clear_hex_results(&mut self) {
        self.hex_model = None;
        let max_hunks = self.hex_view.max_hunks;
        self.hex_view = views_hex::HexCompareView::empty(self.hex_view.mode, max_hunks);
        self.hex_bytes = None;
    }

    /// Adopts a finished hex compare: lines + view + callsite byte snap.
    /// Snap `None` (ya khali sides) par paint offset-only marks par rehta
    /// hai — purana behavior, kabhi jhooth nahi. Pure state, no panic.
    fn set_hex_success(
        &mut self,
        lines: Vec<String>,
        view: views_hex::HexCompareView,
        bytes: Option<HexByteSnap>,
    ) {
        let n = view.hunks.len();
        self.hex_model = Some(lines);
        self.hex_view = view;
        self.hex_bytes = bytes;
        self.error = None;
        self.status = format!("hex compare done: {n} hunk(s)");
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
        self.hex_bytes = None;
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

    /// Gutter-click jump: grid row `row` ko offset banakar covering
    /// hunk select karta hai (parity-m F1 model). Paint
    /// `views_hex::hex_row_start(row)` se row deta hai.
    pub fn hex_gutter_jump_to_row(&mut self, row: u64) -> bool {
        match views_hex::hex_row_to_hunk(row, &self.hex_view.hunks) {
            Some(idx) => {
                self.hex_view.selected = Some(idx);
                self.hex_pager.set_total(self.hex_view.hunks.len() as u64);
                self.hex_pager.bring_into_view(idx as u64);
                self.status = format!("hex: {}", self.hex_view.status_line());
                true
            }
            None => false,
        }
    }

    /// Syncs the hex pager total to the current hunk count.
    pub fn sync_hex_pager(&mut self) {
        self.hex_pager.set_total(self.hex_view.hunks.len() as u64);
    }

    /// One page forward in the hex hunk window.
    pub fn hex_page_next(&mut self) {
        self.sync_hex_pager();
        self.hex_pager.next_page();
        self.status = format!(
            "hex page: top {} of {}",
            self.hex_pager.top,
            self.hex_view.hunks.len()
        );
    }

    /// One page back in the hex hunk window.
    pub fn hex_page_prev(&mut self) {
        self.sync_hex_pager();
        self.hex_pager.prev_page();
        self.status = format!(
            "hex page: top {} of {}",
            self.hex_pager.top,
            self.hex_view.hunks.len()
        );
    }

    /// Selection-aware next: hunk select + pager bring-into-view.
    pub fn hex_select_next_paged(&mut self) {
        self.hex_select_next();
        if let Some(idx) = self.hex_view.selected {
            self.sync_hex_pager();
            self.hex_pager.bring_into_view(idx as u64);
        }
    }

    /// Selection-aware prev: hunk select + pager bring-into-view.
    pub fn hex_select_prev_paged(&mut self) {
        self.hex_select_prev();
        if let Some(idx) = self.hex_view.selected {
            self.sync_hex_pager();
            self.hex_pager.bring_into_view(idx as u64);
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

    /// Table find-box (parity-m F2 model): agli match cell `from_row`
    /// se, row-major wrap ke saath. Caller data rows deta hai.
    pub fn table_find_next_cell(
        rows: &[Vec<String>],
        needle: &str,
        from_row: usize,
        ignore_case: bool,
    ) -> Option<(usize, usize)> {
        views_table::table_find_next(rows, needle, from_row, ignore_case)
    }

    /// Syncs the table pager total to the current model rows.
    pub fn sync_table_pager(&mut self) {
        let total = self.table_model.as_ref().map_or(0, |m| m.rows.len());
        self.table_pager.total_rows = total;
        self.table_pager.clamp_top();
    }

    /// One page forward in the table row window.
    pub fn table_page_next(&mut self) {
        self.sync_table_pager();
        self.table_pager.next_page();
    }

    /// One page back in the table row window.
    pub fn table_page_prev(&mut self) {
        self.sync_table_pager();
        self.table_pager.prev_page();
    }

    /// Paged find-box: painted `label | text` rows par needle dhoondhkar
    /// pager ko `bring_into_view(row)` se visible karta hai. `from`
    /// cursor aage badhta hai taaki Next dabane par agla match mile.
    pub fn table_find_next_paged(&mut self) -> Option<(usize, usize)> {
        let needle = self.table_find_needle.clone();
        if needle.is_empty() {
            return None;
        }
        let model = self.table_model.as_ref()?;
        let texts: Vec<Vec<String>> = model
            .rows
            .iter()
            .map(|row| vec![format!("{} | {}", row.label, row.text)])
            .collect();
        let found = Self::table_find_next_cell(
            &texts,
            &needle,
            self.table_find_from,
            self.table_view.ignore_case,
        )?;
        self.sync_table_pager();
        self.table_pager.bring_into_view(found.0);
        self.table_find_from = found.0.saturating_add(1) % texts.len().max(1);
        self.status = format!("table find: row {} col {}", found.0 + 1, found.1 + 1);
        Some(found)
    }

    /// Table find search rows: painted `label | text` rows par needle
    /// dhoondha jata hai (legacy `table_find_next_paged` precedent).
    /// Khali model par khali vec. Pure-headless core (paint isi par
    /// matches gin-ta hai).
    fn table_find_search_rows(&self) -> Vec<Vec<String>> {
        match self.table_model.as_ref() {
            Some(model) => model
                .rows
                .iter()
                .map(|row| vec![format!("{} | {}", row.label, row.text)])
                .collect(),
            None => Vec::new(),
        }
    }

    /// Current needle par sab table matches (model order me cell pos).
    /// Khali needle par khali vec; paint badge isi se `N/M` gin-ta hai.
    pub fn table_find_matches(&self) -> Vec<(usize, usize)> {
        let needle = self.table_find_needle.clone();
        if needle.is_empty() {
            return Vec::new();
        }
        let texts = self.table_find_search_rows();
        views_table::table_find_match_rows(&texts, &needle, self.table_view.ignore_case)
    }

    /// Total matches ("M" badge ke liye).
    pub fn table_find_total(&self) -> usize {
        self.table_find_matches().len()
    }

    /// 1-based cursor position ("N" badge ke liye), match na ho par None.
    /// Text `text_find_position` precedent mirror.
    pub fn table_find_position(&self) -> Option<usize> {
        let total = self.table_find_total();
        if total == 0 {
            return None;
        }
        let at = self.table_find_cursor % total.max(1);
        Some(at.saturating_add(1))
    }

    /// N/M badge text: "2/5", koi match nahi par "0/0". Paint-side
    /// `views_table::paint_table_find_bar` ka App-side judvaan (text
    /// `text_find_badge` precedent mirror).
    pub fn table_find_badge(&self) -> String {
        views_table::table_find_badge_label(self.table_find_position(), self.table_find_total())
    }

    /// Needle badalne par cursor pehle match par; khali par 0 rehta hai.
    /// Legacy `table_find_from` (row-based) bhi reset hota hai taaki
    /// purana paged nav consistent rahe.
    pub fn table_find_set_needle(&mut self, needle: impl Into<String>) {
        self.table_find_needle = needle.into();
        self.table_find_cursor = 0;
        self.table_find_from = 0;
    }

    /// Agla table match: cursor wrap, pager bring-into-view + status
    /// `table find: row R (N/M)`. Khali needle ya zero matches par None.
    pub fn table_find_next_row(&mut self) -> Option<(usize, usize)> {
        let matches = self.table_find_matches();
        if matches.is_empty() {
            self.status = String::from("table find: no match");
            return None;
        }
        let at = self.table_find_cursor % matches.len().max(1);
        let found = matches.get(at).copied()?;
        self.table_find_cursor = at.saturating_add(1) % matches.len().max(1);
        let total_rows = self.table_find_search_rows().len();
        self.table_find_from = found.0.saturating_add(1) % total_rows.max(1);
        self.sync_table_pager();
        self.table_pager.bring_into_view(found.0);
        let pos = at.saturating_add(1);
        self.status = format!(
            "table find: row {} ({pos}/{})",
            found.0.saturating_add(1),
            matches.len()
        );
        Some(found)
    }

    /// Pichhla table match: cursor peeche wrap, pager bring-into-view +
    /// status `table find: row R (N/M)`. Khali needle ya zero matches par None.
    pub fn table_find_prev_row(&mut self) -> Option<(usize, usize)> {
        let matches = self.table_find_matches();
        if matches.is_empty() {
            self.status = String::from("table find: no match");
            return None;
        }
        let len = matches.len();
        let at = if self.table_find_cursor == 0 {
            len.saturating_sub(1)
        } else {
            (self.table_find_cursor.saturating_sub(1)) % len.max(1)
        };
        let found = matches.get(at).copied()?;
        self.table_find_cursor = at;
        self.table_find_from = found.0;
        self.sync_table_pager();
        self.table_pager.bring_into_view(found.0);
        let pos = at.saturating_add(1);
        self.status = format!(
            "table find: row {} ({pos}/{len})",
            found.0.saturating_add(1)
        );
        Some(found)
    }

    /// Table per-hunk merge (parity-m F3 model): chosen side ki row.
    /// `None` side matlab orphan ka missing side (fallback dusri side).
    pub fn table_merge_row(
        left_row: Option<&[String]>,
        right_row: Option<&[String]>,
        side: views_table::TableMergeSide,
    ) -> Vec<String> {
        views_table::table_resolve_row(left_row, right_row, side)
    }

    /// Live merge conflict hunks behind the current model.
    /// No model par khali vec taaki callers kabhi jhooth na bolen.
    pub fn merge_hunks(&self) -> Vec<viewmodel::MergeHunk> {
        match self.merge_model.as_ref() {
            Some(model) => viewmodel::merge_conflict_hunks(&model.lines),
            None => Vec::new(),
        }
    }

    /// Staged pick for one hunk, last-staged wins.
    /// Unknown index ya khali picks par `None`.
    pub fn merge_pick_for_hunk(&self, conflict_index: usize) -> Option<viewmodel::TakeChoice> {
        self.merge_picks
            .iter()
            .rev()
            .find(|pick| pick.conflict_index == conflict_index)
            .map(|pick| pick.choice)
    }

    /// Stages one per-hunk take-side decision in App state (last-wins
    /// upsert). Unknown hunk index chup-chaap ignore hota hai (false),
    /// kabhi jhootha decided count nahi banata. File apply (I/O) agle
    /// wave ka hai — ye sirf state wiring hai.
    pub fn decide_merge_hunk(
        &mut self,
        conflict_index: usize,
        side: viewmodel::TakeChoice,
    ) -> bool {
        let total = self.merge_hunks().len();
        if conflict_index >= total {
            return false;
        }
        for pick in self.merge_picks.iter_mut() {
            if pick.conflict_index == conflict_index {
                pick.choice = side;
                return true;
            }
        }
        self.merge_picks.push(viewmodel::MergeHunkPick {
            conflict_index,
            choice: side,
        });
        true
    }

    /// Clears staged merge picks (fresh compare / swap / failure par).
    pub fn clear_merge_picks(&mut self) {
        self.merge_picks.clear();
    }

    /// Paint bridge se staged picks adopt karta hai: dedup (aakhri staged
    /// jeetta hai, [`viewmodel::apply_merge_picks`] rule) + unknown-index
    /// prune (stale kabhi decided na gine). Silent hai (har frame chalta
    /// hai, status nahi chherta). Headless-tested.
    fn adopt_merge_picks(&mut self, lines: &[String], staged: Vec<viewmodel::MergeHunkPick>) {
        let total = viewmodel::merge_conflict_hunks(lines).len();
        let mut kept: Vec<viewmodel::MergeHunkPick> = Vec::new();
        for pick in &staged {
            if pick.conflict_index >= total {
                continue;
            }
            let mut next: Vec<viewmodel::MergeHunkPick> = Vec::new();
            for kept_pick in &kept {
                if kept_pick.conflict_index != pick.conflict_index {
                    next.push(*kept_pick);
                }
            }
            next.push(*pick);
            kept = next;
        }
        self.merge_picks = kept;
    }

    /// Paint ke baad App picks ka hisab: lines badli hon (naya compare /
    /// swap) to staged indices purane hunks naam lenge, isliye clear +
    /// status; nahi to staged adopt. Paint lines mutate nahi karta
    /// (global row apply-all stage karta hai), isliye aam frame me adopt
    /// chalta hai. Headless-tested.
    fn reconcile_merge_picks(
        &mut self,
        before: &[String],
        after: &[String],
        staged: Vec<viewmodel::MergeHunkPick>,
    ) {
        if before != after {
            self.clear_merge_picks();
            self.status = String::from("merge changed: staged picks cleared");
        } else {
            self.adopt_merge_picks(after, staged);
        }
    }

    /// Honest per-hunk progress over live lines + App-staged picks.
    /// No model par zero summary taaki ginti kabhi jhooth na bole.
    pub fn merge_pick_summary(&self) -> viewmodel::MergePickSummary {
        match self.merge_model.as_ref() {
            Some(model) => viewmodel::merge_pick_summary(&model.lines, &self.merge_picks),
            None => {
                let empty: Vec<String> = Vec::new();
                viewmodel::merge_pick_summary(&empty, &self.merge_picks)
            }
        }
    }

    /// Merge counts line: real picks se gini decided/pending.
    /// No model par purani `no results yet` wording rehti hai.
    pub fn merge_summary_line(&self) -> String {
        match self.merge_model.as_ref() {
            Some(model) => {
                let summary = self.merge_pick_summary();
                format!(
                    "merge: {} line(s), {}/{} decided, {} pending{}",
                    model.lines.len(),
                    summary.resolved,
                    summary.total,
                    summary.pending,
                    if summary.conflicted {
                        ", conflicted"
                    } else {
                        ""
                    }
                )
            }
            None => String::from("merge: no results yet"),
        }
    }

    /// Resolves staged take-side picks over the live merged lines
    /// (Gap #7/#8 apply path). Take-left/right/base/both decisions
    /// `apply_merge_picks` se splice hote hain; undecided hunks marker
    /// block bankar rehte hain taaki output kabhi jhooth na bole. No
    /// model par `Err` (compare first), stale picks par engine error
    /// string me. Pure-headless core; no I/O, never panics.
    pub fn merge_resolved_lines(&self) -> Result<Vec<String>, String> {
        match self.merge_model.as_ref() {
            Some(model) => viewmodel::apply_merge_picks(&model.lines, &self.merge_picks)
                .map_err(|err| err.to_string()),
            None => Err(String::from("merge apply: compare first")),
        }
    }

    /// Resolved merge lines ko file text me jodta hai: `"\n"` join plus
    /// trailing newline jab non-empty, khali par khali. Pure;
    /// headless-tested.
    pub fn join_merge_output(lines: &[String]) -> String {
        if lines.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        for (idx, line) in lines.iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            out.push_str(line);
        }
        out.push('\n');
        out
    }

    /// Resolved lines ka file text. No model ya stale picks par `Err`
    /// string. Pure; headless-tested.
    pub fn merge_apply_text(&self) -> Result<String, String> {
        match self.merge_resolved_lines() {
            Ok(lines) => Ok(Self::join_merge_output(&lines)),
            Err(err) => Err(err),
        }
    }

    /// Dry-run report line: kitni output lines likhi jayengi, kitne
    /// hunks decided/pending, saath me no-writes promise. Pure;
    /// headless-tested.
    pub fn merge_dry_run_summary(
        resolved: usize,
        decided: usize,
        total: usize,
        pending: usize,
    ) -> String {
        format!(
            "dry-run: {resolved} line(s) output, {decided}/{total} decided, {pending} pending (no writes)"
        )
    }

    /// Honest dry-run line over live model + staged picks. No model par
    /// compare-first note taaki ginti kabhi jhooth na bole. Pure-headless
    /// core (paint isi ko frame-start report karta hai).
    pub fn merge_dry_run_line(&self) -> String {
        match self.merge_model.as_ref() {
            Some(model) => {
                let summary = viewmodel::merge_pick_summary(&model.lines, &self.merge_picks);
                let resolved = match self.merge_resolved_lines() {
                    Ok(lines) => lines.len(),
                    Err(_) => 0,
                };
                Self::merge_dry_run_summary(
                    resolved,
                    summary.resolved,
                    summary.total,
                    summary.pending,
                )
            }
            None => String::from("merge apply: compare first"),
        }
    }

    /// Real-write outcome line: likhi lines plus output path. Pure;
    /// headless-tested.
    pub fn describe_merge_apply_report(wrote_lines: usize, out: &str) -> String {
        format!("merge apply done: {wrote_lines} line(s) -> {out}")
    }

    /// Merge output text ko `out` path par likhta hai. Missing parent ya
    /// permission fail graceful `Err` string me aata hai, kabhi panic
    /// nahi. Returns likhi line count (trailing newline = lines).
    /// Pure filesystem write; headless-tested over temp dirs.
    pub fn write_merge_output(out: &Path, text: &str) -> Result<usize, String> {
        match std::fs::write(out, text) {
            Ok(()) => {
                if text.is_empty() {
                    Ok(0)
                } else {
                    Ok(text.bytes().filter(|byte| *byte == b'\n').count())
                }
            }
            Err(err) => Err(format!(
                "merge apply failed: writing {}: {err}",
                out.display()
            )),
        }
    }

    /// Sets the merge dry-run gate. Default true; turning it off still
    /// needs a typed output path before any write happens.
    pub fn set_merge_dry_run(&mut self, dry_run: bool) {
        self.merge_dry_run = dry_run;
        self.status = if dry_run {
            String::from("merge dry-run on: Apply reports only, no writes")
        } else {
            String::from("merge dry-run off: Apply writes resolved output")
        };
    }

    /// Sets the merge output file path Apply likhega.
    pub fn set_merge_output_path(&mut self, path: impl Into<String>) {
        self.merge_output_path = path.into();
    }

    /// Whether Merge Apply may write now: dry-run off, model maujood,
    /// output path typed. Pending hunks write nahi rokte (markers ke
    /// saath likhe jate hain, report pending ginata hai). Pure;
    /// headless-tested.
    pub fn merge_should_write(&self) -> bool {
        !self.merge_dry_run
            && self.merge_model.is_some()
            && !self.merge_output_path.trim().is_empty()
    }

    /// Merge Apply entry point (Gap #7/#8, merge side). Dry-run staged
    /// decisions se output ginta hai aur kuch nahi likhta; real run
    /// output path par resolved text likhta hai. Har I/O ya resolve
    /// failure graceful message banta hai, model untouched rehta hai
    /// taaki user retry kar sake. Har branch report ke saath
    /// `merge_apply_state` + `merge_apply_last_summary` bhi set karta
    /// hai aur button intent consume karta hai. Returns true jab report
    /// line bani ho. Never panics; roots missing file error ek message hai.
    pub fn run_merge_apply(&mut self) -> bool {
        self.merge_apply_requested = false;
        let resolved = match self.merge_resolved_lines() {
            Ok(lines) => lines,
            Err(err) => {
                self.merge_apply_last_report = Some(err.clone());
                self.merge_apply_state = MergeApplyState::NoModel;
                let has_model = self.merge_model.is_some();
                self.merge_apply_last_summary = Some(MergeApplySummary {
                    dry_run: self.merge_dry_run,
                    resolved_lines: 0,
                    decided: 0,
                    total: if has_model {
                        self.merge_pick_summary().total
                    } else {
                        0
                    },
                    pending: if has_model {
                        self.merge_pick_summary().pending
                    } else {
                        0
                    },
                    wrote_lines: 0,
                    out: String::new(),
                });
                self.status = format!("merge apply: {err}");
                return false;
            }
        };
        let summary = self.merge_pick_summary();
        if self.merge_dry_run {
            let line = Self::merge_dry_run_summary(
                resolved.len(),
                summary.resolved,
                summary.total,
                summary.pending,
            );
            self.merge_apply_last_report = Some(line.clone());
            self.merge_apply_state = MergeApplyState::DryRunReported;
            self.merge_apply_last_summary = Some(MergeApplySummary {
                dry_run: true,
                resolved_lines: resolved.len(),
                decided: summary.resolved,
                total: summary.total,
                pending: summary.pending,
                wrote_lines: 0,
                out: String::new(),
            });
            self.status = format!("merge apply {line}");
            return true;
        }
        let out = self.merge_output_path.trim().to_string();
        if out.is_empty() {
            let msg = String::from("merge apply: type an output path first");
            self.merge_apply_last_report = Some(msg.clone());
            self.merge_apply_state = MergeApplyState::NeedsPath;
            self.merge_apply_last_summary = Some(MergeApplySummary {
                dry_run: false,
                resolved_lines: resolved.len(),
                decided: summary.resolved,
                total: summary.total,
                pending: summary.pending,
                wrote_lines: 0,
                out: String::new(),
            });
            self.status = msg;
            return false;
        }
        let text = Self::join_merge_output(&resolved);
        match Self::write_merge_output(Path::new(&out), &text) {
            Ok(wrote) => {
                let line = Self::describe_merge_apply_report(wrote, &out);
                self.merge_apply_last_report = Some(line.clone());
                self.merge_apply_state = MergeApplyState::Wrote;
                self.merge_apply_last_summary = Some(MergeApplySummary {
                    dry_run: false,
                    resolved_lines: resolved.len(),
                    decided: summary.resolved,
                    total: summary.total,
                    pending: summary.pending,
                    wrote_lines: wrote,
                    out: out.clone(),
                });
                self.status = format!("merge apply done: {line}");
                true
            }
            Err(err) => {
                self.merge_apply_last_report = Some(err.clone());
                self.merge_apply_state = MergeApplyState::Failed;
                self.merge_apply_last_summary = Some(MergeApplySummary {
                    dry_run: false,
                    resolved_lines: resolved.len(),
                    decided: summary.resolved,
                    total: summary.total,
                    pending: summary.pending,
                    wrote_lines: 0,
                    out: out.clone(),
                });
                self.status = format!("merge apply: {err}");
                false
            }
        }
    }

    /// Apply button intent stage karta hai (Gap #8 state half). Paint ka
    /// Apply click agle wave me isi ko chalayega; headless core sirf flag
    /// + status deta hai, koi I/O nahi. Hamesha true (intent noted).
    pub fn request_merge_apply(&mut self) -> bool {
        self.merge_apply_requested = true;
        self.status = String::from("merge apply requested");
        true
    }

    /// Staged Apply intent saaf karta hai (naya compare / user cancel).
    pub fn clear_merge_apply_request(&mut self) {
        self.merge_apply_requested = false;
    }

    /// Staged Apply intent consume karta hai: true tha to false karke
    /// true deta hai, nahi to false. Paint dispatch isi se one-shot chalta hai.
    pub fn take_merge_apply_request(&mut self) -> bool {
        if self.merge_apply_requested {
            self.merge_apply_requested = false;
            return true;
        }
        false
    }

    /// Paint Apply enable gate: model maujood jab Apply dabane layak hai.
    /// Dry-run/path gate `merge_should_write` alag rehta hai (write vs
    /// report ka faisla wahi karta hai). Pure; headless-tested.
    pub fn merge_apply_enabled(&self) -> bool {
        self.merge_model.is_some()
    }

    /// Apply state + structured summary + report ko ek saath reset karta
    /// hai. Naya compare / swap / validation-fail par purani intent ya
    /// stale summary inherit nahi hota. Paint adoption agle wave ka hai.
    pub fn clear_merge_apply_state(&mut self) {
        self.merge_apply_requested = false;
        self.merge_apply_state = MergeApplyState::Idle;
        self.merge_apply_last_summary = None;
        self.merge_apply_last_report = None;
    }

    /// Last apply ka honest summary line (Gap #8 state half). State +
    /// structured summary se banti hai taaki ginti kabhi jhooth na bole:
    /// no-model par compare-first, Idle par dry-run line ya idle note,
    /// baaki par last report reuse. Pure-headless core; paint agle wave
    /// me Apply section isi se dikhayega.
    pub fn merge_apply_summary_line(&self) -> String {
        match self.merge_model.as_ref() {
            None => String::from("merge apply: compare first"),
            Some(_) => match self.merge_apply_state {
                MergeApplyState::Idle => {
                    if self.merge_apply_requested {
                        String::from("merge apply: requested, compare done")
                    } else {
                        self.merge_dry_run_line()
                    }
                }
                MergeApplyState::DryRunReported
                | MergeApplyState::Wrote
                | MergeApplyState::NeedsPath
                | MergeApplyState::NoModel
                | MergeApplyState::Failed => match self.merge_apply_last_report.as_ref() {
                    Some(line) => line.clone(),
                    None => self.merge_dry_run_line(),
                },
            },
        }
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

    /// Honest counts line for the Folder tab, filtered tree par.
    /// Display filter ke baad jo rows dual panes me dikhti hain, ginti
    /// unhi par aati hai taaki summary kabhi jhooth na bole. Empty model
    /// reports no results yet so the view never lies.
    pub fn folder_counts_summary(&self) -> String {
        match self.folder_rows() {
            Some(rows) => self.folder_session.filtered_counts(rows).summary(),
            None => String::from("folder: no results yet"),
        }
    }

    /// Toggles one directory in the folder tree. Keeps the legacy
    /// `folder_expanded` set mirrored so older callers stay correct.
    pub fn toggle_folder_dir(&mut self, dir: &str) {
        self.folder_session.toggle(dir);
        tree::toggle_expanded(&mut self.folder_expanded, dir);
    }

    // ------------------------------------------------------------------
    // Folder filter-bar wiring (Wave AT-FOLDERFILTER, headless).
    // Naam/type/status + legacy checkbox setters: har setter display
    // state badalkar pager/dual sync karta hai taaki scroll stale na
    // rahe. Paint adoption agle wave ka hai; ye sirf App truth hai.
    // ------------------------------------------------------------------

    /// Current name query (filter-bar text field ka source of truth).
    pub fn folder_name_filter(&self) -> &str {
        self.folder_session.display.name_query.as_str()
    }

    /// Sets the name substring filter; trims nothing (match time par
    /// trim hota hai), pager/dual sync hota hai. Khali = saaf.
    pub fn set_folder_name_filter(&mut self, query: impl Into<String>) {
        let text = query.into();
        let shown = text.trim().to_string();
        self.folder_session.display.name_query = text;
        self.sync_folder_pager();
        self.sync_folder_dual();
        self.status = if shown.is_empty() {
            String::from("folder filter: name cleared")
        } else {
            format!("folder filter: name contains '{shown}'")
        };
    }

    /// Current entry-kind filter.
    pub fn folder_entry_kind(&self) -> tree::FolderEntryKind {
        self.folder_session.display.kind
    }

    /// Sets the file/dir chunta + pager/dual sync.
    pub fn set_folder_entry_kind(&mut self, kind: tree::FolderEntryKind) {
        self.folder_session.display.kind = kind;
        self.sync_folder_pager();
        self.sync_folder_dual();
        self.status = format!("folder filter: type {}", kind.label());
    }

    /// Current status filter.
    pub fn folder_status_filter(&self) -> tree::FolderStatusFilter {
        self.folder_session.display.status
    }

    /// Sets the glyph-family chunta + pager/dual sync.
    pub fn set_folder_status_filter(&mut self, status: tree::FolderStatusFilter) {
        self.folder_session.display.status = status;
        self.sync_folder_pager();
        self.sync_folder_dual();
        self.status = format!("folder filter: status {}", status.label());
    }

    /// Sets hide-orphans + pager/dual sync.
    pub fn set_folder_hide_orphans(&mut self, on: bool) {
        self.folder_session.display.hide_orphans = on;
        self.sync_folder_pager();
        self.sync_folder_dual();
        self.status = if on {
            String::from("folder filter: orphans hidden")
        } else {
            String::from("folder filter: orphans shown")
        };
    }

    /// Sets left-newer-only + pager/dual sync.
    pub fn set_folder_left_newer_only(&mut self, on: bool) {
        self.folder_session.display.left_newer_only = on;
        self.sync_folder_pager();
        self.sync_folder_dual();
        self.status = if on {
            String::from("folder filter: left-newer only")
        } else {
            String::from("folder filter: left-newer off")
        };
    }

    /// Sets right-newer-only + pager/dual sync.
    pub fn set_folder_right_newer_only(&mut self, on: bool) {
        self.folder_session.display.right_newer_only = on;
        self.sync_folder_pager();
        self.sync_folder_dual();
        self.status = if on {
            String::from("folder filter: right-newer only")
        } else {
            String::from("folder filter: right-newer off")
        };
    }

    /// Sets flatten + pager/dual sync.
    pub fn set_folder_flatten(&mut self, on: bool) {
        self.folder_session.display.flatten = on;
        self.sync_folder_pager();
        self.sync_folder_dual();
        self.status = if on {
            String::from("folder filter: flat list")
        } else {
            String::from("folder filter: tree list")
        };
    }

    /// Clears every folder filter (filter-bar Clear button ka App roop).
    /// Pager/dual sync hota hai; view state default par laut-ta hai.
    pub fn clear_folder_filters(&mut self) {
        self.folder_session.clear_filters();
        self.sync_folder_pager();
        self.sync_folder_dual();
        self.status = String::from("folder filters cleared");
    }

    /// Koi folder filter laga hai (badge jalane ke liye).
    pub fn folder_has_active_filters(&self) -> bool {
        self.folder_session.has_active_filters()
    }

    /// Lage hue folder filters ki ginti (badge `N`).
    pub fn folder_active_filter_count(&self) -> usize {
        self.folder_session.active_filter_count()
    }

    /// Khali folder view ka honest kaaran. Bina model hamesha
    /// `NoResults` (compare-first jhooth nahi bolta).
    pub fn folder_empty_state(&self) -> Option<tree::FolderEmptyState> {
        match self.folder_rows() {
            Some(rows) => self.folder_session.empty_state(rows),
            None => Some(tree::FolderEmptyState::NoResults),
        }
    }

    /// Khali folder view ka paint-stable hint line.
    pub fn folder_empty_hint(&self) -> &'static str {
        match self.folder_rows() {
            Some(rows) => self.folder_session.empty_hint(rows),
            None => tree::folder_empty_hint(Some(tree::FolderEmptyState::NoResults)),
        }
    }

    /// Rows-aware empty state: identical folders `AllSame` dete hain
    /// (glyph-blind legacy `folder_empty_state` prune/all-same me farak
    /// nahi karta). Paint-adoption par paint isi ko padhe.
    pub fn folder_empty_state_full_rows(&self) -> Option<tree::FolderEmptyState> {
        match self.folder_rows() {
            Some(rows) => {
                let visible = self.folder_session.visible_tree(rows).len();
                tree::folder_empty_state_full_rows(
                    true,
                    rows,
                    visible,
                    &self.folder_session.display,
                )
            }
            None => Some(tree::FolderEmptyState::NoResults),
        }
    }

    /// Rows-aware hint line: `folder_empty_state_full_rows` ka paint roop.
    /// Identical par "all same, nothing to sync", warna legacy line.
    pub fn folder_empty_hint_full_rows(&self) -> &'static str {
        tree::folder_empty_hint(self.folder_empty_state_full_rows())
    }

    /// Rows the folder tree paints: filtered, then hierarchical/flat.
    pub fn folder_visible_tree(&self) -> Vec<tree::FolderTreeNode> {
        match self.folder_rows() {
            Some(rows) => self.folder_session.visible_tree(rows),
            None => Vec::new(),
        }
    }

    /// Rows the dual panes paint: display filter lagakar, order same.
    /// Paint (`views::paint_folder_dual`) inhi se tree banata hai, isliye
    /// dual panes filtered dikhte hain; paint API same rehta hai.
    pub fn folder_dual_rows(&self) -> Vec<FolderRow> {
        match self.folder_rows() {
            Some(rows) => self.folder_session.filtered_rows(rows),
            None => Vec::new(),
        }
    }

    /// Folder gutter next-change (parity-m F1 model): `from` ke baad
    /// ka agla changed row index, wrap ke saath.
    pub fn folder_next_changed_index(&self, from: usize) -> Option<usize> {
        match self.folder_rows() {
            Some(rows) => tree::folder_next_changed(rows, from),
            None => None,
        }
    }

    /// Folder gutter prev-change (parity-m F1 model): `from` se pehle
    /// ka pichhla changed row index, wrap ke saath.
    pub fn folder_prev_changed_index(&self, from: usize) -> Option<usize> {
        match self.folder_rows() {
            Some(rows) => tree::folder_prev_changed(rows, from),
            None => None,
        }
    }

    /// Syncs the folder pager total to the current visible tree.
    pub fn sync_folder_pager(&mut self) {
        let total = self.folder_visible_tree().len();
        self.folder_pager.total_rows = total;
        self.folder_pager.clamp_top();
    }

    /// Syncs the dual-tree shared scroll total to the current visible
    /// tree (same rows `paint_folder_dual` paints). Select/pending
    /// rehta hai; top clamp hota hai. Headless-tested.
    pub fn sync_folder_dual(&mut self) {
        let total = self.folder_visible_tree().len();
        self.folder_dual.sync_total(total);
    }

    /// Clears dual-tree select + stub action and zeroes its scroll
    /// total; page size rehta hai. Fresh compare / swap / error par
    /// stale stub line na dikhe isliye. Headless-tested.
    pub fn reset_folder_dual(&mut self) {
        let per_page = self.folder_dual.pager.rows_per_page;
        self.folder_dual = tree::DualTreeState::new(per_page);
    }

    /// One page forward in the folder tree window.
    pub fn folder_page_next(&mut self) {
        self.sync_folder_pager();
        self.folder_pager.next_page();
    }

    /// One page back in the folder tree window.
    pub fn folder_page_prev(&mut self) {
        self.sync_folder_pager();
        self.folder_pager.prev_page();
    }

    /// Paged next-change: agla changed row + pager bring-into-view.
    pub fn folder_next_changed_paged(&mut self) -> Option<usize> {
        let from = self.folder_cursor;
        let found = self.folder_next_changed_index(from)?;
        self.sync_folder_pager();
        // Visible-tree index par map: raw rows index se visible index
        // nikaalna zaroori hai kyunki filter/collapse chhupa sakta hai.
        let nodes = self.folder_visible_tree();
        let visible_idx = nodes.iter().position(|n| {
            self.folder_rows().is_some_and(|rows| {
                rows.iter().position(|r| r.rel_path == n.row.rel_path) == Some(found)
            })
        })?;
        self.folder_pager.bring_into_view(visible_idx);
        self.folder_cursor = found.saturating_add(1);
        self.status = format!("folder next change: row {}", found + 1);
        Some(found)
    }

    /// Paged prev-change: pichhla changed row + pager bring-into-view.
    pub fn folder_prev_changed_paged(&mut self) -> Option<usize> {
        let from = self.folder_cursor;
        let found = self.folder_prev_changed_index(from)?;
        self.sync_folder_pager();
        let nodes = self.folder_visible_tree();
        let visible_idx = nodes.iter().position(|n| {
            self.folder_rows().is_some_and(|rows| {
                rows.iter().position(|r| r.rel_path == n.row.rel_path) == Some(found)
            })
        })?;
        self.folder_pager.bring_into_view(visible_idx);
        self.folder_cursor = found;
        self.status = format!("folder prev change: row {}", found + 1);
        Some(found)
    }

    /// Paged find-box: path me needle + pager bring-into-view.
    pub fn folder_find_next_paged(&mut self) -> Option<usize> {
        let needle = self.folder_find_needle.clone();
        if needle.is_empty() {
            return None;
        }
        let rows = self.folder_rows()?.clone();
        let found = tree::folder_find_next(&rows, &needle, self.folder_cursor, true)?;
        self.sync_folder_pager();
        let nodes = self.folder_visible_tree();
        let wanted = rows.get(found)?.rel_path.clone();
        if let Some(visible_idx) = nodes.iter().position(|n| n.row.rel_path == wanted) {
            self.folder_pager.bring_into_view(visible_idx);
        }
        self.folder_cursor = found.saturating_add(1) % rows.len().max(1);
        self.status = format!("folder find: row {}", found + 1);
        Some(found)
    }

    /// Text find-box (Wave Q model): needle row text me hai ya nahi.
    /// Case flag par ascii folding hota hai; khali needle kabhi hit nahi.
    pub fn text_find_contains(hay: &str, needle: &str, ignore_case: bool) -> bool {
        if needle.is_empty() {
            return false;
        }
        if ignore_case {
            hay.to_lowercase().contains(&needle.to_lowercase())
        } else {
            hay.contains(needle)
        }
    }

    /// Text find-box scan: dono panes par needle wale model row idx.
    /// Khali needle par khali vec; pure, paint se alag test hota hai.
    pub fn text_find_match_rows(
        rows: &[viewmodel::TextRow],
        needle: &str,
        ignore_case: bool,
    ) -> Vec<usize> {
        let mut out = Vec::new();
        if needle.trim().is_empty() {
            return out;
        }
        let mut idx = 0usize;
        for row in rows {
            if Self::text_find_contains(&row.left, needle, ignore_case)
                || Self::text_find_contains(&row.right, needle, ignore_case)
            {
                out.push(idx);
            }
            idx = idx.saturating_add(1);
        }
        out
    }

    /// Current needle par text matches (model order me row idx).
    pub fn text_find_matches(&self) -> Vec<usize> {
        match self.text_model.as_ref() {
            Some(model) => Self::text_find_match_rows(
                &model.rows,
                &self.text_find_needle,
                self.text_find_ignore_case,
            ),
            None => Vec::new(),
        }
    }

    /// Total matches ("M" badge ke liye).
    pub fn text_find_total(&self) -> usize {
        self.text_find_matches().len()
    }

    /// 1-based cursor position ("N" badge ke liye), match na ho par None.
    pub fn text_find_position(&self) -> Option<usize> {
        let total = self.text_find_total();
        if total == 0 {
            return None;
        }
        let at = self.text_find_cursor % total.max(1);
        Some(at.saturating_add(1))
    }

    /// N/M badge text: "2/5", koi match nahi par "0/0". Paint-side
    /// `views::text_find_badge_label` ka App-side judvaan (badge parity
    /// tested); FindState paint badge isi format me dikhata hai.
    pub fn text_find_badge(&self) -> String {
        views::text_find_badge_label(self.text_find_position(), self.text_find_total())
    }

    /// Needle badalne par cursor pehle match par; khali par 0 rehta hai.
    pub fn text_find_set_needle(&mut self, needle: impl Into<String>) {
        self.text_find_needle = needle.into();
        self.text_find_cursor = 0;
    }

    /// Agla text match: cursor wrap, model selection + status update.
    /// Khali needle ya zero matches par None.
    pub fn text_find_next_row(&mut self) -> Option<usize> {
        let matches = self.text_find_matches();
        if matches.is_empty() {
            self.status = String::from("text find: no match");
            return None;
        }
        let at = self.text_find_cursor % matches.len().max(1);
        let row = matches.get(at).copied()?;
        self.text_find_cursor = at.saturating_add(1) % matches.len().max(1);
        if let Some(model) = self.text_model.as_mut() {
            model.selected = Some(row);
        }
        let pos = at.saturating_add(1);
        self.status = format!(
            "text find: row {} ({pos}/{})",
            row.saturating_add(1),
            matches.len()
        );
        Some(row)
    }

    /// Pichhla text match: cursor peeche wrap, selection + status update.
    pub fn text_find_prev_row(&mut self) -> Option<usize> {
        let matches = self.text_find_matches();
        if matches.is_empty() {
            self.status = String::from("text find: no match");
            return None;
        }
        let len = matches.len();
        let at = if self.text_find_cursor == 0 {
            len.saturating_sub(1)
        } else {
            (self.text_find_cursor.saturating_sub(1)) % len.max(1)
        };
        let row = matches.get(at).copied()?;
        self.text_find_cursor = at;
        if let Some(model) = self.text_model.as_mut() {
            model.selected = Some(row);
        }
        let pos = at.saturating_add(1);
        self.status = format!("text find: row {} ({pos}/{})", row.saturating_add(1), len);
        Some(row)
    }

    /// Text paint prefs behind `views::paint_text_with_find`: App toggles
    /// se ek `TextViewPrefs`. Pure; headless-tested.
    pub fn text_find_prefs(&self) -> viewmodel::TextViewPrefs {
        viewmodel::TextViewPrefs {
            word_wrap: self.word_wrap,
            show_line_numbers: self.show_line_numbers,
            syntax_highlight: self.syntax_highlight,
        }
    }

    /// Legacy needle/flag ko `FindState` me seed karta hai: query ya flag
    /// badla ho to `set_query_cased` se recompute (cursor first hit par),
    /// warna stale model par `refresh_if_stale` (naya compare honest counts).
    /// Returns true jab FindState recompute/refresh hua. Pure-headless core
    /// (paint isi ko frame-start par chalata hai).
    pub fn sync_text_find_state(&mut self) -> bool {
        let needle = self.text_find_needle.clone();
        let flag = self.text_find_ignore_case;
        let stale = self.text_find.query() != needle || self.text_find.ignore_case() != flag;
        if stale {
            let rows_len = self.text_model.as_ref().map_or(0, |model| model.rows.len());
            if rows_len == 0 {
                self.text_find.set_query_cased(&[], &needle, flag);
            } else if let Some(model) = self.text_model.as_ref() {
                let rows = model.rows.clone();
                self.text_find.set_query_cased(&rows, &needle, flag);
            }
            return true;
        }
        match self.text_model.as_ref() {
            Some(model) => {
                let rows = model.rows.clone();
                self.text_find.refresh_if_stale(&rows)
            }
            None => self.text_find.refresh_if_stale(&[]),
        }
    }

    /// Paint ke baad legacy needle/flag ko `FindState` se mirror karta hai
    /// taaki purane compat helpers (badge/tests) stale na rahen. Cursor
    /// untouched rehta hai (legacy Next/Prev apna cursor chalate hain).
    pub fn mirror_legacy_from_text_find(&mut self) {
        let query = self.text_find.query().to_string();
        let flag = self.text_find.ignore_case();
        self.text_find_needle = query;
        self.text_find_ignore_case = flag;
    }

    /// `FindState` N/M badge text jo Text tab paint dikhata hai.
    pub fn text_find_state_badge(&self) -> String {
        self.text_find.count_label()
    }

    /// Hex needle text se bytes: separators hatakar hex jode decode hota
    /// hai, warna raw utf-8 bytes. Khali par khali vec.
    pub fn parse_hex_needle(text: &str) -> Vec<u8> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let mut compact = String::new();
        for ch in trimmed.chars() {
            if ch == ' '
                || ch == '\t'
                || ch == ','
                || ch == ';'
                || ch == '_'
                || ch == '-'
                || ch == ':'
            {
                continue;
            }
            compact.push(ch);
        }
        let even = compact.len().is_multiple_of(2);
        let all_hex = !compact.is_empty() && compact.chars().all(|c| c.is_ascii_hexdigit());
        if even && all_hex {
            let mut out = Vec::new();
            let bytes = compact.as_bytes();
            let mut i = 0usize;
            while i < bytes.len() {
                let hi = bytes.get(i).copied().unwrap_or(b'0');
                let lo = bytes.get(i.saturating_add(1)).copied().unwrap_or(b'0');
                let hex = [hi, lo];
                if let Ok(pair) = std::str::from_utf8(&hex)
                    && let Ok(byte) = u8::from_str_radix(pair, 16)
                {
                    out.push(byte);
                } else {
                    return trimmed.as_bytes().to_vec();
                }
                i = i.saturating_add(2);
            }
            return out;
        }
        trimmed.as_bytes().to_vec()
    }

    /// Hex needle ke occurrences ki ginti (non-overlapping aage scan).
    pub fn hex_find_count(haystack: &[u8], needle_text: &str) -> usize {
        let needle = Self::parse_hex_needle(needle_text);
        if needle.is_empty() || haystack.is_empty() || needle.len() > haystack.len() {
            return 0;
        }
        let mut count = 0usize;
        let mut from = 0u64;
        let total = haystack.len() as u64;
        while from < total {
            let Some(hit) = views_hex::hex_find_next(haystack, &needle, from) else {
                break;
            };
            count = count.saturating_add(1);
            let step = hit.saturating_add(needle.len() as u64);
            if step <= from {
                break;
            }
            from = step;
            if count > haystack.len() {
                break;
            }
        }
        count
    }

    /// Hex needle ki agli occurrence `from` se (wrap, views_hex reuse).
    pub fn hex_find_next_from(haystack: &[u8], needle_text: &str, from: u64) -> Option<u64> {
        let needle = Self::parse_hex_needle(needle_text);
        views_hex::hex_find_next(haystack, &needle, from)
    }

    /// Hex needle ki pichhli occurrence `from` se peeche (views_hex reuse).
    pub fn hex_find_prev_from(haystack: &[u8], needle_text: &str, from: u64) -> Option<u64> {
        let needle = Self::parse_hex_needle(needle_text);
        views_hex::hex_find_prev(haystack, &needle, from)
    }

    /// App cursor se hex agli occurrence; milne par cursor aage badhta hai.
    pub fn hex_find_advance(&mut self, haystack: &[u8]) -> Option<u64> {
        let hit = Self::hex_find_next_from(haystack, &self.hex_find_needle, self.hex_find_cursor)?;
        let step = hit.saturating_add(1) % (haystack.len() as u64).max(1);
        self.hex_find_cursor = step;
        self.status = format!("hex find: offset {hit:#010x}");
        Some(hit)
    }

    /// App cursor se hex pichhli occurrence; milne par cursor wahin tikta hai.
    pub fn hex_find_retreat(&mut self, haystack: &[u8]) -> Option<u64> {
        let hit = Self::hex_find_prev_from(haystack, &self.hex_find_needle, self.hex_find_cursor)?;
        self.hex_find_cursor = hit;
        self.status = format!("hex find: offset {hit:#010x}");
        Some(hit)
    }

    /// Stages a double-clicked node and routes via
    /// [`home::resolve_compare_tab`]. Directory clicks stage nothing
    /// and keep the tab; file clicks stage the path and move to Text.
    /// Returns the tab the host should show.
    pub fn stage_folder_double_click(&mut self, node: &tree::FolderTreeNode) -> Tab {
        let staged_file = self.folder_session.stage_double_click(node);
        let target = home::resolve_compare_tab(&self.folder_session.staged);
        if staged_file {
            self.set_tab(target);
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

    /// Ek row ka sync direction current preview mode me. Model ya row
    /// na mile par `None` (koi action nahi). Paint agle wave me isi
    /// se per-row badge banayega. Pure; headless-tested.
    pub fn folder_sync_direction(&self, rel_path: &str) -> tree::SyncRowDirection {
        match self.folder_rows() {
            Some(rows) => rows
                .iter()
                .find(|row| row.rel_path == rel_path)
                .map(|row| tree::sync_direction_for_row(row, self.folder_session.sync_mode))
                .unwrap_or(tree::SyncRowDirection::None),
            None => tree::SyncRowDirection::None,
        }
    }

    /// Ek row ka badge text current preview mode me. Unknown paths
    /// par `=` (koi action nahi). Paint adoption agle wave ka hai.
    pub fn folder_sync_badge(&self, rel_path: &str) -> &'static str {
        self.folder_sync_direction(rel_path).badge()
    }

    /// Ek real row ko sync selection me jodo. Returns true jab row
    /// maujood thi; unknown paths kabhi select nahi hote.
    pub fn select_folder_sync_row(&mut self, rel_path: &str) -> bool {
        let real = self
            .folder_rows()
            .is_some_and(|rows| rows.iter().any(|row| row.rel_path == rel_path));
        if real {
            self.folder_sync_selection.select(rel_path);
        }
        real
    }

    /// Ek row ka select flip karo. Unknown paths par false, kuch
    /// badalta nahi. Returns ab selected hai ya nahi.
    pub fn toggle_folder_sync_row(&mut self, rel_path: &str) -> bool {
        let real = self
            .folder_rows()
            .is_some_and(|rows| rows.iter().any(|row| row.rel_path == rel_path));
        if real {
            self.folder_sync_selection.toggle(rel_path)
        } else {
            false
        }
    }

    /// Sync selection saaf karo.
    pub fn clear_folder_sync_selection(&mut self) {
        self.folder_sync_selection.clear();
    }

    /// Selection par scoped dry-run preview (sirf selected rows).
    /// Khali selection ya bina model ke khali preview.
    pub fn folder_sync_selected_preview(&self) -> tree::SyncPreview {
        match self.folder_rows() {
            Some(rows) => tree::sync_preview_for_selection(
                rows,
                self.folder_session.sync_mode,
                &self.folder_sync_selection,
            ),
            None => tree::SyncPreview::default(),
        }
    }

    /// Poore model par real dry-run summary line (paint agle wave
    /// me summary section me dikhayega).
    pub fn folder_sync_summary_line(&self) -> String {
        let preview = self.folder_sync_preview();
        tree::format_sync_summary(tree::summarize_sync_preview(&preview))
    }

    /// Selection par real dry-run summary line: `N selected: <line>`.
    /// Khali selection par explicit zero line, jhooth kabhi nahi.
    pub fn folder_sync_selected_summary_line(&self) -> String {
        let preview = self.folder_sync_selected_preview();
        let counts = tree::summarize_sync_preview(&preview);
        format!(
            "{} selected: {}",
            self.folder_sync_selection.len(),
            tree::format_sync_summary(counts)
        )
    }

    /// Engine sync options behind the current preview mode (same mapping
    /// `plan_sync` uses: Both = NewerWins/no-prune, Mirror =
    /// LeftToRight/prune). Shown as dry-run text; preview itself never
    /// touches the filesystem.
    pub fn folder_sync_options_label(&self) -> String {
        let opts = tree::sync_options_for(self.folder_session.sync_mode);
        format!("options: {opts:?} (plan_sync parity, dry-run only)")
    }

    /// Safe one-direction options behind Gap #8 kadam 1: left-to-right
    /// copies only, no prune. `plan_sync` with these options can only
    /// produce `CopyToRight` steps; right-only / newer-right entries
    /// stay skipped, nothing is ever deleted. Pure; headless-tested.
    pub fn mirror_copy_options() -> SyncOptions {
        SyncOptions {
            direction: SyncDirection::LeftToRight,
            prune: false,
        }
    }

    /// Defensive subset of an engine plan: keeps only `CopyToRight`
    /// file steps, drops deletes and reverse copies even if a caller
    /// passes a wider plan. Skips and truncation pass through so the
    /// summary stays honest. Pure; headless-tested.
    pub fn mirror_copy_subset(plan: &SyncPlan) -> SyncPlan {
        let mut out = SyncPlan {
            steps: Vec::new(),
            skipped: plan.skipped.clone(),
            truncated: plan.truncated,
        };
        for step in &plan.steps {
            if step.action == SyncAction::CopyToRight {
                out.steps.push(step.clone());
            }
        }
        out
    }

    /// Plans the safe mirror copy via the engine (read-only): left and
    /// right roots must exist as directories. Errors become strings;
    /// never panics. Pure filesystem read; headless-tested.
    pub fn plan_mirror_copy(left: &Path, right: &Path) -> Result<SyncPlan, String> {
        plan_sync(left, right, &Self::mirror_copy_options())
            .map(|plan| Self::mirror_copy_subset(&plan))
            .map_err(|err| err.to_string())
    }

    /// Applies a (possibly pre-filtered) plan via the engine, keeping
    /// only the safe `CopyToRight` subset first. Each step failure is
    /// recorded in the report; missing roots are a `String` error.
    /// Never panics. Headless-tested over temp dirs.
    pub fn apply_mirror_copy(
        left: &Path,
        right: &Path,
        plan: &SyncPlan,
    ) -> Result<SyncReport, String> {
        let safe = Self::mirror_copy_subset(plan);
        apply_sync(left, right, &safe).map_err(|err| err.to_string())
    }

    /// One-line outcome behind Folder Apply: copies, no deletes, and
    /// the failure count. Pure; headless-tested.
    pub fn describe_mirror_report(report: &SyncReport) -> String {
        format!(
            "mirror copy: {} copied, {} deleted, {} failed",
            report.copied,
            report.deleted,
            report.failed.len()
        )
    }

    /// Sets the dry-run gate. Default true; turning it off still needs
    /// the consent checkbox before any write happens.
    pub fn set_folder_sync_dry_run(&mut self, dry_run: bool) {
        self.folder_sync_dry_run = dry_run;
        self.status = if dry_run {
            String::from("sync dry-run on: Apply reports only, no writes")
        } else {
            String::from("sync dry-run off: consented Apply writes left->right copies")
        };
    }

    /// Whether Folder Apply may write now: dry-run off, consented, and
    /// the current preview has steps. Pure; headless-tested.
    pub fn folder_should_write(&self) -> bool {
        !self.folder_sync_dry_run && self.folder_can_apply()
    }

    /// Folder Apply entry point (Gap #8 kadam 1). Dry-run reports what
    /// the engine would copy and writes nothing; consented non-dry-run
    /// plans via `plan_mirror_copy` and applies the safe subset, then
    /// refreshes the report line + status. Returns true when a report
    /// line was produced. Never panics; roots missing is a message.
    pub fn run_folder_mirror_apply(&mut self) -> bool {
        let preview = self.folder_sync_preview();
        if preview.steps.is_empty() {
            self.folder_sync_last_report = Some(String::from("mirror copy: nothing to do"));
            self.status = String::from("sync apply: nothing to do");
            return false;
        }
        if self.folder_sync_dry_run {
            let mut copies = 0usize;
            for step in &preview.steps {
                if step.action == SyncAction::CopyToRight {
                    copies = copies.saturating_add(1);
                }
            }
            let line =
                format!("dry-run: {copies} left->right cop(ies) would run, 0 deletes (no writes)");
            self.folder_sync_last_report = Some(line.clone());
            self.status = format!("sync apply {line}");
            return true;
        }
        if !self.folder_session.consented {
            self.folder_sync_last_report = Some(String::from("mirror copy: consent needed"));
            self.status = String::from("sync apply needs consent first");
            return false;
        }
        let (left, right) = (self.folder.left.clone(), self.folder.right.clone());
        if left.trim().is_empty() || right.trim().is_empty() {
            self.folder_sync_last_report =
                Some(String::from("mirror copy: type both folder paths first"));
            self.status = String::from("sync apply needs both folder paths");
            return false;
        }
        let planned = Self::plan_mirror_copy(Path::new(&left), Path::new(&right));
        let plan = match planned {
            Ok(plan) => plan,
            Err(err) => {
                self.folder_sync_last_report = Some(format!("mirror copy failed: {err}"));
                self.status = format!("sync apply failed: {err}");
                return false;
            }
        };
        match Self::apply_mirror_copy(Path::new(&left), Path::new(&right), &plan) {
            Ok(report) => {
                let line = Self::describe_mirror_report(&report);
                self.folder_sync_last_report = Some(line.clone());
                self.status = format!("sync apply done: {line}");
                true
            }
            Err(err) => {
                self.folder_sync_last_report = Some(format!("mirror copy failed: {err}"));
                self.status = format!("sync apply failed: {err}");
                false
            }
        }
    }

    /// Sets the Sync-tab dry-run gate. Default true; turning it off
    /// still needs the confirm checkbox before any write happens.
    pub fn set_sync_dry_run(&mut self, dry_run: bool) {
        self.sync_dry_run = dry_run;
        self.status = if dry_run {
            String::from("sync dry-run on: Apply reports only, no writes")
        } else {
            String::from("sync dry-run off: confirmed Apply writes checked left->right copies")
        };
    }

    /// Sets explicit Apply confirm for the Sync-tab checklist. Every
    /// real Apply resets it, so consent never carries over.
    pub fn set_sync_confirmed(&mut self, confirmed: bool) {
        self.sync_confirmed = confirmed;
        self.status = if confirmed {
            String::from("sync confirm on: Apply enabled when checked copies exist")
        } else {
            String::from("sync confirm off: preview only")
        };
    }

    /// Whether Sync-tab Apply may run now: the one shared gate behind
    /// both Apply paths. Folder side reads
    /// [`App::folder_can_apply`] (`can_apply_now`: consented + steps);
    /// Sync side reads this (confirmed + checked left->right copies) —
    /// same shape, same [`sync_apply_enabled`] core. Pure;
    /// headless-tested.
    pub fn sync_tab_can_apply(&self) -> bool {
        match self.sync_model.as_ref() {
            Some(list) => {
                sync_apply_enabled(self.sync_confirmed, list.copy_right_plan().steps.len())
            }
            None => false,
        }
    }

    /// Sync-tab Apply entry point (Gap #8, Sync checklist side).
    /// Dry-run counts checked left->right copies and writes nothing;
    /// a confirmed non-dry-run run plans nothing new — it converts the
    /// checklist (`copy_right_plan`) and applies via the same safe
    /// `apply_mirror_copy` subset the Folder tab uses, then refreshes
    /// the report line + status and drops confirm. Returns true when a
    /// report line was produced. Never panics; roots missing is a message.
    pub fn run_sync_checklist_apply(&mut self) -> bool {
        let (copy_steps, checked_total) = match self.sync_model.as_ref() {
            Some(list) => (list.copy_right_plan().steps.len(), list.checked_count()),
            None => {
                self.sync_last_report = Some(String::from("sync apply: compare first"));
                self.status = String::from("sync apply: compare first");
                return false;
            }
        };
        if copy_steps == 0 {
            self.sync_last_report = Some(String::from("sync apply: no checked left->right copies"));
            self.status = String::from("sync apply: nothing to copy");
            return false;
        }
        if self.sync_dry_run {
            let line = sync_dry_run_summary(copy_steps, checked_total);
            self.sync_last_report = Some(line.clone());
            self.status = format!("sync apply {line}");
            self.sync_confirmed = false;
            return true;
        }
        if !self.sync_confirmed {
            self.sync_last_report = Some(String::from("sync apply: confirm needed"));
            self.status = String::from("sync apply needs confirm first");
            return false;
        }
        let (left, right) = (self.sync.left.clone(), self.sync.right.clone());
        if left.trim().is_empty() || right.trim().is_empty() {
            self.sync_last_report = Some(String::from("sync apply: type both folder paths first"));
            self.status = String::from("sync apply needs both folder paths");
            return false;
        }
        let plan = match self.sync_model.as_ref() {
            Some(list) => list.copy_right_plan(),
            None => {
                self.sync_last_report = Some(String::from("sync apply: compare first"));
                self.status = String::from("sync apply: compare first");
                return false;
            }
        };
        let outcome = Self::apply_mirror_copy(Path::new(&left), Path::new(&right), &plan);
        self.sync_confirmed = false;
        match outcome {
            Ok(report) => {
                let line = Self::describe_mirror_report(&report);
                self.sync_last_report = Some(line.clone());
                self.status = format!("sync apply done: {line}");
                true
            }
            Err(err) => {
                self.sync_last_report = Some(format!("sync apply failed: {err}"));
                self.status = format!("sync apply failed: {err}");
                false
            }
        }
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
            Tab::Merge => self.merge_summary_line(),
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
                    self.save_slots_into_active();
                    self.tab = Tab::Text;
                    self.text.left = first.clone();
                    if let Some(second) = paths.get(1) {
                        self.text.right = second.clone();
                    }
                    self.adopt_slots_as_session();
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
        self.save_slots_into_active();
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
        let handle: JobHandle<()> = JobRunner::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, None, "starting text compare"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Text(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            let res =
                compare_text_files(Path::new(&left), Path::new(&right), &TextOptions::default())
                    .map(|cmp| viewmodel::build_text_rows(&cmp))
                    .map_err(|err| err.to_string());
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Text(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            progress(JobProgress::new(1, Some(1), "text compare complete"));
            let _ = tx.send(CompareResult::Text(res));
            Ok(())
        });
        self.note_job_started(rx, handle);
    }

    fn run_folder(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "folder") {
            self.folder_model = None;
            self.reset_folder_dual();
            return;
        }
        let (tx, rx) = channel();
        let handle: JobHandle<()> = JobRunner::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, None, "starting folder compare"));
            let res = compare_folders_cancellable(
                Path::new(&left),
                Path::new(&right),
                &FolderOptions::default(),
                &cancel,
            )
            .map(|cmp| viewmodel::build_folder_rows(&cmp, cmp.truncated))
            .map_err(|err| err.to_string());
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Folder(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            progress(JobProgress::new(1, Some(1), "folder compare complete"));
            let _ = tx.send(CompareResult::Folder(res));
            Ok(())
        });
        self.note_job_started(rx, handle);
    }

    fn run_hex(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "file") {
            self.clear_hex_results();
            return;
        }
        let (tx, rx) = channel();
        let mode = self.hex_view.mode;
        let max_hunks = self.hex_view.max_hunks;
        let handle: JobHandle<()> = JobRunner::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, None, "starting hex compare"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Hex(Err(String::from("operation cancelled"))));
                return Ok(());
            }
            let res = compare_binary_files_aligned(
                Path::new(&left),
                Path::new(&right),
                max_hunks,
                mode.to_alignment(),
            )
            .map(|cmp| {
                let view = views_hex::HexCompareView::from_comparison(&cmp, mode, max_hunks);
                let lines = viewmodel::build_hex_summary(&cmp);
                let bytes = read_hex_inputs_capped(Path::new(&left), Path::new(&right));
                (lines, view, bytes)
            })
            .map_err(|err| err.to_string());
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Hex(Err(String::from("operation cancelled"))));
                return Ok(());
            }
            progress(JobProgress::new(1, Some(1), "hex compare complete"));
            let _ = tx.send(CompareResult::Hex(res));
            Ok(())
        });
        self.note_job_started(rx, handle);
    }

    fn run_merge(&mut self) {
        let (base, left, right) = self.triple_paths();
        if !self.require_triple(&base, &left, &right, "file") {
            self.merge_model = None;
            self.merge_picks.clear();
            self.clear_merge_apply_state();
            return;
        }
        let (tx, rx) = channel();
        let handle: JobHandle<()> = JobRunner::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, Some(3), "loading base file"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Merge(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            let base_lines = match load_lines(Path::new(&base)) {
                Ok(lines) => lines,
                Err(err) => {
                    let _ = tx.send(CompareResult::Merge(Err(err.to_string())));
                    return Ok(());
                }
            };
            progress(JobProgress::new(1, Some(3), "loading left file"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Merge(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            let left_lines = match load_lines(Path::new(&left)) {
                Ok(lines) => lines,
                Err(err) => {
                    let _ = tx.send(CompareResult::Merge(Err(err.to_string())));
                    return Ok(());
                }
            };
            progress(JobProgress::new(2, Some(3), "loading right file"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Merge(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
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
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Merge(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            let summary = viewmodel::build_merge_summary(&merged);
            progress(JobProgress::new(3, Some(3), "merge complete"));
            let _ = tx.send(CompareResult::Merge(Ok(summary)));
            Ok(())
        });
        self.note_job_started(rx, handle);
    }

    fn run_table(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "table") {
            self.table_model = None;
            self.table_view.summary = views_table::TableKeySummary::default();
            return;
        }
        let (tx, rx) = channel();
        let key_opts = self.table_view.to_key_options();
        let positional = self.table_view.is_positional();
        let handle: JobHandle<()> = JobRunner::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, Some(2), "comparing table rows"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Table(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            let rows = compare_table_files(
                Path::new(&left),
                Path::new(&right),
                &TableOptions::default(),
            )
            .map(|cmp| viewmodel::build_table_rows(&cmp))
            .map_err(|err| err.to_string());
            progress(JobProgress::new(1, Some(2), "summarizing keyed columns"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Table(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
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
            progress(JobProgress::new(2, Some(2), "table compare complete"));
            let _ = tx.send(CompareResult::Table(combined));
            Ok(())
        });
        self.note_job_started(rx, handle);
    }

    fn run_sync(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "folder") {
            self.sync_model = None;
            return;
        }
        let (tx, rx) = channel();
        let handle: JobHandle<()> = JobRunner::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, None, "planning sync"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Sync(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            let res = plan_sync(Path::new(&left), Path::new(&right), &SyncOptions::default())
                .map(|plan| SyncChecklist::from_plan(&plan))
                .map_err(|err| err.to_string());
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Sync(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            progress(JobProgress::new(1, Some(1), "sync plan complete"));
            let _ = tx.send(CompareResult::Sync(res));
            Ok(())
        });
        self.note_job_started(rx, handle);
    }

    fn run_picture(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "picture") {
            self.picture_model = None;
            self.heatmap = None;
            return;
        }
        let (tx, rx) = channel();
        let handle: JobHandle<()> = JobRunner::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, Some(2), "comparing pictures"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Picture(
                    Err(String::from("operation cancelled")),
                    None,
                ));
                return Ok(());
            }
            let opts = PictureOptions::default();
            let res = compare_picture_files(Path::new(&left), Path::new(&right), &opts)
                .map(|cmp| PictureCompareModel::from_diff(&cmp.diff))
                .map_err(|err| err.to_string());
            progress(JobProgress::new(1, Some(2), "building heatmap"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Picture(
                    Err(String::from("operation cancelled")),
                    None,
                ));
                return Ok(());
            }
            let hm = load_heatmap_rgba(Path::new(&left), Path::new(&right)).ok();
            progress(JobProgress::new(2, Some(2), "picture compare complete"));
            let _ = tx.send(CompareResult::Picture(res, hm));
            Ok(())
        });
        self.note_job_started(rx, handle);
    }

    /// FolderMerge ke saare natije saaf karta hai: plan, teen snapshots
    /// aur staged decisions. Naya compare / swap / error / path-change
    /// par model ke saath hi clear hota hai.
    fn clear_folder_merge_results(&mut self) {
        self.folder_merge_model = None;
        self.folder_merge_base = Vec::new();
        self.folder_merge_left = Vec::new();
        self.folder_merge_right = Vec::new();
        self.folder_merge_decisions = Vec::new();
    }

    fn run_folder_merge(&mut self) {
        let (base, left, right) = self.triple_paths();
        if !self.require_triple(&base, &left, &right, "folder") {
            self.clear_folder_merge_results();
            return;
        }
        let (tx, rx) = channel();
        let handle: JobHandle<()> = JobRunner::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, None, "planning folder merge"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::FolderMerge(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            let res = plan_tree_merge(Path::new(&base), Path::new(&left), Path::new(&right))
                .map(|plan| FolderMergeOutcome {
                    plan: FolderMergeChecklist::from_plan(&plan),
                    base: snapshot_folder_presence(Path::new(&base)),
                    left: snapshot_folder_presence(Path::new(&left)),
                    right: snapshot_folder_presence(Path::new(&right)),
                })
                .map_err(|err| err.to_string());
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::FolderMerge(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            progress(JobProgress::new(1, Some(1), "folder merge plan complete"));
            let _ = tx.send(CompareResult::FolderMerge(res));
            Ok(())
        });
        self.note_job_started(rx, handle);
    }

    fn run_audio(&mut self) {
        let (left, right) = self.paths();
        if !self.require_paths(&left, &right, "audio") {
            self.audio_model = None;
            return;
        }
        let (tx, rx) = channel();
        let handle: JobHandle<()> = JobRunner::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, None, "starting audio compare"));
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Audio(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            let res = compare_audio_files(Path::new(&left), Path::new(&right))
                .map(|cmp| AudioChecklist::from_diff(&cmp.diff))
                .map_err(|err| err.to_string());
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Audio(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            progress(JobProgress::new(1, Some(1), "audio compare complete"));
            let _ = tx.send(CompareResult::Audio(res));
            Ok(())
        });
        self.note_job_started(rx, handle);
    }
}

/// Narrow-window breakpoint (px): below this width content rows keep
/// their split min-widths and h-scroll instead of overlapping.
pub const LAYOUT_NARROW_BREAKPOINT: f32 = 900.0;
/// Minimum hit size (px) for find prev/next controls.
pub const LAYOUT_FIND_HIT_MIN: f32 = 24.0;

/// Below-width test for the narrow-window path. Pure; unit-tested.
pub fn layout_is_narrow(avail: f32) -> bool {
    avail < LAYOUT_NARROW_BREAKPOINT
}

/// Split widths for a two-pane content row at `fraction` of `avail`.
/// Reuses Wave-R [`views::split_picture_widths`] so both panes share
/// one floor and the pair always fills the row; narrow windows keep
/// the floors and scroll instead of crushing. Pure; unit-tested.
pub fn layout_pane_widths(avail: f32, fraction: f32) -> (f32, f32) {
    views::split_picture_widths(avail, fraction)
}

/// Minimum honest row width for the current split: both pane floors
/// plus one section gap. Pure; unit-tested.
pub fn layout_row_min_width(avail: f32, fraction: f32) -> f32 {
    let (left, right) = layout_pane_widths(avail, fraction);
    left + right + views::SECTION_GAP
}

/// Whether the row at `avail` must h-scroll at `min_width` instead of
/// crushing panes. Pure; unit-tested.
pub fn layout_needs_hscroll(avail: f32, min_width: f32) -> bool {
    avail < min_width
}

/// Gap between adjacent toolbar controls (px). Reuses the shared
/// [`views::TIGHT_GAP`] token so the toolbar breathes on the same
/// beat as every other tight row. Pure; unit-tested.
pub fn toolbar_item_gap() -> f32 {
    views::TIGHT_GAP
}

/// Gap around the toolbar group separator (px). Reuses the shared
/// [`views::SECTION_GAP`] token so actions vs view-toggles read as
/// two groups with section rhythm. Pure; unit-tested.
pub fn toolbar_group_gap() -> f32 {
    views::SECTION_GAP
}

/// Docked find-bar geometry: the bar spans the full width, the badge
/// pins at the right end, prev/next hit rects keep the 24px floor.
/// Pure; unit-tested.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FindBarGeometry {
    /// Full-width bar span (px).
    pub bar_w: f32,
    /// Badge right-end x (px): always the bar end.
    pub badge_x: f32,
    /// Prev hit width (px): always the 24px floor.
    pub prev_hit: f32,
    /// Next hit width (px): always the 24px floor.
    pub next_hit: f32,
}

/// Computes [`FindBarGeometry`] for `avail` px of bar width. Pure;
/// unit-tested.
pub fn find_bar_geometry(avail: f32) -> FindBarGeometry {
    let bar_w = avail.max(0.0);
    FindBarGeometry {
        bar_w,
        badge_x: bar_w,
        prev_hit: LAYOUT_FIND_HIT_MIN,
        next_hit: LAYOUT_FIND_HIT_MIN,
    }
}

/// Pixel scroll-offset clamp after a layout change: keeps `offset`
/// inside `[0, content - viewport]`, zero when the content fits or the
/// input is not finite. Pure; unit-tested.
pub fn clamp_scroll_offset(offset: f32, content: f32, viewport: f32) -> f32 {
    if !offset.is_finite() || !content.is_finite() || !viewport.is_finite() {
        return 0.0;
    }
    let max = (content - viewport).max(0.0);
    offset.clamp(0.0, max)
}

/// Split-aware content wrapper: reserves the split row minimum inside
/// a horizontal scroll area so narrow windows h-scroll instead of
/// crushing or overlapping panes. Paint-only; geometry comes from
/// [`layout_row_min_width`].
fn show_split_content(ui: &mut egui::Ui, fraction: f32, content: impl FnOnce(&mut egui::Ui)) {
    let avail = ui.available_width();
    let row_min = layout_row_min_width(avail, fraction);
    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.allocate_space(egui::vec2(row_min.max(1.0), 0.0));
            content(ui);
        });
}

/// Find-nav button with the 24px exquisite hit floor. Paint-only;
/// returns clicked like `small_button`.
fn find_nav_button(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add(egui::Button::new(label).min_size(egui::vec2(LAYOUT_FIND_HIT_MIN, LAYOUT_FIND_HIT_MIN)))
        .clicked()
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

    /// One-direction plan behind Sync-tab Apply (Gap #8): keeps only
    /// checked `CopyToRight` file rows, drops deletes, reverse copies
    /// and unchecked rows. Skips + truncation pass through so the
    /// summary stays honest. Pure; headless-tested. The write path
    /// re-checks via [`App::mirror_copy_subset`] before touching disk.
    pub fn copy_right_plan(&self) -> SyncPlan {
        SyncPlan {
            steps: self
                .rows
                .iter()
                .filter(|row| row.checked && !row.is_dir && row.action == SyncAction::CopyToRight)
                .map(|row| autocompare_compare::SyncStep {
                    rel_path: row.rel_path.clone(),
                    is_dir: false,
                    action: row.action,
                })
                .collect(),
            skipped: self.skipped.clone(),
            truncated: self.truncated,
        }
    }
}

/// Whether Sync-tab Apply may run now: explicit confirm on and at
/// least one checked left->right copy. Dry-run needs the same gate;
/// the flag only decides writes vs report. Pure; headless-tested.
pub fn sync_apply_enabled(confirmed: bool, copy_steps: usize) -> bool {
    confirmed && copy_steps > 0
}

/// Dry-run report line: how many checked copies would run, with the
/// explicit no-writes promise. Pure; headless-tested.
pub fn sync_dry_run_summary(copy_steps: usize, checked_total: usize) -> String {
    format!(
        "dry-run: {copy_steps} of {checked_total} checked step(s) would copy left->right (no writes)"
    )
}

/// Folder-merge worker ka poora natija: engine plan se bana checklist
/// plus teen roots ke presence snapshots (tree rows isi se bante hain).
/// Snapshots khali ho sakte hain (roots khali hon ya walk skip ho), plan
/// tab bhi honest rehta hai.
#[derive(Debug, Clone, Default)]
pub struct FolderMergeOutcome {
    /// Engine plan se bana per-file checklist.
    pub plan: FolderMergeChecklist,
    /// Base root ka presence snapshot.
    pub base: Vec<FolderRow>,
    /// Left root ka presence snapshot.
    pub left: Vec<FolderRow>,
    /// Right root ka presence snapshot.
    pub right: Vec<FolderRow>,
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

/// Ek root ka presence snapshot: har entry ka relative path + dir mark,
/// sorted. Sirf tree presence ke liye hai (glyph/size/time khali rehte
/// hain). Walk errors skip hote hain — plan engine honest fail karta hai,
/// snapshot kabhi worker ko nahi rokta. Headless; unit-tested.
fn snapshot_folder_presence(root: &Path) -> Vec<FolderRow> {
    let mut out = Vec::new();
    let mut stack = vec![(String::new(), root.to_path_buf())];
    while let Some((rel, dir)) = stack.pop() {
        let Ok(children) = std::fs::read_dir(&dir) else {
            continue;
        };
        for child in children {
            let Ok(entry) = child else {
                continue;
            };
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let child_rel = if rel.is_empty() {
                name.to_string()
            } else {
                format!("{rel}/{name}")
            };
            let is_dir = std::fs::symlink_metadata(entry.path())
                .map(|meta| meta.is_dir())
                .unwrap_or_default();
            if is_dir {
                stack.push((child_rel.clone(), entry.path()));
            }
            out.push(FolderRow {
                rel_path: child_rel,
                is_dir,
                ..Default::default()
            });
        }
    }
    out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    out
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
    /// 10ms comparison windows over the common duration. The action-UI
    /// waveform strip reads these, so the strip always comes from App
    /// state (never a paint-side guess).
    pub windows_total: u64,
    /// Windows containing at least one differing sample.
    pub windows_differing: u64,
    /// Seconds into the stream of the first differing frame, when the
    /// streams are comparable and any window differs.
    pub first_diff_at: Option<f64>,
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
            windows_total: diff.windows_total,
            windows_differing: diff.windows_differing,
            first_diff_at: diff.first_diff_at,
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

/// Fixed width of the audio action-UI waveform strip, in characters.
/// One shared width keeps every compare on the same visual beat.
pub const AUDIO_WAVEFORM_WIDTH: usize = 40;

/// Stub label of the audio Play affordance. The word "stub" stays in
/// the label so nobody mistakes it for real playback; there is no
/// audio output path anywhere behind it.
pub fn audio_play_stub_label() -> &'static str {
    "Play (stub: no playback)"
}

/// Textual waveform strip over the compared windows: the differing
/// share renders as `#`, the rest as `-`. Total zero (nothing to draw
/// over) renders an honest all-`-` strip; differing clamps to total so
/// a wild model never overflows the fixed width. Pure; headless-tested.
pub fn audio_waveform_bar(differing: u64, total: u64) -> String {
    let width = AUDIO_WAVEFORM_WIDTH.max(1);
    let filled = if total == 0 {
        0
    } else {
        let ratio = differing.min(total) as f64 / total as f64;
        if ratio.is_finite() {
            (ratio * width as f64).ceil().clamp(0.0, width as f64) as usize
        } else {
            0
        }
    };
    let mut bar = String::new();
    for idx in 0..width {
        bar.push(if idx < filled { '#' } else { '-' });
    }
    bar
}

/// Honest first-difference line for the audio action UI. Non-finite or
/// missing stamps render as `-`, never as a made-up timestamp. Pure;
/// headless-tested.
pub fn audio_first_diff_text(first: Option<f64>) -> String {
    match first {
        Some(at) if at.is_finite() => format!("first difference at {at:.3}s"),
        _ => String::from("first difference: -"),
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

    /// Action-UI summary line for the Audio tab, straight from App
    /// state: the checklist verdict when compared, else an honest
    /// no-results note. Pure; headless-tested.
    pub fn audio_action_summary(&self) -> String {
        match self.audio_model.as_ref() {
            Some(list) => list.summary(),
            None => String::from("no audio results yet"),
        }
    }

    /// Stub behind the Audio tab Play affordance: records a status
    /// line only. There is no playback path, so the model stays
    /// untouched and the status names the stub outright. Headless-tested.
    pub fn request_audio_play_stub(&mut self) {
        self.status = String::from("audio play (stub): no playback — compare output only");
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(
            Self::DARK_MODE_STORAGE_KEY,
            Self::dark_mode_to_storage(self.dark_mode),
        );
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        // CK-DARKBUG: eframe 0.37 me `set_visuals` sirf active theme slot likhta
        // hai; preference default `System` hai isliye Light-Mode OS par
        // creation-time dark apply light slot me kabhi nahi lagta (live light +
        // badge desync). Har frame flag se preference + visuals dono re-assert
        // (existing Visuals reuse, naya palette zero) — toggle-corruption bhi
        // isi se repair hota hai.
        ui.ctx().set_theme(if self.dark_mode {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        ui.ctx().set_visuals(if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Title(self.window_title()));
        // Font zoom keys (Wave R zoom model): Cmd/Ctrl + +/-/0 over
        // `crate::theme::ZOOM_LEVELS`; theme owns sizes, App owns level.
        let (zoom_in, zoom_out, zoom_reset) = ui.input(|i| {
            let cmd = i.modifiers.command;
            (
                cmd && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)),
                cmd && i.key_pressed(egui::Key::Minus),
                cmd && i.key_pressed(egui::Key::Num0),
            )
        });
        let zoom_action = Self::resolve_zoom_key(zoom_in, zoom_out, zoom_reset);
        self.apply_zoom_key(zoom_action);
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
                            self.folder_sync_selection.clear();
                            self.sync_folder_dual();
                            self.error = None;
                            self.folder_sync_last_report = None;
                            self.status = format!("folder compare done: {n} entries");
                        }
                        Err(err) => {
                            self.folder_model = None;
                            self.folder_sync_selection.clear();
                            self.reset_folder_dual();
                            self.folder_sync_last_report = None;
                            self.status = format!("folder compare failed: {err}");
                            self.error = Some(err);
                        }
                    },
                    CompareResult::Hex(res) => match res {
                        Ok((lines, view, bytes)) => {
                            self.set_hex_success(lines, view, bytes);
                        }
                        Err(err) => {
                            self.clear_hex_results();
                            if let Some(view) = self.hex_over_cap_view() {
                                self.hex_view = view;
                                self.status = self.hex_view.status_line();
                            } else {
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
                            self.merge_picks.clear();
                            self.clear_merge_apply_state();
                            self.error = None;
                            self.status = format!(
                                "merge done: {n} line(s){}",
                                if conflicted { ", conflicted" } else { "" }
                            );
                        }
                        Err(err) => {
                            self.merge_model = None;
                            self.merge_picks.clear();
                            self.clear_merge_apply_state();
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
                            self.sync_last_report = None;
                            self.sync_confirmed = false;
                            self.status = format!("sync plan done: {n} step(s)");
                        }
                        Err(err) => {
                            self.sync_model = None;
                            self.sync_last_report = None;
                            self.sync_confirmed = false;
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
                        Ok(outcome) => {
                            let n = outcome.plan.rows.len();
                            self.folder_merge_model = Some(outcome.plan);
                            self.folder_merge_base = outcome.base;
                            self.folder_merge_left = outcome.left;
                            self.folder_merge_right = outcome.right;
                            self.folder_merge_decisions = Vec::new();
                            self.error = None;
                            self.status = format!("folder merge done: {n} step(s)");
                        }
                        Err(err) => {
                            self.clear_folder_merge_results();
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
                self.job = None;
                self.progress_fraction = None;
                self.progress_message = None;
            } else {
                self.poll_job_progress();
                ui.ctx().request_repaint();
            }
        }

        // Central panel with unified outer margin.
        egui::Frame::central_panel(ui.style())
            .inner_margin(egui::Margin::same(views::OUTER_MARGIN))
            .show(ui, |ui| {
            views::paint_header(ui, &mut self.dark_mode);
            // Session strip live wiring (T3/T2): patti mode tabs ke UPAR,
            // hit par switch/close (switch par paths load bridge ke andar).
            let mut strip_action = views::SessionAction::None;
            views::paint_session_tab_strip(ui, &self.sessions, &mut strip_action);
            self.apply_session_action(strip_action);
            // Mode-tab click session adopt: paint pick alag buffer me, badla
            // to `set_tab` se adopt (same-kind switch / fresh-kind open,
            // switch se pehle T5/T6 save andar). Direct `&mut self.tab`
            // bypass ab nahi (CALLER 07a8db8 follow-up).
            let mut picked_tab = self.tab;
            views::paint_tabs(ui, &mut picked_tab);
            if picked_tab != self.tab {
                self.set_tab(picked_tab);
            }
            ui.label(RichText::new(self.session_identity()).small().weak());
            ui.add_space(views::TAB_BOTTOM_GAP);

            // Single aligned toolbar row: actions (views call-site) + view toggles.
            let toolbar_active = self.tab != Tab::Home;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = toolbar_item_gap();
                // Icon-led Swap/Reload row: views::paint_session_toolbar live
                // call-site (glyph + chrome role + 120x28 floor andar hi);
                // hit par caller swap/run chalata hai.
                match views::paint_session_toolbar(ui, toolbar_active) {
                    views::ToolbarAction::Swap => {
                        // Swap, then re-run so both sides refresh like
                        // Beyond Compare does; empty sides report validation.
                        if self.swap_sides() {
                            self.run_active_tab();
                        }
                    }
                    views::ToolbarAction::Reload => {
                        self.run_active_tab();
                    }
                    views::ToolbarAction::None => {}
                }
                ui.add_space(toolbar_group_gap());
                ui.separator();
                ui.add_space(toolbar_group_gap());
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
            ui.add_space(views::TAB_BOTTOM_GAP);

            let tab = self.tab;
            let mut compare_clicked = false;
            match tab {
                Tab::Home => {
                    views::paint_home(ui, &mut self.tab);
                    ui.add_space(views::SECTION_GAP);
                    ui.label(RichText::new("Start here: open Text or Folder above, type both paths, then Compare. Swap Sides flips inputs; Reload reruns the same compare.").weak());
                }
                Tab::Text => {
                    let (left_meta, right_meta) = self.active_path_meta();
                    let current = self.text.left.clone();
                    views::paint_paths(
                        ui,
                        "file",
                        false,
                        &mut self.text.left,
                        &mut self.text.right,
                        &mut compare_clicked,
                        &mut self.split_fraction,
                    );
                    views::paint_path_meta_row(
                        ui,
                        &[left_meta, right_meta],
                        self.path_history_items(),
                        current.as_str(),
                    );
                }
                Tab::Folder => {
                    let (left_meta, right_meta) = self.active_path_meta();
                    let current = self.folder.left.clone();
                    views::paint_paths(
                        ui,
                        "folder",
                        true,
                        &mut self.folder.left,
                        &mut self.folder.right,
                        &mut compare_clicked,
                        &mut self.split_fraction,
                    );
                    views::paint_path_meta_row(
                        ui,
                        &[left_meta, right_meta],
                        self.path_history_items(),
                        current.as_str(),
                    );
                }
                Tab::Hex => {
                    let (left_meta, right_meta) = self.active_path_meta();
                    let current = self.hex.left.clone();
                    views::paint_paths(
                        ui,
                        "file",
                        false,
                        &mut self.hex.left,
                        &mut self.hex.right,
                        &mut compare_clicked,
                        &mut self.split_fraction,
                    );
                    views::paint_path_meta_row(
                        ui,
                        &[left_meta, right_meta],
                        self.path_history_items(),
                        current.as_str(),
                    );
                }
                Tab::Merge => show_split_content(ui, views::SPLIT_DEFAULT_FRACTION, |ui| {
                    let (left_meta, right_meta) = self.active_path_meta();
                    let current = self.merge.base.clone();
                    views::paint_triple_paths(
                        ui,
                        "file",
                        false,
                        &mut self.merge.base,
                        &mut self.merge.left,
                        &mut self.merge.right,
                        &mut compare_clicked,
                    );
                    views::paint_path_meta_row(
                        ui,
                        &[left_meta, right_meta],
                        self.path_history_items(),
                        current.as_str(),
                    );
                }),
                Tab::Table => {
                    let (left_meta, right_meta) = self.active_path_meta();
                    let current = self.table.left.clone();
                    views::paint_paths(
                        ui,
                        "table",
                        false,
                        &mut self.table.left,
                        &mut self.table.right,
                        &mut compare_clicked,
                        &mut self.split_fraction,
                    );
                    views::paint_path_meta_row(
                        ui,
                        &[left_meta, right_meta],
                        self.path_history_items(),
                        current.as_str(),
                    );
                }
                Tab::Sync => {
                    let (left_meta, right_meta) = self.active_path_meta();
                    let current = self.sync.left.clone();
                    views::paint_paths(
                        ui,
                        "folder",
                        true,
                        &mut self.sync.left,
                        &mut self.sync.right,
                        &mut compare_clicked,
                        &mut self.split_fraction,
                    );
                    views::paint_path_meta_row(
                        ui,
                        &[left_meta, right_meta],
                        self.path_history_items(),
                        current.as_str(),
                    );
                }
                Tab::Picture => {
                    let (left_meta, right_meta) = self.active_path_meta();
                    let current = self.picture.left.clone();
                    views::paint_paths(
                        ui,
                        "picture",
                        false,
                        &mut self.picture.left,
                        &mut self.picture.right,
                        &mut compare_clicked,
                        &mut self.split_fraction,
                    );
                    views::paint_path_meta_row(
                        ui,
                        &[left_meta, right_meta],
                        self.path_history_items(),
                        current.as_str(),
                    );
                }
                Tab::FolderMerge => show_split_content(ui, views::SPLIT_DEFAULT_FRACTION, |ui| {
                    let (left_meta, right_meta) = self.active_path_meta();
                    let current = self.folder_merge.base.clone();
                    views::paint_triple_paths(
                        ui,
                        "folder",
                        true,
                        &mut self.folder_merge.base,
                        &mut self.folder_merge.left,
                        &mut self.folder_merge.right,
                        &mut compare_clicked,
                    );
                    views::paint_path_meta_row(
                        ui,
                        &[left_meta, right_meta],
                        self.path_history_items(),
                        current.as_str(),
                    );
                }),
                Tab::Audio => {
                    let (left_meta, right_meta) = self.active_path_meta();
                    let current = self.audio.left.clone();
                    views::paint_paths(
                        ui,
                        "audio",
                        false,
                        &mut self.audio.left,
                        &mut self.audio.right,
                        &mut compare_clicked,
                        &mut self.split_fraction,
                    );
                    views::paint_path_meta_row(
                        ui,
                        &[left_meta, right_meta],
                        self.path_history_items(),
                        current.as_str(),
                    );
                }
            }

            // T5/T6 bridge: type ke baad slots session me save (trimmed
            // paths + cursor; khaali par no-op). Har frame idempotent hai.
            self.save_slots_into_active();

            let job_running = self.rx.is_some();
            let shown_fraction = self.progress_fraction;
            let mut cancel_clicked = false;
            if job_running {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("Comparing on background thread...").weak());
                    if let Some(frac) = shown_fraction {
                        ui.add(
                            egui::ProgressBar::new(frac.clamp(0.0, 1.0)).show_percentage(),
                        );
                    }
                    if ui.small_button("Cancel").clicked() {
                        cancel_clicked = true;
                    }
                });
            }
            if cancel_clicked {
                self.cancel_active_job();
            }

            if compare_clicked {
                self.run_active_tab();
            }

            if let Some(err) = self.error.clone() {
                views::paint_error(ui, err.as_str());
            }
            ui.add_space(views::TAB_BOTTOM_GAP);

            match tab {
                Tab::Home => {}
                Tab::Text => {
                    // Find box FINAL wiring (Gap #12): `FindState`-owned bar +
                    // N/M badge `views::paint_text_with_find` se aata hai.
                    // Bar geometry dock full-width rehti hai (U-GEO rects),
                    // prefs App toggles se bante hain, selection/filter/copy
                    // real model par hote hain (koi view-clone nahi). Legacy
                    // `text_find_needle` fields compat ke liye seed + mirror
                    // hote hain (Wave Q/S/T tests unhi par hain).
                    let bar_geo = find_bar_geometry(ui.available_width());
                    ui.set_min_width(bar_geo.bar_w);
                    self.sync_text_find_state();
                    let prefs = self.text_find_prefs();
                    let snapshot = self.text_model.clone();
                    let text_fraction = self.split_fraction();
                    self.clamp_scroll_after_layout();
                    if let Some(model) = self.text_model.as_mut() {
                        let find = &mut self.text_find;
                        show_split_content(ui, text_fraction, |ui| {
                            views::paint_text_with_find(ui, model, &prefs, find);
                        });
                    } else {
                        // No results: halka legacy bar live rehta hai (same
                        // GEO tokens, same labels, N/M badge) taaki needle
                        // compare se pehle type ho sake; frame-start sync
                        // usko FindState me push karta hai. Panes khali hain
                        // isliye `paint_text` empty note dikhata hai.
                        let badge = self.text_find_badge();
                        let old_needle = self.text_find_needle.clone();
                        let (find_prev, find_next) = views::paint_text_find_bar(
                            ui,
                            &mut self.text_find_needle,
                            &mut self.text_find_ignore_case,
                            &badge,
                        );
                        if self.text_find_needle != old_needle {
                            self.text_find_cursor = 0;
                        }
                        if find_prev
                            && self.text_find_prev_row().is_none()
                            && !self.text_find_needle.trim().is_empty()
                        {
                            self.status = String::from("text find: no match");
                        }
                        if find_next
                            && self.text_find_next_row().is_none()
                            && !self.text_find_needle.trim().is_empty()
                        {
                            self.status = String::from("text find: no match");
                        }
                        let mut empty = viewmodel::TextRows::default();
                        show_split_content(ui, text_fraction, |ui| {
                            views::paint_text(ui, &mut empty);
                        });
                        ui.label(
                            RichText::new("no text results yet: type both file paths and Compare.")
                                .weak(),
                        );
                    }
                    self.mirror_legacy_from_text_find();
                    if let (Some(before), Some(real)) =
                        (snapshot.as_ref(), self.text_model.as_ref())
                    {
                        if real.filter != before.filter {
                            let name = match real.filter {
                                viewmodel::TextFilter::All => "all rows",
                                viewmodel::TextFilter::Differences => "differences only",
                                viewmodel::TextFilter::Same => "same rows only",
                                viewmodel::TextFilter::Context => "differences with context",
                            };
                            self.status = format!("text filter: {name}");
                        }
                        let mut copied = None;
                        for (idx, after_row) in real.rows.iter().enumerate() {
                            let Some(before_row) = before.rows.get(idx) else {
                                break;
                            };
                            if before_row.kind != viewmodel::TextRowKind::Equal
                                && after_row.kind == viewmodel::TextRowKind::Equal
                            {
                                if after_row.right == before_row.left
                                    && after_row.left == before_row.left
                                {
                                    copied = Some((
                                        idx,
                                        viewmodel::CopyDirection::LeftToRight,
                                    ));
                                    break;
                                }
                                if after_row.left == before_row.right
                                    && after_row.right == before_row.right
                                {
                                    copied = Some((
                                        idx,
                                        viewmodel::CopyDirection::RightToLeft,
                                    ));
                                    break;
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
                        let mut display = self.folder_session.display.clone();
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
                        ui.horizontal_wrapped(|ui| {
                            if ui.small_button("◀ Prev page").clicked() {
                                self.folder_page_prev();
                            }
                            if ui.small_button("Next page ▶").clicked() {
                                self.folder_page_next();
                            }
                            if find_nav_button(ui, "◀ Prev change")
                                && self.folder_prev_changed_paged().is_none()
                            {
                                self.status = String::from("folder: no changed rows");
                            }
                            if find_nav_button(ui, "Next change ▶")
                                && self.folder_next_changed_paged().is_none()
                            {
                                self.status = String::from("folder: no changed rows");
                            }
                            ui.label(RichText::new("Find:").small().weak());
                            ui.add(
                                egui::TextEdit::singleline(&mut self.folder_find_needle)
                                    .desired_width(120.0)
                                    .hint_text("path needle"),
                            );
                            if find_nav_button(ui, "Next match ▶")
                                && self.folder_find_next_paged().is_none()
                            {
                                self.status = String::from("folder find: no match");
                            }
                        });
                        self.sync_folder_pager();
                        let (folder_page_up, folder_page_down) = ui.input(|input| {
                            (
                                input.key_pressed(egui::Key::PageUp),
                                input.key_pressed(egui::Key::PageDown),
                            )
                        });
                        if folder_page_up {
                            self.folder_page_prev();
                        }
                        if folder_page_down {
                            self.folder_page_next();
                        }
                        let nodes = self.folder_visible_tree();
                        self.folder_pager.total_rows = nodes.len();
                        self.folder_pager.clamp_top();
                        let (wstart, wend) = self.folder_pager.visible_range();
                        let wend = wend.min(nodes.len());
                        ui.label(
                            RichText::new(format!(
                                "page {} of {}, showing {} row(s)",
                                self.folder_pager.top
                                    / self.folder_pager.rows_per_page.max(1)
                                    + 1,
                                self.folder_pager.page_count().max(1),
                                wend.saturating_sub(wstart),
                            ))
                            .small()
                            .weak(),
                        );
                        // Split-aware rows: list keeps the split
                        // min-widths and h-scrolls instead of overlapping.
                        let folder_fraction = self.split_fraction();
                        self.clamp_scroll_after_layout();
                        show_split_content(ui, folder_fraction, |ui| {
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
                                let window = nodes.get(wstart..wend).unwrap_or(&[]);
                                for node in window {
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
                        });
                        // Dual-tree panes (Gap #19 kadam 3): filtered rows +
                        // filtered summary, shared select/scroll/action.
                        // Paint API same hai; filter App side se aata hai.
                        // Single-list paint upar untouched hai.
                        if self.folder_model.is_some() {
                            let dual_rows = self.folder_dual_rows();
                            let summary = self.folder_counts_summary();
                            views::paint_folder_dual(
                                ui,
                                &dual_rows,
                                summary.as_str(),
                                &mut self.folder_session.expanded,
                                &mut self.folder_dual,
                                &mut self.split_fraction,
                            );
                            self.folder_expanded =
                                self.folder_session.expanded.clone();
                        }
                        // Sync badges + selection + mode (BI-S1/S2 wiring):
                        // live Folder view me paint_folder_sync — mode radio
                        // paint ke andar hai, pick wapas App me
                        // set_folder_sync_mode se judta hai (consent reset
                        // samet) taaki grid + preview ek mode par rahen.
                        if self.folder_model.is_some() {
                            let sync_rows = self.folder_dual_rows();
                            let sync_summary = self.folder_counts_summary();
                            let sync_mode = self.folder_session.sync_mode;
                            let picked = {
                                let selection = &mut self.folder_sync_selection;
                                let expanded = &mut self.folder_session.expanded;
                                let pager = &mut self.folder_pager;
                                views::paint_folder_sync(
                                    ui,
                                    &sync_rows,
                                    sync_summary.as_str(),
                                    sync_mode,
                                    selection,
                                    expanded,
                                    pager,
                                )
                            };
                            self.folder_expanded =
                                self.folder_session.expanded.clone();
                            if picked != self.folder_session.sync_mode {
                                self.set_folder_sync_mode(picked);
                            }
                        }
                        // Sync preview: dry-run options, consent gate,
                        // Apply button (mode radio grid paint ke andar hai).
                        ui.add_space(views::TIGHT_GAP);
                        ui.separator();
                        ui.label(RichText::new("Sync preview (dry-run)").strong());
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
                        // Gap #8 kadam 1: dry-run default on, real writes
                        // only when user explicitly unchecks + consents.
                        // ONE direction: left->right copies, no deletes.
                        let mut dry_run = self.folder_sync_dry_run;
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut dry_run, "Dry-run (no writes)");
                            ui.label(
                                RichText::new("uncheck + consent to copy left->right").small().weak(),
                            );
                        });
                        if dry_run != self.folder_sync_dry_run {
                            self.set_folder_sync_dry_run(dry_run);
                        }
                        ui.label(
                            RichText::new("Apply copies left->right only, never deletes.")
                                .small()
                                .weak(),
                        );
                        if let Some(report) = self.folder_sync_last_report.clone() {
                            ui.label(RichText::new(report).monospace().small().weak());
                        }
                        let can_apply = self.folder_can_apply();
                        let apply = ui.add_enabled(can_apply, egui::Button::new("Apply sync plan"));
                        if apply.clicked() {
                            self.run_folder_mirror_apply();
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
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Gutter row:").small().weak());
                            ui.add(
                                egui::TextEdit::singleline(&mut self.hex_gutter_row_text)
                                    .desired_width(60.0)
                                    .hint_text("e.g. 2"),
                            );
                            if find_nav_button(ui, "Jump to row") {
                                match views::parse_gutter_row_text(&self.hex_gutter_row_text) {
                                    Some(row) => {
                                        if !self.hex_gutter_jump_to_row(row) {
                                            self.status = format!(
                                                "hex gutter: row {row} has no covering hunk"
                                            );
                                        }
                                    }
                                    None => {
                                        self.status = String::from(
                                            "hex gutter: type a row number >= 0",
                                        );
                                    }
                                }
                            }
                            if find_nav_button(ui, "◀ Prev change") {
                                self.hex_select_prev_paged();
                            }
                            if find_nav_button(ui, "Next change ▶") {
                                self.hex_select_next_paged();
                            }
                        });
                        if let Some(lines) = self.hex_model.clone() {
                            let bytes = self.hex_bytes.as_ref().map(HexByteSnap::as_slices);
                            let (view, pager) = (&mut self.hex_view, &mut self.hex_pager);
                            views::paint_hex(ui, view, pager, &lines, bytes);
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
                    if self.merge_model.is_some() {
                        let before: Vec<String> = match self.merge_model.as_ref() {
                            Some(model) => model.lines.clone(),
                            None => Vec::new(),
                        };
                        // Seed: App decisions paint temp store me, taaki cards
                        // App truth dikhayen (KEVL paint signature unchanged).
                        let seed = self.merge_picks.clone();
                        ui.data_mut(|data| {
                            data.insert_temp(views::merge_pick_id(&before), seed);
                        });
                        if let Some(model) = self.merge_model.as_mut() {
                            views::paint_merge(ui, model);
                        }
                        let after: Vec<String> = match self.merge_model.as_ref() {
                            Some(model) => model.lines.clone(),
                            None => Vec::new(),
                        };
                        // Adopt: card clicks (temp) App state me wapas.
                        let staged: Vec<viewmodel::MergeHunkPick> = ui
                            .data_mut(|data| {
                                data.get_temp::<Vec<viewmodel::MergeHunkPick>>(
                                    views::merge_pick_id(&before),
                                )
                            })
                            .unwrap_or_default();
                        self.reconcile_merge_picks(&before, &after, staged);
                        // Apply gate: dry-run + output path + Apply button live.
                        let enabled = self.merge_apply_enabled();
                        let state = self.merge_apply_state;
                        let summary = self.merge_apply_last_summary.clone();
                        let summary_line = self.merge_apply_summary_line();
                        let mut dry_run = self.merge_dry_run;
                        let mut out_path = self.merge_output_path.clone();
                        let apply_clicked = views::paint_merge_apply(
                            ui,
                            enabled,
                            &mut dry_run,
                            &mut out_path,
                            state,
                            summary.as_ref(),
                            &summary_line,
                        );
                        if dry_run != self.merge_dry_run {
                            self.set_merge_dry_run(dry_run);
                        }
                        if out_path != self.merge_output_path {
                            self.set_merge_output_path(out_path);
                        }
                        if apply_clicked {
                            self.request_merge_apply();
                            self.run_merge_apply();
                        }
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
                    // Table toolbar single row me (G-B2 dedup, `views_table`
                    // single source): Find + Prev/Next + page Prev/Next +
                    // page label + N/M badge; cursor/page advance App state
                    // me hota hai. `views.rs paint_table` apna temp-needle
                    // Find + page row abhi bhi chalata hai (BR wave usko
                    // single call me layega — exact spec REPORT me).
                    self.sync_table_pager();
                    let old_table_needle = self.table_find_needle.clone();
                    let table_badge = self.table_find_badge();
                    let table_page_label =
                        views_table::format_table_page_label(self.table_pager);
                    let table_action = views_table::paint_table_toolbar(
                        ui,
                        &mut self.table_find_needle,
                        &table_badge,
                        &table_page_label,
                    );
                    if self.table_find_needle != old_table_needle {
                        self.table_find_cursor = 0;
                        self.table_find_from = 0;
                    }
                    if table_action.find_next && self.table_find_next_row().is_none() {
                        self.status = String::from("table find: no match");
                    }
                    if table_action.find_prev && self.table_find_prev_row().is_none() {
                        self.status = String::from("table find: no match");
                    }
                    if table_action.page_next {
                        self.table_page_next();
                    }
                    if table_action.page_prev {
                        self.table_page_prev();
                    }
                    if self.table_model.is_some() {
                        // Split-aware rows: row keeps the split
                        // min-widths and h-scrolls instead of crushing.
                        let table_fraction = self.split_fraction();
                        self.clamp_scroll_after_layout();
                        if let Some(model) = self.table_model.clone() {
                            let (view, pager) = (&mut self.table_view, &mut self.table_pager);
                            show_split_content(ui, table_fraction, |ui| {
                                views::paint_table_keyed(ui, view, &model, pager);
                            });
                        }
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
                    // Gap #8: one-direction Apply (checked left->right
                    // copies only, never deletes). Dry-run default on;
                    // real writes need uncheck + explicit confirm.
                    ui.add_space(views::TIGHT_GAP);
                    ui.separator();
                    ui.label(
                        RichText::new("Apply copies checked left->right only, never deletes.")
                            .small()
                            .weak(),
                    );
                    let mut dry_run = self.sync_dry_run;
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut dry_run, "Dry-run (no writes)");
                        ui.label(
                            RichText::new("uncheck + confirm to copy left->right")
                                .small()
                                .weak(),
                        );
                    });
                    if dry_run != self.sync_dry_run {
                        self.set_sync_dry_run(dry_run);
                    }
                    let mut confirmed = self.sync_confirmed;
                    let (copy_steps, checked_total) = match self.sync_model.as_ref() {
                        Some(list) => (
                            list.copy_right_plan().steps.len(),
                            list.checked_count(),
                        ),
                        None => (0, 0),
                    };
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut confirmed, "I understand Apply copies files left->right");
                        ui.label(
                            RichText::new(home::sync_consent_hint(confirmed, copy_steps))
                                .small()
                                .weak(),
                        );
                    });
                    if confirmed != self.sync_confirmed {
                        self.set_sync_confirmed(confirmed);
                    }
                    if let Some(report) = self.sync_last_report.clone() {
                        ui.label(RichText::new(report).monospace().small().weak());
                    }
                    ui.label(
                        RichText::new(format!(
                            "{copy_steps} of {checked_total} checked step(s) are left->right copies"
                        ))
                        .small()
                        .weak(),
                    );
                    let apply = ui.add_enabled(
                        self.sync_tab_can_apply(),
                        egui::Button::new("Apply checked copies"),
                    );
                    if apply.clicked() {
                        self.run_sync_checklist_apply();
                    }
                }
                Tab::Picture => {
                    if self.picture_model.is_some() {
                        let mut view = self.picture_view;
                        let mut changed = false;
                        // Split-aware columns: row keeps the split
                        // min-widths and h-scrolls instead of crushing.
                        let picture_fraction = self.split_fraction();
                        self.clamp_scroll_after_layout();
                        if let Some(model) = self.picture_model.as_ref() {
                            let heat = self.heatmap.as_ref();
                            show_split_content(ui, picture_fraction, |ui| {
                                views::paint_picture_compare(
                                    ui,
                                    model,
                                    &mut view,
                                    heat,
                                    &mut self.split_fraction,
                                );
                            });
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
                    if self.folder_merge_model.is_some() {
                        let rows = tree::three_way_tree_rows(
                            &self.folder_merge_base,
                            &self.folder_merge_left,
                            &self.folder_merge_right,
                        );
                        if rows.is_empty() {
                            // Khali union par flat plan hi honest hai (koi
                            // tree row nahi); snapshots aate hi tree paint
                            // hota hai. Dono painters live rehte hain.
                            if let Some(list) = self.folder_merge_model.as_mut() {
                                views::paint_folder_merge_checklist(ui, list);
                            }
                        } else {
                            views::paint_three_way_tree(
                                ui,
                                &rows,
                                &mut self.folder_merge_decisions,
                            );
                        }
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
                    // Gap #8 audio action UI: stub play affordance plus
                    // summary/waveform display from App state. Play is a
                    // labeled stub (koi real playback nahi); the strip
                    // reads the checklist windows, never a paint guess.
                    ui.add_space(views::TIGHT_GAP);
                    ui.separator();
                    ui.label(RichText::new(self.audio_action_summary()).small().weak());
                    if let Some(list) = self.audio_model.as_ref() {
                        let bar = audio_waveform_bar(
                            list.windows_differing,
                            list.windows_total,
                        );
                        ui.label(
                            RichText::new(format!("waveform (diff share): [{bar}]"))
                                .monospace()
                                .small()
                                .weak(),
                        );
                        ui.label(
                            RichText::new(audio_first_diff_text(list.first_diff_at))
                                .small()
                                .weak(),
                        );
                    }
                    let play = ui.add_enabled(
                        self.audio_model.is_some(),
                        egui::Button::new(audio_play_stub_label()),
                    );
                    if play.clicked() {
                        self.request_audio_play_stub();
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
    fn path_history_records_dedupe_and_caps() {
        let mut app = App::default();
        assert!(app.path_history_items().is_empty());
        app.record_path_history(&["  ", "b.txt", "a.txt"]);
        assert_eq!(
            app.path_history_items(),
            &["a.txt".to_string(), "b.txt".to_string()]
        );
        app.record_path_history(&["b.txt"]);
        assert_eq!(
            app.path_history_items(),
            &["b.txt".to_string(), "a.txt".to_string()]
        );
        for index in 0..20 {
            app.record_path_history(&[format!("f{index}.txt").as_str()]);
        }
        assert_eq!(app.path_history_items().len(), home::HOME_RECENT_MAX);
        assert!(
            app.path_history_items()
                .first()
                .is_some_and(|first| first == "f19.txt")
        );
    }

    #[test]
    fn path_labels_are_honest_on_missing_files() {
        assert_eq!(App::path_byte_label(""), String::new());
        assert_eq!(App::path_time_label(""), String::new());
        assert_eq!(
            App::path_byte_label("definitely-missing-pathmeta-file.txt"),
            String::new()
        );
        assert_eq!(
            App::path_time_label("definitely-missing-pathmeta-file.txt"),
            String::new()
        );
        assert!(!App::path_byte_label("src/app.rs").is_empty());
        assert!(!App::path_time_label("src/app.rs").is_empty());
    }

    #[test]
    fn path_meta_lines_carry_rules_and_flags() {
        let mut app = App::default();
        assert_eq!(app.path_rules_label(), "");
        assert_eq!(app.path_flags_label(), String::new());
        app.set_tab(Tab::Text);
        assert_eq!(app.path_rules_label(), "Text: exact");
        assert_eq!(app.path_flags_label(), "wrap:off lines:on");
        let (left, right) = app.active_path_meta();
        assert!(left.contains("Text: exact"));
        assert!(right.contains("Text: exact"));
        app.set_tab(Tab::Merge);
        app.set_merge_paths("b", "l", "r");
        let (merged_left, merged_right) = app.active_path_meta();
        assert!(merged_left.contains(" || "));
        assert!(merged_right.contains("Merge: 3-way"));
    }

    #[test]
    fn split_fraction_defaults_centered_and_clamps() {
        let mut app = App::default();
        assert_eq!(app.split_fraction(), 0.50);
        app.set_split_fraction(0.0);
        assert_eq!(app.split_fraction(), 0.20);
        app.set_split_fraction(1.0);
        assert_eq!(app.split_fraction(), 0.80);
        app.set_split_fraction(0.60);
        assert_eq!(app.split_fraction(), 0.60);
    }

    #[test]
    fn split_drag_moves_fraction_within_range() {
        let mut app = App::default();
        app.drag_split_by(80.0, 800.0);
        assert_eq!(app.split_fraction(), 0.60);
        app.drag_split_by(-800.0, 800.0);
        assert_eq!(app.split_fraction(), 0.20);
        app.drag_split_by(800.0, 800.0);
        assert_eq!(app.split_fraction(), 0.80);
    }

    #[test]
    fn home_session_open_records_recent() {
        let mut app = App::default();
        assert!(app.recent_sessions().is_empty());
        assert!(!app.open_home_session(Tab::Home));
        assert_eq!(app.current_tab(), Tab::Home);
        assert!(app.open_home_session(Tab::Text));
        assert_eq!(app.current_tab(), Tab::Text);
        assert!(app.open_home_session(Tab::Folder));
        assert_eq!(app.current_tab(), Tab::Folder);
        let recent = app.recent_sessions();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent.first().map(String::as_str), Some("Folder"));
        assert_eq!(recent.get(1).map(String::as_str), Some("Text"));
        app.clear_recent_sessions();
        assert!(app.recent_sessions().is_empty());
    }

    #[test]
    fn bridge_derived_tab_empty_is_home() {
        let app = App::default();
        assert!(app.sessions.is_empty());
        assert_eq!(app.current_tab(), Tab::Home);
    }

    #[test]
    fn bridge_set_tab_switch_vs_open() {
        let mut app = App::default();
        app.set_tab(Tab::Text);
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.current_tab(), Tab::Text);
        app.set_text_paths("left.txt", "right.txt");
        app.set_tab(Tab::Folder);
        assert_eq!(app.sessions.len(), 2);
        assert_eq!(app.current_tab(), Tab::Folder);
        app.set_tab(Tab::Text);
        assert_eq!(app.sessions.len(), 2);
        assert_eq!(app.current_tab(), Tab::Text);
        assert_eq!(
            app.text_paths(),
            ("left.txt".to_string(), "right.txt".to_string())
        );
    }

    #[test]
    fn bridge_type_then_switch_preserves_paths_and_cursor() {
        let mut app = App::default();
        app.set_tab(Tab::Text);
        app.set_text_paths("a.txt", "b.txt");
        app.text_find_cursor = 4;
        app.set_tab(Tab::Folder);
        app.set_folder_paths("dir_a", "dir_b");
        app.set_tab(Tab::Text);
        assert_eq!(app.text_paths(), ("a.txt".to_string(), "b.txt".to_string()));
        assert_eq!(app.text_find_cursor, 4);
        let active = app.sessions.active().map(|item| item.kind);
        assert_eq!(active, Some(Tab::Text));
        let stored = app
            .sessions
            .list()
            .iter()
            .find(|item| item.kind == Tab::Folder)
            .map(|item| (item.left.clone(), item.right.clone()));
        assert_eq!(stored, Some(("dir_a".to_string(), "dir_b".to_string())));
    }

    #[test]
    fn bridge_close_neighbour_and_empty() {
        let mut app = App::default();
        app.set_tab(Tab::Text);
        app.set_tab(Tab::Folder);
        app.set_tab(Tab::Audio);
        assert_eq!(app.sessions.len(), 3);
        let active = match app.sessions.active_id() {
            Some(id) => id,
            None => return,
        };
        assert!(!app.close_session(999_999));
        assert!(app.close_session(active));
        assert_eq!(app.current_tab(), Tab::Folder);
        assert_eq!(app.sessions.len(), 2);
        let ids: Vec<u64> = app.sessions.list().iter().map(|item| item.id).collect();
        for id in ids {
            assert!(app.close_session(id));
        }
        assert!(app.sessions.is_empty());
        assert_eq!(app.current_tab(), Tab::Home);
    }

    #[test]
    fn bridge_open_home_session_creates_live_session() {
        let mut app = App::default();
        assert!(app.open_home_session(Tab::Text));
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.current_tab(), Tab::Text);
        app.set_text_paths("a.txt", "b.txt");
        assert!(app.open_home_session(Tab::Text));
        assert_eq!(app.sessions.len(), 2);
        assert_eq!(
            app.recent_sessions().first().map(String::as_str),
            Some("Text")
        );
        assert_eq!(app.session_count(), 2);
        assert!(app.status.contains("opened Text"));
    }

    #[test]
    fn bridge_cap_keeps_active_and_drops_oldest_inactive() {
        let mut app = App::default();
        let kinds = [
            Tab::Text,
            Tab::Folder,
            Tab::Hex,
            Tab::Merge,
            Tab::Table,
            Tab::Sync,
            Tab::Picture,
            Tab::FolderMerge,
            Tab::Audio,
        ];
        for kind in kinds {
            app.set_tab(kind);
        }
        assert_eq!(app.sessions.len(), home::HOME_RECENT_MAX);
        assert_eq!(app.current_tab(), Tab::Audio);
        let has_text = app
            .sessions
            .list()
            .iter()
            .any(|item| item.kind == Tab::Text);
        assert!(!has_text);
    }

    #[test]
    fn session_restore_reopens_paths_and_cursor_gracefully() {
        let mut app = App::default();
        assert_eq!(app.session_count(), 0);
        assert!(!app.reopen_session(0));
        app.set_tab(Tab::Text);
        app.set_text_paths("left.txt", "right.txt");
        app.text_find_cursor = 3;
        assert!(app.record_current_session());
        assert_eq!(app.session_count(), 1);
        app.set_text_paths("", "");
        app.text_find_cursor = 0;
        assert!(app.reopen_session(0));
        assert_eq!(app.current_tab(), Tab::Text);
        assert_eq!(
            app.text_paths(),
            ("left.txt".to_string(), "right.txt".to_string())
        );
        assert_eq!(app.text_find_cursor, 3);
        assert!(app.text_model.is_none());
        assert!(app.error.is_none());
        assert!(!app.reopen_session(7));
        app.clear_session_history();
        assert_eq!(app.session_count(), 0);
    }

    #[test]
    fn session_restore_empty_paths_opens_with_hint() {
        let mut app = App::default();
        app.set_tab(Tab::Folder);
        app.set_folder_paths("", "");
        assert!(app.record_current_session());
        app.set_folder_paths("dir_a", "dir_b");
        assert!(app.reopen_session(0));
        assert_eq!(app.current_tab(), Tab::Folder);
        assert_eq!(app.folder_paths(), (String::new(), String::new()));
        assert!(app.folder_model.is_none());
        assert!(app.status.contains("empty"));
    }

    #[test]
    fn session_restore_triple_paths_round_trip() {
        let mut app = App::default();
        app.set_tab(Tab::Merge);
        app.set_merge_paths("base.txt", "l.txt", "r.txt");
        assert!(app.record_current_session());
        app.set_merge_paths("", "", "");
        assert!(app.reopen_session(0));
        assert_eq!(app.current_tab(), Tab::Merge);
        assert_eq!(
            app.merge_paths(),
            (
                "base.txt".to_string(),
                "l.txt".to_string(),
                "r.txt".to_string()
            )
        );
        assert!(app.merge_model.is_none());
    }

    #[test]
    fn homerecent_stamped_record_carries_timestamp_and_meta() {
        let mut app = App::default();
        assert_eq!(
            app.recent_empty_hint(),
            "no recent sessions yet: open any compare above to pin it here"
        );
        app.set_tab(Tab::Text);
        app.set_text_paths("left.txt", "right.txt");
        assert!(app.record_current_session_at(1_000));
        assert_eq!(app.session_count(), 1);
        assert_eq!(
            app.session_age_label(0, 1_060),
            Some(String::from("1m ago"))
        );
        assert!(app.session_age_label(7, 1_060).is_none());
        let meta = app.session_row_meta(0, 1_060).unwrap_or_default();
        assert!(meta.contains("Compare to refresh"));
        assert!(meta.contains("1m ago"));
        assert_eq!(
            app.session_row_status(0),
            Some("paths restored: Compare to refresh")
        );
        assert!(app.session_row_status(7).is_none());
        assert_eq!(
            app.recent_empty_hint(),
            "no recent sessions yet: open any compare above to pin it here"
        );
        app.clear_session_history();
        assert_eq!(app.session_count(), 0);
        assert_eq!(
            app.recent_empty_hint(),
            "recent cleared: open any compare above to pin it here"
        );
        assert!(app.session_age_label(0, 2_000).is_none());
    }

    #[test]
    fn homerecent_unknown_time_hides_timestamp_but_keeps_status() {
        let mut app = App::default();
        app.set_tab(Tab::Folder);
        app.set_folder_paths("", "");
        assert!(app.record_current_session());
        assert!(app.session_age_label(0, 9_999).is_none());
        let meta = app.session_row_meta(0, 9_999).unwrap_or_default();
        assert!(meta.contains("paths missing"));
        assert!(!meta.contains("ago"));
        assert!(!meta.contains("just now"));
        assert_eq!(
            app.session_row_status(0),
            Some("paths missing: type both paths then Compare")
        );
        app.clear_recent_sessions();
        assert!(app.recent_sessions().is_empty());
        assert!(app.recent_empty_hint().contains("recent cleared"));
        app.set_tab(Tab::Text);
        assert!(app.open_home_session_at(Tab::Text, 5_000));
        assert!(app.recent_empty_hint().contains("no recent sessions yet"));
        assert_eq!(
            app.session_age_label(0, 5_030),
            Some(String::from("just now"))
        );
    }

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

    fn merge_conflicted_fixture() -> MergeSummary {
        viewmodel::build_merge_summary(&[
            "base line".to_string(),
            "<<<<<<< left".to_string(),
            "left change".to_string(),
            "||||||| base".to_string(),
            "base change".to_string(),
            "=======".to_string(),
            "right change".to_string(),
            ">>>>>>> right".to_string(),
            "tail".to_string(),
        ])
    }

    #[test]
    fn merge_decide_persists_last_wins() {
        let mut app = App {
            merge_model: Some(merge_conflicted_fixture()),
            ..Default::default()
        };
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Left));
        assert_eq!(
            app.merge_pick_for_hunk(0),
            Some(viewmodel::TakeChoice::Left)
        );
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Right));
        assert_eq!(
            app.merge_pick_for_hunk(0),
            Some(viewmodel::TakeChoice::Right)
        );
        assert_eq!(app.merge_picks.len(), 1);
    }

    #[test]
    fn merge_decide_unknown_hunk_ignored() {
        let mut app = App::default();
        assert!(!app.decide_merge_hunk(0, viewmodel::TakeChoice::Left));
        assert!(app.merge_picks.is_empty());
        assert_eq!(app.merge_pick_for_hunk(0), None);
        app.merge_model = Some(merge_conflicted_fixture());
        assert!(!app.decide_merge_hunk(7, viewmodel::TakeChoice::Right));
        assert!(app.merge_picks.is_empty());
        assert_eq!(app.merge_pick_for_hunk(7), None);
    }

    #[test]
    fn merge_pick_for_hunk_returns_last_staged() {
        let mut app = App {
            merge_model: Some(merge_conflicted_fixture()),
            ..Default::default()
        };
        assert_eq!(app.merge_pick_for_hunk(0), None);
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Base));
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Both));
        assert_eq!(
            app.merge_pick_for_hunk(0),
            Some(viewmodel::TakeChoice::Both)
        );
    }

    #[test]
    fn merge_pick_summary_reflects_app_picks() {
        let mut app = App::default();
        let empty = app.merge_pick_summary();
        assert_eq!(empty.total, 0);
        assert_eq!(empty.resolved, 0);
        assert_eq!(empty.pending, 0);
        app.merge_model = Some(merge_conflicted_fixture());
        let pending = app.merge_pick_summary();
        assert_eq!(pending.total, 1);
        assert_eq!(pending.resolved, 0);
        assert_eq!(pending.pending, 1);
        assert!(pending.conflicted);
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Left));
        let decided = app.merge_pick_summary();
        assert_eq!(decided.total, 1);
        assert_eq!(decided.resolved, 1);
        assert_eq!(decided.pending, 0);
        assert!(!decided.conflicted);
    }

    #[test]
    fn merge_summary_line_counts_real_picks() {
        let mut app = App::default();
        assert_eq!(app.merge_summary_line(), "merge: no results yet");
        app.merge_model = Some(merge_conflicted_fixture());
        let before = app.merge_summary_line();
        assert!(before.contains("0/1 decided"));
        assert!(before.contains("1 pending"));
        assert!(before.contains("conflicted"));
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Right));
        let after = app.merge_summary_line();
        assert!(after.contains("1/1 decided"));
        assert!(after.contains("0 pending"));
        assert!(!after.contains("conflicted"));
    }

    #[test]
    fn merge_swap_and_recompare_clear_picks() {
        let mut app = App::default();
        app.set_tab(Tab::Merge);
        app.set_merge_paths("b.txt", "l.txt", "r.txt");
        app.merge_model = Some(merge_conflicted_fixture());
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Left));
        assert_eq!(app.merge_picks.len(), 1);
        assert!(app.swap_sides());
        assert!(app.merge_picks.is_empty());
        assert!(app.merge_model.is_none());
        app.merge_model = Some(merge_conflicted_fixture());
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Right));
        app.clear_merge_picks();
        assert!(app.merge_picks.is_empty());
        assert_eq!(app.merge_pick_summary().resolved, 0);
    }

    /// Paint bridge fixture: do well-formed conflict hunks.
    fn merge_bridge_fixture_lines() -> Vec<String> {
        vec![
            "clean header".to_string(),
            "<<<<<<< left".to_string(),
            "left one".to_string(),
            "=======".to_string(),
            "right one".to_string(),
            ">>>>>>> right".to_string(),
            "clean middle".to_string(),
            "<<<<<<< left".to_string(),
            "left two".to_string(),
            "=======".to_string(),
            "right two".to_string(),
            ">>>>>>> right".to_string(),
            "clean footer".to_string(),
        ]
    }

    #[test]
    fn merge_adopt_dedups_and_prunes_unknown() {
        let mut app = App::default();
        let lines = merge_bridge_fixture_lines();
        app.merge_model = Some(viewmodel::build_merge_summary(&lines));
        let staged = vec![
            viewmodel::MergeHunkPick {
                conflict_index: 0,
                choice: viewmodel::TakeChoice::Left,
            },
            viewmodel::MergeHunkPick {
                conflict_index: 0,
                choice: viewmodel::TakeChoice::Right,
            },
            viewmodel::MergeHunkPick {
                conflict_index: 9,
                choice: viewmodel::TakeChoice::Left,
            },
            viewmodel::MergeHunkPick {
                conflict_index: 1,
                choice: viewmodel::TakeChoice::Base,
            },
        ];
        app.adopt_merge_picks(&lines, staged);
        assert_eq!(app.merge_picks.len(), 2);
        assert_eq!(
            app.merge_pick_for_hunk(0),
            Some(viewmodel::TakeChoice::Right)
        );
        assert_eq!(
            app.merge_pick_for_hunk(1),
            Some(viewmodel::TakeChoice::Base)
        );
        assert_eq!(app.merge_pick_summary().resolved, 2);
    }

    #[test]
    fn merge_reconcile_clears_on_lines_change() {
        let mut app = App::default();
        let lines = merge_bridge_fixture_lines();
        app.merge_model = Some(viewmodel::build_merge_summary(&lines));
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Left));
        let staged = app.merge_picks.clone();
        let mut changed = lines.clone();
        changed.push("extra".to_string());
        app.reconcile_merge_picks(&lines, &changed, staged);
        assert!(app.merge_picks.is_empty());
        assert!(app.status.contains("staged picks cleared"));
    }

    #[test]
    fn merge_reconcile_keeps_picks_without_change() {
        let mut app = App::default();
        let lines = merge_bridge_fixture_lines();
        app.merge_model = Some(viewmodel::build_merge_summary(&lines));
        assert!(app.decide_merge_hunk(1, viewmodel::TakeChoice::Both));
        let staged = app.merge_picks.clone();
        app.reconcile_merge_picks(&lines, &lines, staged);
        assert_eq!(app.merge_picks.len(), 1);
        assert_eq!(
            app.merge_pick_for_hunk(1),
            Some(viewmodel::TakeChoice::Both)
        );
    }

    /// Merge apply fixture: header, one full-marker conflict block
    /// (left/base/right sections), footer.
    fn merge_apply_fixture() -> MergeSummary {
        viewmodel::build_merge_summary(&[
            "header".to_string(),
            "<<<<<<< left".to_string(),
            "left change".to_string(),
            "||||||| base".to_string(),
            "base change".to_string(),
            "=======".to_string(),
            "right change".to_string(),
            ">>>>>>> right".to_string(),
            "footer".to_string(),
        ])
    }

    fn merge_apply_expected(choice: viewmodel::TakeChoice) -> Vec<String> {
        let mut out = vec!["header".to_string()];
        match choice {
            viewmodel::TakeChoice::Left => out.push("left change".to_string()),
            viewmodel::TakeChoice::Right => out.push("right change".to_string()),
            viewmodel::TakeChoice::Base => out.push("base change".to_string()),
            viewmodel::TakeChoice::Both => {
                out.push("left change".to_string());
                out.push("right change".to_string());
            }
        }
        out.push("footer".to_string());
        out
    }

    static MERGE_APPLY_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    /// Temp dir ke andar merge output file path deta hai. Dir pehle se
    /// bani hoti hai taaki real write seedha chal sake. Fail par `None`.
    fn merge_apply_test_path(tag: &str) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        let n = MERGE_APPLY_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("am-apply-{}-{n}-{tag}", std::process::id()));
        if std::fs::create_dir_all(&dir).is_err() {
            return None;
        }
        let out = dir.join("merged.txt");
        Some((dir, out))
    }

    fn remove_merge_apply_tree(dir: &std::path::Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn read_merge_apply_file(path: &std::path::Path) -> String {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(_) => String::from("<missing>"),
        }
    }

    #[test]
    fn merge_apply_takes_left_right_base_both() {
        for choice in [
            viewmodel::TakeChoice::Left,
            viewmodel::TakeChoice::Right,
            viewmodel::TakeChoice::Base,
            viewmodel::TakeChoice::Both,
        ] {
            let mut app = App {
                merge_model: Some(merge_apply_fixture()),
                ..Default::default()
            };
            assert!(app.decide_merge_hunk(0, choice));
            let Ok(lines) = app.merge_resolved_lines() else {
                continue;
            };
            assert_eq!(lines, merge_apply_expected(choice));
        }
    }

    #[test]
    fn merge_apply_undecided_keeps_markers() {
        let app = App {
            merge_model: Some(merge_apply_fixture()),
            ..Default::default()
        };
        let Ok(lines) = app.merge_resolved_lines() else {
            return;
        };
        assert!(lines.iter().any(|line| line.starts_with("<<<<<<<")));
        assert!(lines.iter().any(|line| line.starts_with(">>>>>>>")));
        assert_eq!(app.merge_pick_summary().pending, 1);
    }

    #[test]
    fn merge_apply_without_model_reports_compare_first() {
        let app = App::default();
        assert!(app.merge_model.is_none());
        let Err(err) = app.merge_resolved_lines() else {
            return;
        };
        assert!(err.contains("compare first"));
        assert_eq!(app.merge_dry_run_line(), "merge apply: compare first");
        let mut app = app;
        assert!(!app.run_merge_apply());
        let Some(report) = app.merge_apply_last_report.clone() else {
            return;
        };
        assert!(report.contains("compare first"));
    }

    #[test]
    fn merge_apply_dry_run_default_reports_without_writes() {
        let Some((dir, out)) = merge_apply_test_path("dry") else {
            return;
        };
        let mut app = App::default();
        assert!(app.merge_dry_run);
        assert!(app.merge_apply_last_report.is_none());
        app.merge_model = Some(merge_apply_fixture());
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Left));
        app.set_merge_output_path(out.to_string_lossy().into_owned());
        assert!(!app.merge_should_write());
        assert!(app.run_merge_apply());
        let Some(report) = app.merge_apply_last_report.clone() else {
            remove_merge_apply_tree(&dir);
            return;
        };
        assert!(report.contains("no writes"), "unexpected: {report}");
        assert!(report.contains("1/1 decided"), "unexpected: {report}");
        assert_eq!(read_merge_apply_file(&out), "<missing>");
        remove_merge_apply_tree(&dir);
    }

    #[test]
    fn merge_apply_real_write_end_to_end() {
        let Some((dir, out)) = merge_apply_test_path("e2e") else {
            return;
        };
        let mut app = App {
            merge_model: Some(merge_apply_fixture()),
            ..Default::default()
        };
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Both));
        app.set_merge_output_path(out.to_string_lossy().into_owned());
        assert!(!app.merge_should_write());
        app.set_merge_dry_run(false);
        assert!(app.merge_should_write());
        assert!(app.run_merge_apply());
        let Some(report) = app.merge_apply_last_report.clone() else {
            remove_merge_apply_tree(&dir);
            return;
        };
        assert!(report.contains("merge apply done"), "unexpected: {report}");
        assert!(report.contains("4 line(s)"), "unexpected: {report}");
        assert_eq!(
            read_merge_apply_file(&out),
            "header\nleft change\nright change\nfooter\n"
        );
        remove_merge_apply_tree(&dir);
    }

    #[test]
    fn merge_apply_missing_output_path_writes_nothing() {
        let mut app = App {
            merge_model: Some(merge_apply_fixture()),
            ..Default::default()
        };
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Right));
        app.set_merge_dry_run(false);
        assert!(!app.merge_should_write());
        assert!(!app.run_merge_apply());
        let Some(report) = app.merge_apply_last_report.clone() else {
            return;
        };
        assert!(report.contains("output path"));
    }

    #[test]
    fn merge_apply_io_error_graceful_on_dir_path() {
        let Some((dir, out)) = merge_apply_test_path("ioerr") else {
            return;
        };
        // Output path par dir banana: `fs::write` fail hoga, graceful Err.
        let clash = dir.join("clash");
        if std::fs::create_dir_all(&clash).is_err() {
            remove_merge_apply_tree(&dir);
            return;
        }
        let mut app = App {
            merge_model: Some(merge_apply_fixture()),
            ..Default::default()
        };
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Base));
        app.set_merge_output_path(clash.to_string_lossy().into_owned());
        app.set_merge_dry_run(false);
        assert!(app.merge_should_write());
        assert!(!app.run_merge_apply());
        let Some(report) = app.merge_apply_last_report.clone() else {
            remove_merge_apply_tree(&dir);
            return;
        };
        assert!(
            report.contains("merge apply failed"),
            "unexpected: {report}"
        );
        assert_eq!(read_merge_apply_file(&out), "<missing>");
        remove_merge_apply_tree(&dir);
    }

    #[test]
    fn merge_dry_run_summary_honest_counts() {
        let line = App::merge_dry_run_summary(9, 1, 2, 1);
        assert!(line.contains("9 line(s) output"));
        assert!(line.contains("1/2 decided"));
        assert!(line.contains("1 pending"));
        assert!(line.contains("no writes"));
        let done = App::describe_merge_apply_report(4, "out.txt");
        assert!(done.contains("4 line(s)"));
        assert!(done.contains("out.txt"));
        assert_eq!(App::join_merge_output(&[]), String::new());
        assert_eq!(
            App::join_merge_output(&["a".to_string(), "b".to_string()]),
            "a\nb\n"
        );
    }

    #[test]
    fn merge_apply_state_defaults_idle_without_request() {
        let app = App::default();
        assert!(!app.merge_apply_requested);
        assert_eq!(app.merge_apply_state, MergeApplyState::Idle);
        assert!(app.merge_apply_last_summary.is_none());
        assert!(app.merge_apply_last_report.is_none());
        assert!(!app.merge_apply_enabled());
        assert_eq!(app.merge_apply_summary_line(), "merge apply: compare first");
    }

    #[test]
    fn merge_apply_request_take_roundtrip() {
        let mut app = App::default();
        assert!(app.request_merge_apply());
        assert!(app.merge_apply_requested);
        assert!(app.take_merge_apply_request());
        assert!(!app.merge_apply_requested);
        assert!(!app.take_merge_apply_request());
        app.request_merge_apply();
        app.clear_merge_apply_request();
        assert!(!app.merge_apply_requested);
    }

    #[test]
    fn merge_apply_enabled_needs_model() {
        let mut app = App::default();
        assert!(!app.merge_apply_enabled());
        app.merge_model = Some(merge_apply_fixture());
        assert!(app.merge_apply_enabled());
    }

    #[test]
    fn merge_apply_dry_run_sets_reported_state_and_summary() {
        let Some((dir, out)) = merge_apply_test_path("an-dry") else {
            return;
        };
        let mut app = App {
            merge_model: Some(merge_apply_fixture()),
            ..Default::default()
        };
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Left));
        app.set_merge_output_path(out.to_string_lossy().into_owned());
        assert!(app.request_merge_apply());
        assert!(app.run_merge_apply());
        assert!(!app.merge_apply_requested);
        assert_eq!(app.merge_apply_state, MergeApplyState::DryRunReported);
        let Some(summary) = app.merge_apply_last_summary.clone() else {
            remove_merge_apply_tree(&dir);
            return;
        };
        assert!(summary.dry_run);
        assert_eq!((summary.decided, summary.total, summary.pending), (1, 1, 0));
        assert_eq!(summary.resolved_lines, 3);
        assert_eq!(summary.wrote_lines, 0);
        let Some(report) = app.merge_apply_last_report.clone() else {
            remove_merge_apply_tree(&dir);
            return;
        };
        assert_eq!(app.merge_apply_summary_line(), report);
        assert!(report.contains("no writes"));
        assert_eq!(read_merge_apply_file(&out), "<missing>");
        remove_merge_apply_tree(&dir);
    }

    #[test]
    fn merge_apply_real_write_sets_wrote_state_and_summary() {
        let Some((dir, out)) = merge_apply_test_path("an-wrote") else {
            return;
        };
        let mut app = App {
            merge_model: Some(merge_apply_fixture()),
            ..Default::default()
        };
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Both));
        app.set_merge_output_path(out.to_string_lossy().into_owned());
        app.set_merge_dry_run(false);
        assert!(app.run_merge_apply());
        assert_eq!(app.merge_apply_state, MergeApplyState::Wrote);
        let Some(summary) = app.merge_apply_last_summary.clone() else {
            remove_merge_apply_tree(&dir);
            return;
        };
        assert!(!summary.dry_run);
        assert_eq!(summary.wrote_lines, 4);
        assert_eq!(summary.resolved_lines, 4);
        assert_eq!(
            read_merge_apply_file(&out),
            "header\nleft change\nright change\nfooter\n"
        );
        remove_merge_apply_tree(&dir);
    }

    #[test]
    fn merge_apply_missing_path_sets_needs_path() {
        let mut app = App {
            merge_model: Some(merge_apply_fixture()),
            ..Default::default()
        };
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Right));
        app.set_merge_dry_run(false);
        assert!(!app.run_merge_apply());
        assert_eq!(app.merge_apply_state, MergeApplyState::NeedsPath);
        let Some(summary) = app.merge_apply_last_summary.clone() else {
            return;
        };
        assert!(!summary.dry_run);
        assert_eq!(summary.wrote_lines, 0);
        assert!(summary.out.is_empty());
        assert!(app.merge_apply_summary_line().contains("output path"));
    }

    #[test]
    fn merge_apply_without_model_sets_no_model() {
        let mut app = App::default();
        assert!(app.request_merge_apply());
        assert!(!app.run_merge_apply());
        assert_eq!(app.merge_apply_state, MergeApplyState::NoModel);
        assert!(!app.merge_apply_requested);
        assert!(app.merge_apply_last_summary.is_some());
        assert_eq!(app.merge_apply_summary_line(), "merge apply: compare first");
    }

    #[test]
    fn merge_apply_io_error_sets_failed_state() {
        let Some((dir, _out)) = merge_apply_test_path("an-fail") else {
            return;
        };
        let clash = dir.join("clash");
        if std::fs::create_dir_all(&clash).is_err() {
            remove_merge_apply_tree(&dir);
            return;
        }
        let mut app = App {
            merge_model: Some(merge_apply_fixture()),
            ..Default::default()
        };
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Base));
        app.set_merge_output_path(clash.to_string_lossy().into_owned());
        app.set_merge_dry_run(false);
        assert!(!app.run_merge_apply());
        assert_eq!(app.merge_apply_state, MergeApplyState::Failed);
        let Some(summary) = app.merge_apply_last_summary.clone() else {
            remove_merge_apply_tree(&dir);
            return;
        };
        assert_eq!(summary.wrote_lines, 0);
        assert!(!summary.out.is_empty());
        remove_merge_apply_tree(&dir);
    }

    #[test]
    fn merge_apply_state_clears_on_swap_and_retry() {
        let mut app = App {
            merge_model: Some(merge_apply_fixture()),
            ..Default::default()
        };
        app.set_tab(Tab::Merge);
        app.set_merge_paths("b.txt", "l.txt", "r.txt");
        assert!(app.decide_merge_hunk(0, viewmodel::TakeChoice::Left));
        assert!(app.run_merge_apply());
        assert_eq!(app.merge_apply_state, MergeApplyState::DryRunReported);
        assert!(app.swap_sides());
        assert!(!app.merge_apply_requested);
        assert_eq!(app.merge_apply_state, MergeApplyState::Idle);
        assert!(app.merge_apply_last_summary.is_none());
        assert!(app.merge_apply_last_report.is_none());
        app.clear_merge_apply_state();
        assert_eq!(app.merge_apply_state, MergeApplyState::Idle);
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
    fn dark_mode_storage_parse_trims_and_defaults_dark() {
        assert!(App::dark_mode_from_storage(None));
        assert!(App::dark_mode_from_storage(Some("")));
        assert!(App::dark_mode_from_storage(Some("  ")));
        assert!(App::dark_mode_from_storage(Some("1")));
        assert!(App::dark_mode_from_storage(Some(" true ")));
        assert!(App::dark_mode_from_storage(Some("dark")));
        assert!(App::dark_mode_from_storage(Some("bogus-value")));
        assert!(!App::dark_mode_from_storage(Some("0")));
        assert!(!App::dark_mode_from_storage(Some(" false ")));
        assert!(!App::dark_mode_from_storage(Some("light")));
    }

    #[test]
    fn dark_mode_storage_roundtrip_through_save() {
        use eframe::App as _;
        use std::collections::HashMap;

        struct FakeStorage {
            map: HashMap<String, String>,
        }

        impl eframe::Storage for FakeStorage {
            fn get_string(&self, key: &str) -> Option<String> {
                self.map.get(key).cloned()
            }

            fn set_string(&mut self, key: &str, value: String) {
                self.map.insert(key.to_string(), value);
            }

            fn remove_string(&mut self, key: &str) {
                self.map.remove(key);
            }

            fn flush(&mut self) {}
        }

        let mut app = App::default();
        assert!(app.dark_mode);
        let mut storage = FakeStorage {
            map: HashMap::new(),
        };
        app.save(&mut storage);
        assert_eq!(
            storage.map.get(App::DARK_MODE_STORAGE_KEY),
            Some(&String::from("1"))
        );
        app.dark_mode = false;
        app.save(&mut storage);
        let stored = storage.map.get(App::DARK_MODE_STORAGE_KEY).cloned();
        assert!(!App::dark_mode_from_storage(stored.as_deref()));
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
    fn folder_dual_rows_apply_display_filters() {
        let mut app = folder_app_with_rows();
        // Bina filter: dual ko teeno rows milti hain, order same.
        let dual = app.folder_dual_rows();
        assert_eq!(dual.len(), 3);
        let Some(last) = dual.get(2) else {
            return;
        };
        assert_eq!(last.rel_path, "orphan.txt");
        // Hide orphans: dual panes se orphan gayab.
        app.folder_session.display.hide_orphans = true;
        let dual = app.folder_dual_rows();
        assert_eq!(dual.len(), 2);
        assert!(dual.iter().all(|r| r.glyph != 'R'));
        // Summary filtered tree par: orphans chhupne par ginti 2.
        let summary = app.folder_counts_summary();
        assert!(summary.contains("2 entries"));
        assert!(summary.contains("0 right-only"));
        // Sync total dual rows se milta hai.
        app.sync_folder_dual();
        assert_eq!(
            app.folder_dual.pager.total_rows,
            app.folder_visible_tree().len()
        );
    }

    #[test]
    fn folder_name_filter_narrows_tree_and_counts_honestly() {
        let mut app = folder_app_with_rows();
        app.toggle_folder_dir("src");
        assert_eq!(app.folder_visible_tree().len(), 3);
        assert!(!app.folder_has_active_filters());
        assert_eq!(app.folder_active_filter_count(), 0);
        app.set_folder_name_filter("src/");
        assert_eq!(app.folder_name_filter(), "src/");
        assert!(app.folder_has_active_filters());
        assert_eq!(app.folder_active_filter_count(), 1);
        assert_eq!(app.folder_visible_tree().len(), 1);
        let summary = app.folder_counts_summary();
        assert!(summary.contains("1 entries"));
        assert!(app.status.contains("src/"));
        // Khali query = saaf.
        app.set_folder_name_filter("   ");
        assert!(!app.folder_has_active_filters());
        assert_eq!(app.folder_visible_tree().len(), 3);
    }

    #[test]
    fn folder_kind_and_status_filters_narrow_really() {
        let mut app = folder_app_with_rows();
        app.toggle_folder_dir("src");
        app.set_folder_entry_kind(crate::tree::FolderEntryKind::DirsOnly);
        assert_eq!(
            app.folder_entry_kind(),
            crate::tree::FolderEntryKind::DirsOnly
        );
        assert_eq!(app.folder_visible_tree().len(), 1);
        app.set_folder_entry_kind(crate::tree::FolderEntryKind::All);
        app.set_folder_status_filter(crate::tree::FolderStatusFilter::NewerLeft);
        assert_eq!(
            app.folder_status_filter(),
            crate::tree::FolderStatusFilter::NewerLeft
        );
        assert_eq!(app.folder_visible_tree().len(), 1);
        let summary = app.folder_counts_summary();
        assert!(summary.contains("1 entries"));
        assert!(summary.contains("1 newer left"));
        // Legacy checkbox setters bhi sync karte hain.
        app.set_folder_status_filter(crate::tree::FolderStatusFilter::All);
        app.set_folder_hide_orphans(true);
        assert_eq!(app.folder_visible_tree().len(), 2);
        app.set_folder_left_newer_only(true);
        assert_eq!(app.folder_visible_tree().len(), 1);
        app.set_folder_left_newer_only(false);
        app.set_folder_right_newer_only(true);
        assert_eq!(app.folder_visible_tree().len(), 0);
        assert_eq!(
            app.folder_empty_state(),
            Some(crate::tree::FolderEmptyState::NoMatch)
        );
        assert!(app.folder_empty_hint().contains("clear filters"));
        // Flatten setter + clear sab wapas lata hai.
        app.set_folder_right_newer_only(false);
        app.set_folder_hide_orphans(false);
        app.set_folder_flatten(true);
        assert_eq!(app.folder_visible_tree().len(), 3);
        app.clear_folder_filters();
        assert!(!app.folder_has_active_filters());
        assert_eq!(app.folder_active_filter_count(), 0);
        assert_eq!(app.folder_visible_tree().len(), 3);
        assert!(app.status.contains("cleared"));
    }

    #[test]
    fn folder_empty_full_rows_routes_identical_vs_pruned() {
        let mut empty = App::default();
        empty.set_tab(Tab::Folder);
        assert_eq!(
            empty.folder_empty_state_full_rows(),
            Some(crate::tree::FolderEmptyState::NoResults)
        );
        let mut app = App::default();
        app.set_tab(Tab::Folder);
        app.folder_model = Some((
            vec![folder_row("same.txt", false, '=')],
            String::from("1 entries"),
        ));
        app.set_folder_status_filter(crate::tree::FolderStatusFilter::Different);
        assert_eq!(app.folder_visible_tree().len(), 0);
        assert_eq!(
            app.folder_empty_state_full_rows(),
            Some(crate::tree::FolderEmptyState::AllSame)
        );
        assert!(app.folder_empty_hint_full_rows().contains("all same"));
    }

    #[test]
    fn folder_empty_hints_separate_no_results_from_no_match() {
        let mut empty = App::default();
        empty.set_tab(Tab::Folder);
        assert_eq!(
            empty.folder_empty_state(),
            Some(crate::tree::FolderEmptyState::NoResults)
        );
        assert!(empty.folder_empty_hint().contains("no results yet"));
        assert!(empty.folder_counts_summary().contains("no results yet"));
        let mut app = folder_app_with_rows();
        app.toggle_folder_dir("src");
        assert_eq!(app.folder_empty_state(), None);
        assert!(app.folder_empty_hint().is_empty());
        app.set_folder_name_filter("zzz-no-such-file");
        assert_eq!(
            app.folder_empty_state(),
            Some(crate::tree::FolderEmptyState::NoMatch)
        );
        assert!(app.folder_empty_hint().contains("no rows match filters"));
        assert!(app.folder_counts_summary().contains("0 entries"));
    }

    #[test]
    fn folder_sync_badges_follow_preview_mode() {
        use crate::tree::SyncRowDirection;
        let mut app = folder_app_with_rows();
        // UpdateBoth: left-newer L->R, right-only R->L.
        assert_eq!(
            app.folder_sync_direction("src/a.txt"),
            SyncRowDirection::CopyToRight
        );
        assert_eq!(app.folder_sync_badge("src/a.txt"), "->");
        assert_eq!(
            app.folder_sync_direction("orphan.txt"),
            SyncRowDirection::CopyToLeft
        );
        assert_eq!(app.folder_sync_badge("orphan.txt"), "<-");
        // Dir same: koi action nahi.
        assert_eq!(app.folder_sync_direction("src"), SyncRowDirection::None);
        assert_eq!(app.folder_sync_badge("src"), "=");
        // Unknown path: honest none.
        assert_eq!(
            app.folder_sync_direction("ghost.txt"),
            SyncRowDirection::None
        );
        // Mirror me right-only prune badge.
        app.set_folder_sync_mode(crate::tree::SyncModeChoice::Mirror);
        assert_eq!(
            app.folder_sync_direction("orphan.txt"),
            SyncRowDirection::DeleteRight
        );
        assert_eq!(app.folder_sync_badge("orphan.txt"), "X");
        // Bina model: hamesha none.
        let empty = App::default();
        assert_eq!(
            empty.folder_sync_direction("src/a.txt"),
            SyncRowDirection::None
        );
    }

    #[test]
    fn folder_sync_selection_holds_only_real_rows() {
        let mut app = folder_app_with_rows();
        assert!(app.folder_sync_selection.is_empty());
        assert!(app.select_folder_sync_row("src/a.txt"));
        assert!(!app.select_folder_sync_row("ghost.txt"));
        assert!(app.folder_sync_selection.contains("src/a.txt"));
        // Toggle flip karta hai, unknown par false.
        assert!(!app.toggle_folder_sync_row("src/a.txt"));
        assert!(!app.folder_sync_selection.contains("src/a.txt"));
        assert!(app.toggle_folder_sync_row("orphan.txt"));
        assert!(!app.toggle_folder_sync_row("ghost.txt"));
        // Scoped preview sirf selected rows ka.
        let scoped = app.folder_sync_selected_preview();
        assert_eq!(scoped.steps.len(), 1);
        let Some(step) = scoped.steps.first() else {
            return;
        };
        assert_eq!(step.rel_path, "orphan.txt");
        // Summary lines real counts bolti hain.
        let full = app.folder_sync_summary_line();
        assert!(full.contains("L->R"));
        assert!(full.contains("R->L"));
        let sel_line = app.folder_sync_selected_summary_line();
        assert!(sel_line.contains("1 selected"));
        // Clear + swap selection girate hain.
        app.clear_folder_sync_selection();
        assert!(app.folder_sync_selection.is_empty());
        assert!(app.select_folder_sync_row("src/a.txt"));
        app.swap_sides();
        assert!(app.folder_sync_selection.is_empty());
        assert!(app.folder_sync_selected_preview().is_empty());
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
    fn folder_dual_default_empty_and_synced_shape() {
        let app = App::default();
        assert_eq!(app.folder_dual.selected, None);
        assert_eq!(app.folder_dual.pending, None);
        assert_eq!(app.folder_dual.visible_range(), (0, 0));
        assert_eq!(app.folder_dual.pager.rows_per_page, 20);
    }

    #[test]
    fn folder_dual_syncs_total_from_results() {
        let mut app = folder_app_with_rows();
        app.toggle_folder_dir("src");
        app.sync_folder_dual();
        assert_eq!(app.folder_visible_tree().len(), 3);
        assert_eq!(app.folder_dual.pager.total_rows, 3);
        assert_eq!(app.folder_dual.visible_range(), (0, 3));
    }

    #[test]
    fn folder_dual_select_shows_in_both_panes() {
        let mut app = folder_app_with_rows();
        app.folder_dual.select("src/a.txt");
        assert!(app.folder_dual.is_selected("src/a.txt"));
        assert!(!app.folder_dual.is_selected("orphan.txt"));
        app.folder_dual.clear_selection();
        assert!(!app.folder_dual.is_selected("src/a.txt"));
    }

    #[test]
    fn folder_dual_resets_on_swap() {
        let mut app = folder_app_with_rows();
        app.set_folder_paths("left-dir", "right-dir");
        app.sync_folder_dual();
        app.folder_dual.select("src/a.txt");
        app.folder_dual
            .stage_action(crate::tree::DualRowAction::Compare, "src/a.txt");
        assert!(app.swap_sides());
        assert_eq!(app.folder_dual.selected, None);
        assert_eq!(app.folder_dual.pending, None);
        assert_eq!(app.folder_dual.pager.total_rows, 0);
        assert_eq!(app.folder_dual.pager.rows_per_page, 20);
    }

    #[test]
    fn folder_dual_reset_clears_stale_stub() {
        let mut app = folder_app_with_rows();
        app.sync_folder_dual();
        assert_eq!(app.folder_dual.pager.total_rows, 2);
        app.folder_dual
            .stage_action(crate::tree::DualRowAction::CopyLeftToRight, "orphan.txt");
        app.reset_folder_dual();
        assert_eq!(app.folder_dual.pending, None);
        assert_eq!(app.folder_dual.pager.total_rows, 0);
        assert_eq!(app.folder_dual.visible_range(), (0, 0));
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
        assert!(app.hex_bytes.is_none());
        assert!(app.hex_view.hunks.is_empty());
        assert!(app.status.contains("Fast"));
        // Same mode again: no wipe, no-op.
        app.hex_model = Some(vec![String::from("left: 1 bytes")]);
        app.set_hex_mode(HexCompareMode::Fast);
        assert!(app.hex_model.is_some());
    }

    #[test]
    fn hex_byte_snap_slices_feed_nibble_cells() {
        // REAL slices: low nibble alag (0xAB vs 0xAC), high ek jaisa.
        let snap = HexByteSnap {
            left: vec![0xAB],
            right: vec![0xAC],
        };
        let (left, right) = snap.as_slices();
        let cells = crate::views_hex::hex_diff_nibble_cells(0, left, right);
        assert_eq!(cells.len(), 1);
        assert!(
            cells
                .first()
                .is_some_and(|cell| cell.col == 0 && !cell.high_diff && cell.low_diff)
        );
        // Barabar bytes par koi cell nahi.
        let same = crate::views_hex::hex_diff_nibble_cells(0, left, left);
        assert!(same.is_empty());
    }

    #[test]
    fn hex_byte_snap_none_keeps_old_empty_path() {
        // `None` snap = purana offset-only behavior: khali slices se koi
        // accent cell nahi (callsite fallback parity).
        let bytes: Option<HexByteSnap> = None;
        let (left, right): (&[u8], &[u8]) = match bytes.as_ref().map(HexByteSnap::as_slices) {
            Some((l, r)) => (l, r),
            None => (&[], &[]),
        };
        assert!(crate::views_hex::hex_diff_nibble_cells(0, left, right).is_empty());
    }

    #[test]
    fn hex_success_sets_and_resets_clear_byte_snap() {
        use crate::views_hex::{HexCompareMode, HexCompareView};
        let mut app = App::default();
        let view = HexCompareView::build_from_slices(&[1u8, 2], &[1u8, 3], HexCompareMode::None, 8);
        let snap = HexByteSnap {
            left: vec![1u8, 2],
            right: vec![1u8, 3],
        };
        app.set_hex_success(vec![String::from("hex: 1 hunk")], view, Some(snap));
        assert!(app.hex_model.is_some());
        assert_eq!(app.hex_view.hunks.len(), 1);
        assert!(app.hex_bytes.is_some());
        assert!(app.status.contains("1 hunk(s)"));
        // Har stale-wipe site snap bhi saaf karta hai.
        app.clear_hex_results();
        assert!(app.hex_model.is_none());
        assert!(app.hex_bytes.is_none());
        assert!(app.hex_view.hunks.is_empty());
    }

    #[test]
    fn hex_swap_clears_byte_snap() {
        let mut app = App::default();
        app.set_tab(Tab::Hex);
        app.hex_model = Some(vec![String::from("x")]);
        app.hex_bytes = Some(HexByteSnap {
            left: vec![1u8],
            right: vec![2u8],
        });
        assert!(app.swap_sides());
        assert!(app.hex_model.is_none());
        assert!(app.hex_bytes.is_none());
    }

    #[test]
    fn read_hex_input_capped_round_trips_small_file() {
        let path = std::env::temp_dir().join("nibble-bg-capped-probe.bin");
        let _ = std::fs::write(&path, [9u8, 8, 7]);
        let bytes = read_hex_input_capped(&path);
        assert_eq!(bytes, vec![9u8, 8, 7]);
        let _ = std::fs::remove_file(&path);
        // Missing path: khali, kabhi panic nahi.
        let missing = std::env::temp_dir().join("nibble-bg-definitely-missing.bin");
        assert!(read_hex_input_capped(&missing).is_empty());
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

    #[test]
    fn sync_copy_right_plan_keeps_only_checked_copies() {
        let mut list = SyncChecklist::from_plan(&sync_plan_fixture());
        // Fixture: new.txt CopyToRight + old/ DeleteLeft, both checked.
        let plan = list.copy_right_plan();
        assert_eq!(plan.steps.len(), 1);
        let Some(step) = plan.steps.first() else {
            return;
        };
        assert_eq!(step.rel_path, "new.txt");
        assert_eq!(step.action, autocompare_compare::SyncAction::CopyToRight);
        assert_eq!(plan.skipped.len(), 1);
        // Unchecking the copy leaves zero copy steps (delete never passes).
        assert!(list.set_checked(0, false));
        assert!(list.copy_right_plan().steps.is_empty());
        // Re-checking the delete row still yields no copy steps.
        list.set_all(true);
        assert!(list.set_checked(0, false));
        assert!(list.copy_right_plan().steps.is_empty());
    }

    #[test]
    fn sync_apply_gate_needs_confirm_and_steps() {
        assert!(!sync_apply_enabled(false, 0));
        assert!(!sync_apply_enabled(false, 2));
        assert!(!sync_apply_enabled(true, 0));
        assert!(sync_apply_enabled(true, 1));
        assert!(sync_apply_enabled(true, 3));
        assert!(
            sync_dry_run_summary(1, 2).contains("no writes"),
            "unexpected: {}",
            sync_dry_run_summary(1, 2)
        );
        // BI-S3 shared gate: Sync tab button reads sync_tab_can_apply,
        // Folder tab reads folder_can_apply — dono ek shape ke gate.
        let mut app = App::default();
        assert!(!app.sync_tab_can_apply());
        app.sync_model = Some(SyncChecklist::from_plan(&sync_plan_fixture()));
        assert!(!app.sync_tab_can_apply());
        app.set_sync_confirmed(true);
        assert!(app.sync_tab_can_apply());
        app.set_sync_confirmed(false);
        assert!(!app.sync_tab_can_apply());
        // Checked copy hate hi gate band, consent rehne par bhi.
        app.set_sync_confirmed(true);
        let Some(list) = app.sync_model.as_mut() else {
            return;
        };
        list.set_all(false);
        assert!(!app.sync_tab_can_apply());
        assert!(sync_dry_run_summary(0, 0).contains("0 of 0"));
    }

    #[test]
    fn sync_dry_run_default_reports_without_writes() {
        let mut app = App::default();
        assert!(app.sync_dry_run);
        assert!(!app.sync_confirmed);
        assert!(app.sync_last_report.is_none());
        app.sync_model = Some(SyncChecklist::from_plan(&sync_plan_fixture()));
        assert!(app.run_sync_checklist_apply());
        let Some(report) = app.sync_last_report.clone() else {
            return;
        };
        assert!(report.contains("no writes"), "unexpected: {report}");
        assert!(report.contains("1 of 2"), "unexpected: {report}");
        assert!(!app.sync_confirmed);
    }

    #[test]
    fn sync_apply_without_confirm_writes_nothing() {
        let mut app = App::default();
        app.set_sync_dry_run(false);
        app.sync_model = Some(SyncChecklist::from_plan(&sync_plan_fixture()));
        assert!(!app.run_sync_checklist_apply());
        assert!(app.status.contains("confirm"));
        // Model untouched so the user can confirm and retry.
        let Some(list) = app.sync_model.as_ref() else {
            return;
        };
        assert_eq!(list.checked_count(), 2);
    }

    static APPLY_TEST_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn apply_test_dirs(tag: &str) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        let n = APPLY_TEST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("ad-apply-{}-{n}-{tag}", std::process::id()));
        let left = base.join("left");
        let right = base.join("right");
        if std::fs::create_dir_all(&left).is_err() {
            return None;
        }
        if std::fs::create_dir_all(&right).is_err() {
            return None;
        }
        Some((left, right))
    }

    fn write_test_file(path: &std::path::Path, bytes: &[u8]) -> bool {
        std::fs::write(path, bytes).is_ok()
    }

    fn read_test_file(path: &std::path::Path) -> String {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(_) => String::from("<missing>"),
        }
    }

    fn remove_test_tree(left: &std::path::Path, right: &std::path::Path) {
        let base = left.parent().unwrap_or(right);
        let _ = std::fs::remove_dir_all(base);
    }

    /// Makes `path` deterministically newer than `older` by stamping
    /// mtime +60s. Same-size files compare by mtime (content compare
    /// off by default), so this turns a pair into `NewerLeft` without
    /// any sleep. Pure test helper; false on any io hiccup.
    fn touch_newer(path: &std::path::Path, older: &std::path::Path) -> bool {
        let base = match std::fs::metadata(older).and_then(|meta| meta.modified()) {
            Ok(time) => time,
            Err(_) => return false,
        };
        let newer = base
            .checked_add(std::time::Duration::from_secs(60))
            .unwrap_or(base);
        match std::fs::OpenOptions::new().write(true).open(path) {
            Ok(file) => file.set_modified(newer).is_ok(),
            Err(_) => false,
        }
    }

    #[test]
    fn sync_apply_copies_new_and_changed_left_to_right() {
        let Some((left, right)) = apply_test_dirs("copy") else {
            return;
        };
        // changed.txt is the same size both sides so the engine grades
        // by mtime; touch_newer makes the left copy win (NewerLeft).
        let setup_ok = write_test_file(&left.join("new.txt"), b"new")
            && write_test_file(&left.join("changed.txt"), b"left-new!!")
            && write_test_file(&right.join("changed.txt"), b"right-old!")
            && write_test_file(&right.join("right-only.txt"), b"keep");
        assert!(setup_ok);
        assert!(touch_newer(
            &left.join("changed.txt"),
            &right.join("changed.txt")
        ));
        // plan_sync reuse: engine plan, then the checklist one-direction filter.
        let Ok(planned) = App::plan_mirror_copy(&left, &right) else {
            remove_test_tree(&left, &right);
            return;
        };
        let list = SyncChecklist::from_plan(&planned);
        let one_way = list.copy_right_plan();
        assert_eq!(one_way.steps.len(), 2);
        // Confirmed real run through the same safe subset the UI uses.
        let Ok(report) = App::apply_mirror_copy(&left, &right, &one_way) else {
            remove_test_tree(&left, &right);
            return;
        };
        assert_eq!(report.copied, 2);
        assert_eq!(report.deleted, 0);
        assert!(report.failed.is_empty());
        assert_eq!(read_test_file(&right.join("new.txt")), "new");
        assert_eq!(read_test_file(&right.join("changed.txt")), "left-new!!");
        // Right-only entry survives: no prune, no deletes, left untouched.
        assert_eq!(read_test_file(&right.join("right-only.txt")), "keep");
        assert_eq!(read_test_file(&left.join("changed.txt")), "left-new!!");
        remove_test_tree(&left, &right);
    }

    #[test]
    fn sync_checklist_apply_end_to_end_confirmed() {
        let Some((left, right)) = apply_test_dirs("e2e") else {
            return;
        };
        let setup_ok = write_test_file(&left.join("a.txt"), b"a")
            && write_test_file(&right.join("b.txt"), b"b");
        assert!(setup_ok);
        let mut app = App::default();
        app.set_tab(Tab::Sync);
        // Dry-run first: report, no writes.
        let Ok(planned) = App::plan_mirror_copy(&left, &right) else {
            remove_test_tree(&left, &right);
            return;
        };
        app.sync_model = Some(SyncChecklist::from_plan(&planned));
        app.sync.left = left.to_string_lossy().into_owned();
        app.sync.right = right.to_string_lossy().into_owned();
        assert!(app.run_sync_checklist_apply());
        assert_eq!(read_test_file(&right.join("a.txt")), "<missing>");
        // Confirm + real run: left->right copy lands, right-only stays.
        app.set_sync_dry_run(false);
        app.set_sync_confirmed(true);
        assert!(app.run_sync_checklist_apply());
        assert!(!app.sync_confirmed);
        assert_eq!(read_test_file(&right.join("a.txt")), "a");
        assert_eq!(read_test_file(&right.join("b.txt")), "b");
        let Some(report) = app.sync_last_report.clone() else {
            remove_test_tree(&left, &right);
            return;
        };
        assert!(report.contains("1 copied"), "unexpected: {report}");
        remove_test_tree(&left, &right);
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

    static FOLDER_MERGE_WIRE_SEQ: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);

    fn folder_merge_wire_dirs(
        tag: &str,
    ) -> Option<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
        let n = FOLDER_MERGE_WIRE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("fm-wire-{}-{n}-{tag}", std::process::id()));
        let dir_base = root.join("base");
        let dir_left = root.join("left");
        let dir_right = root.join("right");
        if std::fs::create_dir_all(&dir_base).is_err() {
            return None;
        }
        if std::fs::create_dir_all(&dir_left).is_err() {
            return None;
        }
        if std::fs::create_dir_all(&dir_right).is_err() {
            return None;
        }
        Some((dir_base, dir_left, dir_right))
    }

    fn remove_folder_merge_wire_dirs(dir_base: &std::path::Path) {
        if let Some(root) = dir_base.parent() {
            let _ = std::fs::remove_dir_all(root);
        }
    }

    fn write_wire_file(dir: &std::path::Path, name: &str, sub: Option<&str>) -> bool {
        let target = match sub {
            Some(folder) => {
                let folder_path = dir.join(folder);
                if std::fs::create_dir_all(&folder_path).is_err() {
                    return false;
                }
                folder_path.join(name)
            }
            None => dir.join(name),
        };
        std::fs::write(target, b"wire").is_ok()
    }

    #[test]
    fn folder_merge_snapshot_lists_sorted_presence() {
        let Some((dir_base, dir_left, dir_right)) = folder_merge_wire_dirs("snap") else {
            return;
        };
        let setup = write_wire_file(&dir_base, "keep.txt", None)
            && write_wire_file(&dir_base, "nested.txt", Some("sub"))
            && write_wire_file(&dir_left, "keep.txt", None)
            && write_wire_file(&dir_right, "keep.txt", None);
        if !setup {
            remove_folder_merge_wire_dirs(&dir_base);
            return;
        }
        let base = snapshot_folder_presence(&dir_base);
        let names: Vec<&str> = base.iter().map(|row| row.rel_path.as_str()).collect();
        assert_eq!(names, vec!["keep.txt", "sub", "sub/nested.txt"]);
        let Some(sub) = base.iter().find(|row| row.rel_path == "sub") else {
            remove_folder_merge_wire_dirs(&dir_base);
            return;
        };
        assert!(sub.is_dir);
        let Some(keep) = base.iter().find(|row| row.rel_path == "keep.txt") else {
            remove_folder_merge_wire_dirs(&dir_base);
            return;
        };
        assert!(!keep.is_dir);
        assert!(snapshot_folder_presence(&dir_left.join("nope")).is_empty());
        let _ = (dir_right,);
        remove_folder_merge_wire_dirs(&dir_base);
    }

    #[test]
    fn folder_merge_wiring_builds_tree_rows_and_stages_decisions() {
        let Some((dir_base, dir_left, dir_right)) = folder_merge_wire_dirs("tree") else {
            return;
        };
        // del.txt: base + left me hai, right ne hataya (delete-vs-keep
        // conflict). new.txt: base me nahi, dono ne joda (added-both
        // conflict). keep.txt: teeno me (clean).
        let setup = write_wire_file(&dir_base, "keep.txt", None)
            && write_wire_file(&dir_base, "del.txt", None)
            && write_wire_file(&dir_left, "keep.txt", None)
            && write_wire_file(&dir_left, "del.txt", None)
            && write_wire_file(&dir_left, "new.txt", None)
            && write_wire_file(&dir_right, "keep.txt", None)
            && write_wire_file(&dir_right, "new.txt", None);
        if !setup {
            remove_folder_merge_wire_dirs(&dir_base);
            return;
        }
        let mut app = App::default();
        app.set_tab(Tab::FolderMerge);
        app.folder_merge_model = Some(FolderMergeChecklist::from_plan(&merge_plan_fixture()));
        app.folder_merge_base = snapshot_folder_presence(&dir_base);
        app.folder_merge_left = snapshot_folder_presence(&dir_left);
        app.folder_merge_right = snapshot_folder_presence(&dir_right);
        let rows = tree::three_way_tree_rows(
            &app.folder_merge_base,
            &app.folder_merge_left,
            &app.folder_merge_right,
        );
        assert!(!rows.is_empty());
        let conflicts = tree::three_way_conflicts(&rows);
        assert!(conflicts.contains(&"del.txt".to_string()));
        assert!(conflicts.contains(&"new.txt".to_string()));
        assert!(!conflicts.contains(&"keep.txt".to_string()));
        assert!(app.folder_merge_decisions.is_empty());
        app.folder_merge_decisions = views::stage_three_way_decision(
            &app.folder_merge_decisions,
            "del.txt",
            tree::ThreeWaySide::Right,
        );
        assert_eq!(
            views::three_way_decision_for_row(&app.folder_merge_decisions, "del.txt"),
            Some(tree::ThreeWaySide::Right)
        );
        let summary = tree::three_way_tree_summary(&rows, &app.folder_merge_decisions);
        assert_eq!(summary.decided, 1);
        assert!(summary.conflicted);
        // Swap sides ko stale manta hai: model + snapshots + decisions
        // sab clear hote hain.
        assert!(app.swap_sides());
        assert!(app.folder_merge_model.is_none());
        assert!(app.folder_merge_base.is_empty());
        assert!(app.folder_merge_left.is_empty());
        assert!(app.folder_merge_right.is_empty());
        assert!(app.folder_merge_decisions.is_empty());
        remove_folder_merge_wire_dirs(&dir_base);
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

    #[test]
    fn audio_checklist_carries_windows_for_waveform() {
        let list = AudioChecklist::from_diff(&audio_diff_fixture(false, false, Some(0.5)));
        assert_eq!(list.windows_total, 100);
        assert_eq!(list.windows_differing, 1);
        assert_eq!(list.first_diff_at, Some(0.5));
        let fresh = AudioChecklist::default();
        assert_eq!(fresh.windows_total, 0);
        assert_eq!(fresh.windows_differing, 0);
        assert_eq!(fresh.first_diff_at, None);
    }

    #[test]
    fn audio_play_stub_label_names_stub() {
        let label = audio_play_stub_label();
        assert!(label.contains("stub"));
        assert!(label.contains("no playback"));
    }

    #[test]
    fn audio_play_stub_request_sets_status_only() {
        let mut app = App::default();
        app.set_tab(Tab::Audio);
        app.audio_model = Some(AudioChecklist::from_diff(&audio_diff_fixture(
            true, false, None,
        )));
        let rows_before = app.audio_model.as_ref().map_or(0, |list| list.rows.len());
        app.request_audio_play_stub();
        assert!(app.status.contains("stub"));
        assert!(app.status.contains("no playback"));
        assert_eq!(
            app.audio_model.as_ref().map_or(0, |list| list.rows.len()),
            rows_before
        );
        assert!(app.audio_model.is_some());
    }

    #[test]
    fn audio_waveform_bar_covers_empty_and_shares() {
        assert_eq!(audio_waveform_bar(0, 0).len(), AUDIO_WAVEFORM_WIDTH);
        assert!(audio_waveform_bar(0, 0).chars().all(|ch| ch == '-'));
        let same = audio_waveform_bar(0, 100);
        assert_eq!(same.len(), AUDIO_WAVEFORM_WIDTH);
        assert!(same.chars().all(|ch| ch == '-'));
        let all = audio_waveform_bar(100, 100);
        assert!(all.chars().all(|ch| ch == '#'));
        let partial = audio_waveform_bar(1, 100);
        assert_eq!(partial.len(), AUDIO_WAVEFORM_WIDTH);
        assert!(partial.contains('#'));
        assert!(partial.contains('-'));
        // Wild model clamps instead of overflowing.
        let wild = audio_waveform_bar(9_999, 100);
        assert_eq!(wild.len(), AUDIO_WAVEFORM_WIDTH);
        assert!(wild.chars().all(|ch| ch == '#'));
    }

    #[test]
    fn audio_first_diff_text_names_stamp_or_dash() {
        assert!(audio_first_diff_text(Some(0.5)).contains("0.500s"));
        assert_eq!(audio_first_diff_text(None), "first difference: -");
        assert_eq!(audio_first_diff_text(Some(f64::NAN)), "first difference: -");
    }

    #[test]
    fn audio_action_summary_follows_app_state() {
        let mut app = App::default();
        app.set_tab(Tab::Audio);
        assert_eq!(app.audio_action_summary(), "no audio results yet");
        app.audio_model = Some(AudioChecklist::from_diff(&audio_diff_fixture(
            true, false, None,
        )));
        assert_eq!(app.audio_action_summary(), "no differences");
        app.audio_model = Some(AudioChecklist::from_diff(&audio_diff_fixture(
            false,
            false,
            Some(0.5),
        )));
        assert!(app.audio_action_summary().contains("0.500s"));
    }

    #[test]
    fn hex_pager_syncs_and_pages_with_selection() {
        let mut app = App {
            hex_view: views_hex::HexCompareView::build_from_slices(
                &[0u8; 64],
                &[1u8; 64],
                views_hex::HexCompareMode::None,
                8,
            ),
            ..Default::default()
        };
        app.sync_hex_pager();
        assert_eq!(app.hex_pager.total_rows, app.hex_view.hunks.len() as u64);
        let top_before = app.hex_pager.top;
        app.hex_page_next();
        assert!(app.hex_pager.top >= top_before);
        app.hex_page_prev();
        // Gutter jump selects covering hunk and brings into view.
        let row = 1u64;
        if views_hex::hex_row_to_hunk(row, &app.hex_view.hunks).is_some() {
            assert!(app.hex_gutter_jump_to_row(row));
            let idx = app.hex_view.selected.unwrap_or(0) as u64;
            let (start, end) = app.hex_pager.visible_range();
            assert!(idx >= start && idx < end.max(start + 1));
        }
        app.hex_select_next_paged();
        assert!(app.hex_view.selected.is_some());
        app.hex_select_prev_paged();
        assert!(app.hex_view.selected.is_some());
    }

    #[test]
    fn table_pager_syncs_and_find_brings_into_view() {
        use crate::viewmodel::{TableModel, TableRow};
        let mut app = App {
            table_model: Some(TableModel {
                rows: vec![
                    TableRow {
                        glyph: '=',
                        label: "   1".to_string(),
                        text: "apple".to_string(),
                        changed: false,
                    },
                    TableRow {
                        glyph: '!',
                        label: "   2".to_string(),
                        text: "banana".to_string(),
                        changed: true,
                    },
                ],
                truncated: false,
            }),
            table_find_needle: String::from("banana"),
            ..Default::default()
        };
        app.sync_table_pager();
        assert_eq!(app.table_pager.total_rows, 2);
        let found = app.table_find_next_paged();
        assert_eq!(found, Some((1, 0)));
        let (start, end) = app.table_pager.visible_range();
        assert!(1 >= start && 1 < end.max(start + 1));
        app.table_page_next();
        app.table_page_prev();
        assert_eq!(app.table_pager.top, 0);
    }

    #[test]
    fn table_find_bar_tracks_matches_and_badge() {
        use crate::viewmodel::{TableModel, TableRow};
        let rows = vec![
            TableRow {
                glyph: '=',
                label: "   1".to_string(),
                text: "apple".to_string(),
                changed: false,
            },
            TableRow {
                glyph: '!',
                label: "   2".to_string(),
                text: "banana".to_string(),
                changed: true,
            },
            TableRow {
                glyph: '!',
                label: "   3".to_string(),
                text: "Banana bread".to_string(),
                changed: true,
            },
        ];
        let mut app = App {
            table_model: Some(TableModel {
                rows,
                truncated: false,
            }),
            ..Default::default()
        };
        app.table_find_set_needle("banana");
        // Case-sensitive: sirf chhota "banana" milta hai.
        assert_eq!(app.table_find_total(), 1);
        assert_eq!(app.table_find_badge(), "1/1");
        assert_eq!(app.table_find_next_row(), Some((1, 0)));
        assert_eq!(app.table_find_badge(), "1/1");
        assert_eq!(app.table_find_prev_row(), Some((1, 0)));
        assert!(app.status.contains("(1/1)"));
        // Ignore-case par dono banana rows milti hain, Next/Prev wrap karte hain.
        app.table_view.ignore_case = true;
        assert_eq!(app.table_find_total(), 2);
        assert_eq!(app.table_find_badge(), "1/2");
        assert_eq!(app.table_find_next_row(), Some((1, 0)));
        assert_eq!(app.table_find_badge(), "2/2");
        assert_eq!(app.table_find_next_row(), Some((2, 0)));
        assert_eq!(app.table_find_badge(), "1/2");
        assert_eq!(app.table_find_prev_row(), Some((2, 0)));
        assert_eq!(app.table_find_badge(), "2/2");
        // Khali needle: 0/0, koi navigation nahi.
        app.table_find_set_needle("");
        assert_eq!(app.table_find_total(), 0);
        assert_eq!(app.table_find_badge(), "0/0");
        assert_eq!(app.table_find_next_row(), None);
        assert_eq!(app.table_find_prev_row(), None);
        assert_eq!(app.status, String::from("table find: no match"));
    }

    #[test]
    fn folder_pager_syncs_and_find_brings_into_view() {
        use crate::viewmodel::FolderRow;
        let rows = vec![
            FolderRow {
                rel_path: "same.txt".to_string(),
                is_dir: false,
                glyph: '=',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: false,
            },
            FolderRow {
                rel_path: "diff.txt".to_string(),
                is_dir: false,
                glyph: '!',
                left_size: None,
                right_size: None,
                left_time: None,
                right_time: None,
                changed: true,
            },
        ];
        let mut app = App {
            folder_model: Some((rows, String::from("summary"))),
            ..Default::default()
        };
        app.sync_folder_pager();
        assert_eq!(app.folder_pager.total_rows, 2);
        assert_eq!(app.folder_next_changed_paged(), Some(1));
        app.folder_find_needle = String::from("diff");
        app.folder_cursor = 0;
        assert_eq!(app.folder_find_next_paged(), Some(1));
        app.folder_page_next();
        app.folder_page_prev();
        assert_eq!(app.folder_pager.top, 0);
    }

    #[test]
    fn text_find_box_tracks_matches_and_badge() {
        use crate::viewmodel::{TextFilter, TextRow, TextRowKind, TextRows};
        let rows = vec![
            TextRow {
                kind: TextRowKind::Equal,
                left: String::from("alpha line"),
                right: String::from("alpha line"),
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Changed,
                left: String::from("banana split"),
                right: String::from("Banana bread"),
                ..Default::default()
            },
            TextRow {
                kind: TextRowKind::Equal,
                left: String::from("plain"),
                right: String::from("plain"),
                ..Default::default()
            },
        ];
        // Pure matcher: case flag decides hit count.
        assert_eq!(App::text_find_match_rows(&rows, "banana", true), vec![1]);
        assert_eq!(App::text_find_match_rows(&rows, "BANANA", true), vec![1]);
        assert_eq!(App::text_find_match_rows(&rows, "BANANA", false).len(), 0);
        assert!(App::text_find_match_rows(&rows, "   ", true).is_empty());
        assert!(App::text_find_contains("Hello", "ell", true));
        assert!(!App::text_find_contains("Hello", "", true));
        // Badge + navigation through App state.
        let mut app = App {
            text_model: Some(TextRows {
                rows,
                truncated: false,
                coarse: false,
                pane_chars: 12,
                gutter_chars: 4,
                selected: None,
                filter: TextFilter::All,
            }),
            ..Default::default()
        };
        app.text_find_set_needle("banana");
        assert_eq!(app.text_find_total(), 1);
        assert_eq!(app.text_find_badge(), "1/1");
        assert_eq!(app.text_find_next_row(), Some(1));
        assert_eq!(app.text_find_badge(), "1/1");
        assert_eq!(app.text_find_prev_row(), Some(1));
        // Empty needle reports 0/0 and no navigation.
        app.text_find_set_needle("");
        assert_eq!(app.text_find_total(), 0);
        assert_eq!(app.text_find_badge(), "0/0");
        assert_eq!(app.text_find_position(), None);
        assert_eq!(app.text_find_next_row(), None);
    }

    #[test]
    fn text_find_next_prev_wrap_over_many_matches() {
        use crate::viewmodel::{TextFilter, TextRow, TextRowKind, TextRows};
        let mut rows = Vec::new();
        for i in 0..4 {
            rows.push(TextRow {
                kind: TextRowKind::Changed,
                left: format!("needle row {i}"),
                right: String::new(),
                ..Default::default()
            });
        }
        let mut app = App {
            text_model: Some(TextRows {
                rows,
                truncated: false,
                coarse: false,
                pane_chars: 11,
                gutter_chars: 4,
                selected: None,
                filter: TextFilter::All,
            }),
            ..Default::default()
        };
        app.text_find_set_needle("needle");
        assert_eq!(app.text_find_total(), 4);
        assert_eq!(app.text_find_badge(), "1/4");
        assert_eq!(app.text_find_next_row(), Some(0));
        assert_eq!(app.text_find_badge(), "2/4");
        assert_eq!(app.text_find_next_row(), Some(1));
        assert_eq!(app.text_find_prev_row(), Some(1));
        assert_eq!(app.text_find_badge(), "2/4");
        // Wrap: cursor end se shuru par.
        app.text_find_cursor = 3;
        assert_eq!(app.text_find_next_row(), Some(3));
        assert_eq!(app.text_find_badge(), "1/4");
        // Case-sensitive needle misses upper-case text.
        app.text_find_ignore_case = false;
        app.text_find_set_needle("NEEDLE");
        assert_eq!(app.text_find_total(), 0);
        assert_eq!(app.text_find_badge(), "0/0");
    }

    #[test]
    fn text_find_state_prefs_follow_toggles() {
        // Paint prefs App toggles se bante hain (paint_text_with_find ko
        // milte hain): default full-width + numbers + accents.
        let app = App::default();
        let prefs = app.text_find_prefs();
        assert_eq!(
            prefs,
            crate::viewmodel::TextViewPrefs {
                word_wrap: false,
                show_line_numbers: true,
                syntax_highlight: true,
            }
        );
        let mut wrapped = App::default();
        wrapped.set_word_wrap(true);
        wrapped.set_line_numbers(false);
        wrapped.set_syntax_highlight(false);
        let prefs = wrapped.text_find_prefs();
        assert!(prefs.word_wrap);
        assert!(!prefs.show_line_numbers);
        assert!(!prefs.syntax_highlight);
    }

    #[test]
    fn text_find_state_seeds_from_legacy_needle() {
        // Legacy needle/flag (Wave Q compat) FindState me seed hota hai taaki
        // purane setters se paint tak needle pahunche; cursor first hit par.
        use crate::viewmodel::{TextFilter, TextRow, TextRowKind, TextRows};
        let mut app = App {
            text_model: Some(TextRows {
                rows: vec![
                    TextRow {
                        kind: TextRowKind::Changed,
                        left: String::from("apple"),
                        right: String::new(),
                        ..Default::default()
                    },
                    TextRow {
                        kind: TextRowKind::Changed,
                        left: String::from("banana"),
                        right: String::new(),
                        ..Default::default()
                    },
                ],
                truncated: false,
                coarse: false,
                pane_chars: 12,
                gutter_chars: 4,
                selected: None,
                filter: TextFilter::All,
            }),
            ..Default::default()
        };
        app.text_find_set_needle("banana");
        assert!(app.sync_text_find_state());
        assert_eq!(app.text_find.query(), "banana");
        assert_eq!(app.text_find.match_rows(), &[1]);
        assert_eq!(app.text_find_state_badge(), "1/1");
        // Dobara sync bina change ke refresh nahi karta.
        assert!(!app.sync_text_find_state());
        // Case flag badalne par recompute hota hai.
        app.text_find_ignore_case = false;
        app.text_find_set_needle("BANANA");
        assert!(app.sync_text_find_state());
        assert_eq!(app.text_find_state_badge(), "0/0");
    }

    #[test]
    fn text_find_state_badge_matches_legacy_badge() {
        // N/M badge dono paths par same dikhta hai (FindState paint me,
        // legacy compat helpers me).
        use crate::viewmodel::{TextFilter, TextRow, TextRowKind, TextRows};
        let mut app = App {
            text_model: Some(TextRows {
                rows: vec![
                    TextRow {
                        kind: TextRowKind::Changed,
                        left: String::from("needle one"),
                        right: String::new(),
                        ..Default::default()
                    },
                    TextRow {
                        kind: TextRowKind::Changed,
                        left: String::from("plain"),
                        right: String::new(),
                        ..Default::default()
                    },
                    TextRow {
                        kind: TextRowKind::Changed,
                        left: String::from("needle two"),
                        right: String::new(),
                        ..Default::default()
                    },
                ],
                truncated: false,
                coarse: false,
                pane_chars: 12,
                gutter_chars: 4,
                selected: None,
                filter: TextFilter::All,
            }),
            ..Default::default()
        };
        app.text_find_set_needle("needle");
        assert!(app.sync_text_find_state());
        assert_eq!(app.text_find_state_badge(), app.text_find_badge());
        assert_eq!(app.text_find_state_badge(), "1/2");
        // Paint ke baad legacy needle mirror hota hai (stale nahi rehta).
        app.mirror_legacy_from_text_find();
        assert_eq!(app.text_find_needle, "needle");
        assert!(app.text_find_ignore_case);
    }

    #[test]
    fn hex_find_needle_parses_and_reuses_model_search() {
        // Hex text with spaces decodes to bytes.
        assert_eq!(App::parse_hex_needle("0a ff"), vec![0x0au8, 0xffu8]);
        assert_eq!(App::parse_hex_needle("0AFF"), vec![0x0au8, 0xffu8]);
        assert!(App::parse_hex_needle("   ").is_empty());
        // Plain ascii falls back to raw bytes.
        assert_eq!(App::parse_hex_needle("banana"), b"banana".to_vec());
        let hay = vec![10u8, 20, 30, 20, 30, 40];
        // Byte needle via hex text wraps forward and backward.
        assert_eq!(App::hex_find_next_from(&hay, "14 1e", 0), Some(1));
        assert_eq!(App::hex_find_next_from(&hay, "14 1e", 2), Some(3));
        assert_eq!(App::hex_find_next_from(&hay, "14 1e", 4), Some(1));
        assert_eq!(App::hex_find_prev_from(&hay, "14 1e", 4), Some(3));
        assert_eq!(App::hex_find_prev_from(&hay, "14 1e", 0), Some(3));
        // Ascii needle reuses the same byte search.
        assert_eq!(App::hex_find_next_from(b"xxbananaYY", "banana", 0), Some(2));
        assert_eq!(App::hex_find_count(b"xxbananaYYbanana", "banana"), 2);
        // Empty or missing needle never hits.
        assert_eq!(App::hex_find_next_from(&hay, "", 0), None);
        assert_eq!(App::hex_find_next_from(&hay, "ff", 0), None);
        assert_eq!(App::hex_find_count(&hay, ""), 0);
        // App cursor wrappers advance and retreat.
        let mut app = App {
            hex_find_needle: String::from("14 1e"),
            ..Default::default()
        };
        assert_eq!(app.hex_find_advance(&hay), Some(1));
        assert_eq!(app.hex_find_retreat(&hay), Some(1));
        app.hex_find_needle = String::from("");
        assert_eq!(app.hex_find_advance(&hay), None);
    }

    #[test]
    fn font_zoom_steps_clamp_and_reset() {
        // Wave R model live hai (app.rs ui me Cmd +/-/0): helper yahi
        // steps chalate hain, paint me koi change nahi.
        let mut app = App::default();
        assert_eq!(app.zoom_level(), crate::theme::ZOOM_DEFAULT_LEVEL);
        app.zoom_in();
        assert_eq!(app.zoom_level(), crate::theme::ZOOM_DEFAULT_LEVEL + 1);
        assert!(app.status.contains("font zoom"));
        app.zoom_out();
        assert_eq!(app.zoom_level(), crate::theme::ZOOM_DEFAULT_LEVEL);
        app.set_zoom_level(100);
        assert_eq!(app.zoom_level(), crate::theme::ZOOM_MAX_LEVEL);
        app.set_zoom_level(-100);
        assert_eq!(app.zoom_level(), crate::theme::ZOOM_MIN_LEVEL);
        app.zoom_reset();
        assert_eq!(app.zoom_level(), crate::theme::ZOOM_DEFAULT_LEVEL);
    }

    #[test]
    fn font_zoom_keys_resolve_and_apply() {
        // Cmd/Ctrl + +/-/0 binding ka headless core: resolve priority,
        // phir apply se level badalta hai.
        assert_eq!(App::resolve_zoom_key(true, false, false), ZoomKey::In);
        assert_eq!(App::resolve_zoom_key(false, true, false), ZoomKey::Out);
        assert_eq!(App::resolve_zoom_key(false, false, true), ZoomKey::Reset);
        assert_eq!(App::resolve_zoom_key(false, false, false), ZoomKey::None);
        // Reset wins jab 0 +/- ke saath chorded ho; In Out se pehle.
        assert_eq!(App::resolve_zoom_key(true, true, true), ZoomKey::Reset);
        assert_eq!(App::resolve_zoom_key(true, false, true), ZoomKey::Reset);
        assert_eq!(App::resolve_zoom_key(true, true, false), ZoomKey::In);

        let mut app = App::default();
        assert!(app.apply_zoom_key(ZoomKey::In));
        assert_eq!(app.zoom_level(), crate::theme::ZOOM_DEFAULT_LEVEL + 1);
        assert!(app.status.contains("font zoom"));
        assert!(app.apply_zoom_key(ZoomKey::Out));
        assert_eq!(app.zoom_level(), crate::theme::ZOOM_DEFAULT_LEVEL);
        // Reset door se wapas laata hai.
        app.set_zoom_level(crate::theme::ZOOM_MAX_LEVEL);
        assert!(app.apply_zoom_key(ZoomKey::Reset));
        assert_eq!(app.zoom_level(), crate::theme::ZOOM_DEFAULT_LEVEL);
        // None no-op: level + status dono untouched.
        let level = app.zoom_level();
        assert!(!app.apply_zoom_key(ZoomKey::None));
        assert_eq!(app.zoom_level(), level);
    }

    #[test]
    fn text_find_badge_label_matches_app_badge() {
        // Paint badge views helper se aata hai; App model se parity.
        use crate::viewmodel::{TextFilter, TextRow, TextRowKind, TextRows};
        let mut app = App {
            text_model: Some(TextRows {
                rows: vec![TextRow {
                    kind: TextRowKind::Changed,
                    left: String::from("needle here"),
                    right: String::new(),
                    ..Default::default()
                }],
                truncated: false,
                coarse: false,
                pane_chars: 11,
                gutter_chars: 4,
                selected: None,
                filter: TextFilter::All,
            }),
            ..Default::default()
        };
        app.text_find_set_needle("needle");
        assert_eq!(
            crate::views::text_find_badge_label(app.text_find_position(), app.text_find_total()),
            app.text_find_badge(),
        );
        assert_eq!(
            crate::views::text_find_badge_label(app.text_find_position(), app.text_find_total()),
            "1/1",
        );
        app.text_find_set_needle("");
        assert_eq!(
            crate::views::text_find_badge_label(app.text_find_position(), app.text_find_total()),
            "0/0",
        );
    }

    #[test]
    fn layout_narrow_breakpoint_is_900() {
        assert_eq!(LAYOUT_NARROW_BREAKPOINT, 900.0);
        assert!(layout_is_narrow(899.0));
        assert!(!layout_is_narrow(900.0));
        assert!(!layout_is_narrow(1200.0));
    }

    #[test]
    fn layout_pane_widths_follow_split_with_floor() {
        let (left, right) = layout_pane_widths(800.0, 0.50);
        assert_eq!((left, right), (396.0, 396.0));
        let (wide_left, wide_right) = layout_pane_widths(800.0, 0.80);
        assert!(wide_left > wide_right);
        // Narrow windows keep the floors instead of crushing.
        let (tiny_left, tiny_right) = layout_pane_widths(0.0, 0.50);
        assert_eq!((tiny_left, tiny_right), (180.0, 180.0));
        // Same helper the picture columns use: single source.
        assert_eq!(
            layout_pane_widths(800.0, 0.50),
            crate::views::split_picture_widths(800.0, 0.50)
        );
    }

    #[test]
    fn layout_row_min_width_needs_hscroll_when_narrow() {
        assert_eq!(layout_row_min_width(800.0, 0.50), 800.0);
        assert!(!layout_needs_hscroll(
            800.0,
            layout_row_min_width(800.0, 0.50)
        ));
        // Crushed avail keeps floors: min row exceeds avail -> scroll.
        let min = layout_row_min_width(100.0, 0.50);
        assert_eq!(min, 368.0);
        assert!(layout_needs_hscroll(100.0, min));
        assert!(!layout_needs_hscroll(min, min));
    }

    #[test]
    fn find_bar_geometry_docks_full_width_with_right_badge() {
        assert_eq!(LAYOUT_FIND_HIT_MIN, 24.0);
        let geo = find_bar_geometry(800.0);
        assert_eq!(geo.bar_w, 800.0);
        assert_eq!(geo.badge_x, geo.bar_w);
        assert!(geo.prev_hit >= 24.0);
        assert!(geo.next_hit >= 24.0);
        let zero = find_bar_geometry(-5.0);
        assert_eq!((zero.bar_w, zero.badge_x), (0.0, 0.0));
    }

    #[test]
    fn toolbar_gaps_reuse_geo_tokens_with_group_hierarchy() {
        assert_eq!(toolbar_item_gap(), crate::views::TIGHT_GAP);
        assert_eq!(toolbar_group_gap(), crate::views::SECTION_GAP);
        assert!(toolbar_item_gap().is_finite() && toolbar_item_gap() > 0.0);
        assert!(toolbar_group_gap().is_finite() && toolbar_group_gap() > 0.0);
        assert!(toolbar_group_gap() > toolbar_item_gap());
    }

    #[test]
    fn clamp_scroll_offset_keeps_offsets_honest() {
        assert_eq!(clamp_scroll_offset(10.0, 100.0, 40.0), 10.0);
        assert_eq!(clamp_scroll_offset(90.0, 100.0, 40.0), 60.0);
        assert_eq!(clamp_scroll_offset(-4.0, 100.0, 40.0), 0.0);
        // Content fits the viewport: offset collapses to zero.
        assert_eq!(clamp_scroll_offset(10.0, 30.0, 40.0), 0.0);
        // Non-finite input never escapes as NaN.
        assert_eq!(clamp_scroll_offset(f32::NAN, 100.0, 40.0), 0.0);
        assert_eq!(clamp_scroll_offset(10.0, f32::INFINITY, 40.0), 0.0);
    }

    #[test]
    fn scroll_clamp_after_layout_keeps_pager_tops_valid() {
        let mut app = App {
            table_model: Some(crate::viewmodel::TableModel {
                rows: vec![crate::viewmodel::TableRow {
                    glyph: '=',
                    label: String::from("   1"),
                    text: String::from("apple"),
                    changed: false,
                }],
                truncated: false,
            }),
            ..Default::default()
        };
        app.table_pager.top = 99;
        app.set_split_fraction(0.60);
        assert_eq!(app.split_fraction(), 0.60);
        assert_eq!(app.table_pager.top, 0);
        app.table_pager.top = 99;
        app.drag_split_by(40.0, 800.0);
        assert_eq!(app.table_pager.top, 0);
    }

    #[test]
    fn prog_fraction_needs_known_total() {
        use autocompare_compare::JobProgress;
        assert_eq!(
            App::progress_fraction_of(&JobProgress::new(1, Some(4), "x")),
            Some(0.25)
        );
        assert_eq!(
            App::progress_fraction_of(&JobProgress::new(9, Some(4), "x")),
            Some(1.0)
        );
        assert_eq!(
            App::progress_fraction_of(&JobProgress::new(1, None, "x")),
            None
        );
        assert_eq!(
            App::progress_fraction_of(&JobProgress::new(0, Some(0), "x")),
            None
        );
    }

    #[test]
    fn prog_throttle_gate_suppresses_byte_spam() {
        // Same message + same fraction: no repaint.
        assert!(!App::progress_should_emit(
            Some(0.10),
            Some(0.10),
            Some("loading left file"),
            "loading left file",
        ));
        // Same message + tiny drift: still no repaint.
        assert!(!App::progress_should_emit(
            Some(0.10),
            Some(0.12),
            Some("loading left file"),
            "loading left file",
        ));
        // Jump >= 0.05 repaints; message change always repaints.
        assert!(App::progress_should_emit(
            Some(0.10),
            Some(0.20),
            Some("loading left file"),
            "loading left file",
        ));
        assert!(App::progress_should_emit(
            Some(0.50),
            Some(0.51),
            Some("loading left file"),
            "loading right file",
        ));
        // First fraction and completion always repaint.
        assert!(App::progress_should_emit(
            None,
            Some(0.0),
            Some(""),
            "starting"
        ));
        assert!(App::progress_should_emit(
            Some(0.97),
            Some(1.0),
            Some("finishing"),
            "finishing",
        ));
        // Indeterminate with no change stays quiet.
        assert!(!App::progress_should_emit(
            None,
            None,
            Some("planning"),
            "planning"
        ));
    }

    #[test]
    fn prog_status_text_reuses_status_field() {
        let line =
            App::progress_status_text("comparing: a <--> b - Text Compare", Some(0.5), "half");
        assert!(line.contains("comparing: a <--> b - Text Compare"));
        assert!(line.contains("50%"));
        assert!(line.contains("half"));
        let open = App::progress_status_text("comparing: x", None, "planning sync");
        assert!(open.contains("planning sync"));
        assert!(!open.contains('%'));
    }

    #[test]
    fn prog_cancel_path_without_job_reports_idle() {
        let mut app = App::default();
        assert!(!app.is_job_running());
        assert!(!app.cancel_active_job());
        assert!(!app.poll_job_progress());
        assert_eq!(app.progress_fraction, None);
    }

    #[test]
    fn prog_cancel_signals_running_job_and_status_notes_it() {
        use autocompare_compare::JobRunner;
        let mut app = App::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let handle: JobHandle<()> = JobRunner::spawn(move |cancel, progress| {
            progress(JobProgress::new(0, Some(2), "probe half"));
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = tx.send(CompareResult::Text(Err(String::from(
                    "operation cancelled",
                ))));
                return Ok(());
            }
            progress(JobProgress::new(2, Some(2), "probe done"));
            let _ = tx.send(CompareResult::Text(Err(String::from("probe done"))));
            Ok(())
        });
        app.note_job_started(rx, handle);
        assert!(app.is_job_running());
        assert!(app.cancel_active_job());
        assert!(app.status.contains("cancel requested"));
        assert_eq!(app.progress_fraction, Some(0.0));
        // Poll folds the worker's latest progress into the reused
        // status field; either the cancel note or a throttled
        // comparing line is honest while the worker drains.
        let _ = app.poll_job_progress();
        assert!(
            app.status.contains("cancel requested") || app.status.contains("comparing:"),
            "unexpected status: {}",
            app.status
        );
    }

    /// Unique temp root behind mirror-copy tests (no new deps):
    /// `std::env::temp_dir` + pid + atomic counter, best-effort cleanup.
    fn mirror_test_root() -> Option<std::path::PathBuf> {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "autocompare-apply-ad-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        if std::fs::create_dir_all(&dir).is_err() {
            return None;
        }
        Some(dir)
    }

    fn mirror_write_file(path: &std::path::Path, contents: &str) -> bool {
        if let Some(parent) = path.parent()
            && std::fs::create_dir_all(parent).is_err()
        {
            return false;
        }
        std::fs::write(path, contents).is_ok()
    }

    fn mirror_read_file(path: &std::path::Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    #[test]
    fn mirror_copy_options_are_left_to_right_without_prune() {
        let opts = App::mirror_copy_options();
        assert_eq!(opts.direction, SyncDirection::LeftToRight);
        assert!(!opts.prune);
    }

    #[test]
    fn mirror_copy_subset_keeps_only_copy_to_right() {
        let plan = autocompare_compare::SyncPlan {
            steps: vec![
                autocompare_compare::SyncStep {
                    rel_path: String::from("new.txt"),
                    is_dir: false,
                    action: SyncAction::CopyToRight,
                },
                autocompare_compare::SyncStep {
                    rel_path: String::from("back.txt"),
                    is_dir: false,
                    action: SyncAction::CopyToLeft,
                },
                autocompare_compare::SyncStep {
                    rel_path: String::from("gone"),
                    is_dir: true,
                    action: SyncAction::DeleteRight,
                },
            ],
            skipped: vec![(String::from("same.txt"), String::from("already in sync"))],
            truncated: false,
        };
        let safe = App::mirror_copy_subset(&plan);
        assert_eq!(safe.steps.len(), 1);
        let Some(first) = safe.steps.first() else {
            return;
        };
        assert_eq!(first.action, SyncAction::CopyToRight);
        assert_eq!(first.rel_path, "new.txt");
        assert_eq!(safe.skipped.len(), 1);
        assert_eq!(
            App::describe_mirror_report(&SyncReport::default()),
            "mirror copy: 0 copied, 0 deleted, 0 failed"
        );
    }

    fn mirror_set_old_mtime(path: &std::path::Path) -> bool {
        use std::time::{Duration, UNIX_EPOCH};
        let old = UNIX_EPOCH
            .checked_add(Duration::from_secs(1_700_000_000))
            .unwrap_or(UNIX_EPOCH);
        let file = match std::fs::File::options().write(true).open(path) {
            Ok(file) => file,
            Err(_) => return false,
        };
        file.set_times(std::fs::FileTimes::new().set_modified(old))
            .is_ok()
    }

    #[test]
    fn mirror_copy_applies_left_to_right_in_temp_dirs() {
        let Some(root) = mirror_test_root() else {
            return;
        };
        let left = root.join("left");
        let right = root.join("right");
        if std::fs::create_dir_all(&left).is_err() || std::fs::create_dir_all(&right).is_err() {
            let _ = std::fs::remove_dir_all(&root);
            return;
        }
        // Right stale copy pehle (phir old mtime), left updated copy baad
        // me taaki engine ise NewerLeft maane: same size + left newer.
        // Bina mtime fix ke dono writes ek hi tick me gir sakte hain.
        let wrote = mirror_write_file(&right.join("upd.txt"), "v1\n")
            && mirror_write_file(&right.join("keep.txt"), "mine\n")
            && mirror_write_file(&left.join("new.txt"), "fresh\n")
            && mirror_write_file(&left.join("upd.txt"), "v2\n");
        if !wrote {
            let _ = std::fs::remove_dir_all(&root);
            return;
        }
        if !mirror_set_old_mtime(&right.join("upd.txt")) {
            let _ = std::fs::remove_dir_all(&root);
            return;
        }
        let planned = App::plan_mirror_copy(&left, &right);
        let Ok(plan) = planned else {
            let _ = std::fs::remove_dir_all(&root);
            return;
        };
        // Safe subset: only left->right copies, no deletes.
        for step in &plan.steps {
            assert_eq!(step.action, SyncAction::CopyToRight);
        }
        assert!(plan.steps.iter().any(|s| s.rel_path == "new.txt"));
        let applied = App::apply_mirror_copy(&left, &right, &plan);
        let Ok(report) = applied else {
            let _ = std::fs::remove_dir_all(&root);
            return;
        };
        assert!(report.failed.is_empty());
        assert_eq!(report.deleted, 0);
        assert!(report.copied >= 1);
        // Right now mirrors left for the copied paths...
        assert_eq!(
            mirror_read_file(&right.join("new.txt")),
            Some(String::from("fresh\n"))
        );
        assert_eq!(
            mirror_read_file(&right.join("upd.txt")),
            Some(String::from("v2\n"))
        );
        // ...while the right-only file is left alone (no prune here).
        assert_eq!(
            mirror_read_file(&right.join("keep.txt")),
            Some(String::from("mine\n"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn folder_mirror_apply_defaults_to_dry_run_and_needs_consent() {
        let mut app = App::default();
        // Dry-run default on, consent off: no write path open.
        assert!(app.folder_sync_dry_run);
        assert!(!app.folder_session.consented);
        assert!(!app.folder_should_write());
        assert!(app.folder_sync_last_report.is_none());
        app.set_folder_sync_dry_run(false);
        assert!(!app.folder_sync_dry_run);
        // Consent still missing, so writes stay closed.
        assert!(!app.folder_should_write());
        app.set_folder_consent(true);
        // Bina compare rows ke steps nahi, isliye abhi bhi band.
        assert!(!app.folder_should_write());
        // Dry-run wapas on: report line banti hai, write nahi hota.
        app.set_folder_sync_dry_run(true);
        assert!(app.status.contains("dry-run"));
    }

    #[test]
    fn apply_reports_clear_on_swap() {
        // Sync tab: swap drops the stale report + confirm, dry-run rehta hai.
        let mut app = App::default();
        app.set_tab(Tab::Sync);
        app.sync_model = Some(SyncChecklist::from_plan(&sync_plan_fixture()));
        app.sync_last_report = Some(String::from("old report"));
        app.sync_confirmed = true;
        assert!(app.swap_sides());
        assert!(app.sync_last_report.is_none());
        assert!(!app.sync_confirmed);
        assert!(app.sync_dry_run);
        // Folder tab: swap drops the stale mirror report.
        app.set_tab(Tab::Folder);
        app.set_folder_paths("left-dir", "right-dir");
        app.folder_sync_last_report = Some(String::from("old report"));
        assert!(app.swap_sides());
        assert!(app.folder_sync_last_report.is_none());
        assert!(app.folder_sync_dry_run);
    }

    #[test]
    fn sescaller_apply_switch_loads_paths_and_cursor() {
        let mut app = App::default();
        app.set_tab(Tab::Text);
        app.set_text_paths("a.txt", "b.txt");
        app.text_find_cursor = 4;
        app.set_tab(Tab::Folder);
        app.set_folder_paths("dir_a", "dir_b");
        let text_id = app
            .sessions
            .list()
            .iter()
            .find(|item| item.kind == Tab::Text)
            .map(|item| item.id);
        let Some(text_id) = text_id else {
            return;
        };
        assert!(app.apply_session_action(views::SessionAction::Switch(text_id)));
        assert_eq!(app.current_tab(), Tab::Text);
        assert_eq!(app.text_paths(), ("a.txt".to_string(), "b.txt".to_string()));
        assert_eq!(app.text_find_cursor, 4);
        assert!(app.status.contains("switched session"));
        assert!(!app.apply_session_action(views::SessionAction::Switch(999_999)));
        assert!(!app.apply_session_action(views::SessionAction::None));
    }

    #[test]
    fn sescaller_apply_close_only_hits_and_neighbours() {
        let mut app = App::default();
        app.set_tab(Tab::Text);
        app.set_tab(Tab::Folder);
        app.set_tab(Tab::Audio);
        assert_eq!(app.sessions.len(), 3);
        let folder_id = app
            .sessions
            .list()
            .iter()
            .find(|item| item.kind == Tab::Folder)
            .map(|item| item.id);
        let Some(folder_id) = folder_id else {
            return;
        };
        assert!(app.apply_session_action(views::SessionAction::Close(folder_id)));
        assert_eq!(app.sessions.len(), 2);
        assert_eq!(app.current_tab(), Tab::Audio);
        assert!(!app.apply_session_action(views::SessionAction::Close(999_999)));
        assert!(!app.apply_session_action(views::SessionAction::None));
        let ids: Vec<u64> = app.sessions.list().iter().map(|item| item.id).collect();
        for id in ids {
            assert!(app.apply_session_action(views::SessionAction::Close(id)));
        }
        assert!(app.sessions.is_empty());
        assert_eq!(app.current_tab(), Tab::Home);
    }

    #[test]
    fn sescaller_typed_paths_saved_into_active() {
        let mut app = App::default();
        app.set_tab(Tab::Text);
        // Setter bypass: paint `&mut String` me type karne ka simulation.
        app.text.left = String::from("  typed_a.txt  ");
        app.text.right = String::from("typed_b.txt");
        app.save_slots_into_active();
        let stored = app
            .sessions
            .active()
            .map(|item| (item.left.clone(), item.right.clone()));
        assert_eq!(
            stored,
            Some(("typed_a.txt".to_string(), "typed_b.txt".to_string()))
        );
    }

    #[test]
    fn sescaller_cursor_saved_and_restored_across_switch() {
        let mut app = App::default();
        app.set_tab(Tab::Text);
        app.text_find_cursor = 7;
        app.save_slots_into_active();
        let before = app.sessions.active().map(|item| item.cursor);
        assert_eq!(before, Some(7));
        app.set_tab(Tab::Folder);
        app.text_find_cursor = 0;
        let text_id = app
            .sessions
            .list()
            .iter()
            .find(|item| item.kind == Tab::Text)
            .map(|item| item.id);
        let Some(text_id) = text_id else {
            return;
        };
        assert!(app.apply_session_action(views::SessionAction::Switch(text_id)));
        assert_eq!(app.text_find_cursor, 7);
    }

    #[test]
    fn modet7_rename_session_trims_and_rejects_blank_unknown() {
        let mut app = App::default();
        app.set_tab(Tab::Text);
        let id = match app.sessions.active_id() {
            Some(id) => id,
            None => return,
        };
        assert!(app.rename_session(id, "  custom  "));
        assert_eq!(
            app.sessions.get(id).map(|item| item.title.clone()),
            Some(String::from("custom"))
        );
        assert!(app.status.contains("renamed session"));
        assert_eq!(app.current_tab(), Tab::Text);
        assert!(!app.rename_session(id, "   "));
        assert!(!app.rename_session(999_999, "zed"));
        assert_eq!(
            app.sessions.get(id).map(|item| item.title.clone()),
            Some(String::from("custom"))
        );
    }

    #[test]
    fn opener_flag_parses_and_preloads_text_tab() {
        // CN-OPENER: `--open <left> <right> --tab text` parse + preload.
        // Launch par dono path session me preload hote hain aur text tab
        // khulta hai (live capture input-gate ka hal). GUI nahi khulta.
        let args = vec![
            String::from("autocompare-gui"),
            String::from("--open"),
            String::from("left.txt"),
            String::from("right.txt"),
            String::from("--tab"),
            String::from("text"),
        ];
        let parsed = App::parse_open_args(&args);
        assert!(parsed.is_ok(), "opener flag must parse: {parsed:?}");
        let req = match parsed {
            Ok(inner) => inner,
            Err(_) => return,
        };
        let req = match req {
            Some(req) => req,
            None => return,
        };
        assert_eq!(req.left, String::from("left.txt"));
        assert_eq!(req.right, String::from("right.txt"));
        assert_eq!(req.tab, Tab::Text);
        let mut app = App::default();
        app.apply_open_request(&req);
        assert_eq!(app.current_tab(), Tab::Text);
        assert_eq!(
            app.text_paths(),
            (String::from("left.txt"), String::from("right.txt"))
        );
        assert_eq!(app.sessions.len(), 1);
    }

    #[test]
    fn opener_flag_defaults_rejects_and_help() {
        // CN-OPENER: default tab text; missing/unknown values Err; flag na
        // ho to None (existing behavior untouched); `--help` GUI-safe.
        let prog = String::from("autocompare-gui");
        assert_eq!(App::parse_open_args(std::slice::from_ref(&prog)), Ok(None));
        assert_eq!(
            App::parse_open_args(&[
                prog.clone(),
                String::from("--open"),
                String::from("a.txt"),
                String::from("b.txt"),
            ]),
            Ok(Some(OpenRequest {
                left: String::from("a.txt"),
                right: String::from("b.txt"),
                tab: Tab::Text,
            }))
        );
        assert_eq!(App::tab_from_name("TEXT"), Some(Tab::Text));
        assert_eq!(App::tab_from_name("folderMerge"), Some(Tab::FolderMerge));
        assert_eq!(App::tab_from_name("home"), None);
        assert_eq!(App::tab_from_name("bogus"), None);
        assert!(
            App::parse_open_args(&[prog.clone(), String::from("--open")]).is_err(),
            "lone --open must fail"
        );
        assert!(
            App::parse_open_args(&[prog.clone(), String::from("--open"), String::from("a.txt"),])
                .is_err(),
            "single path must fail"
        );
        assert!(
            App::parse_open_args(&[
                prog.clone(),
                String::from("--open"),
                String::from("a.txt"),
                String::from("--tab"),
            ])
            .is_err(),
            "flag as path must fail"
        );
        assert!(
            App::parse_open_args(&[
                prog.clone(),
                String::from("--open"),
                String::from("a.txt"),
                String::from("b.txt"),
                String::from("--tab"),
                String::from("bogus"),
            ])
            .is_err(),
            "unknown tab must fail"
        );
        assert!(!App::help_requested(std::slice::from_ref(&prog)));
        assert!(App::help_requested(&[prog, String::from("--help")]));
        assert!(App::help_text().contains("--open <left> <right>"));
        // Home par opener kuch nahi kholta (launcher jhooth hota).
        let mut app = App::default();
        assert!(!app.apply_open_request(&OpenRequest {
            left: String::from("a.txt"),
            right: String::from("b.txt"),
            tab: Tab::Home,
        }));
        assert!(app.sessions.is_empty());
    }

    #[test]
    fn modet7_modetab_click_adopts_session() {
        let mut app = App::default();
        app.set_tab(Tab::Text);
        app.set_text_paths("a.txt", "b.txt");
        app.set_tab(Tab::Folder);
        assert_eq!(app.sessions.len(), 2);
        // Mode-tab click simulation (update wiring jaisa): paint pick
        // buffer me, badla to `set_tab` adopt — same-kind par switch,
        // duplicate open nahi, paths load hote hain.
        let mut picked = Tab::Text;
        if picked != app.tab {
            app.set_tab(picked);
        }
        assert_eq!(app.sessions.len(), 2);
        assert_eq!(app.current_tab(), Tab::Text);
        assert_eq!(app.text_paths(), ("a.txt".to_string(), "b.txt".to_string()));
        // Fresh kind par click: naya session khulta hai (adopt-open).
        picked = Tab::Hex;
        if picked != app.tab {
            app.set_tab(picked);
        }
        assert_eq!(app.sessions.len(), 3);
        assert_eq!(app.current_tab(), Tab::Hex);
    }
}
