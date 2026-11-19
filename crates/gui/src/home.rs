//! Home View session launcher model.
//!
//! Provides the 9 comparison session types matching the CLI commands.

#![forbid(unsafe_code)]
#![allow(dead_code)]

/// The nine comparison tabs plus the Home launcher view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Home,
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

impl Tab {
    /// Returns the human-readable display title for this tab.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Text => "Text",
            Self::Folder => "Folder",
            Self::Hex => "Hex",
            Self::Merge => "Merge",
            Self::Table => "Table",
            Self::Sync => "Sync",
            Self::Picture => "Picture",
            Self::FolderMerge => "FolderMerge",
            Self::Audio => "Audio",
        }
    }
}

/// One session launcher card in the Home View.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeItem {
    pub title: &'static str,
    pub description: &'static str,
    pub tab: Tab,
}

/// Headless model for the Home View launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeModel {
    pub items: Vec<HomeItem>,
}

/// Builds the pure Home View model with all 9 comparison session types.
pub fn home_model() -> HomeModel {
    HomeModel {
        items: vec![
            HomeItem {
                title: "Text Compare",
                description: "Side-by-side text comparison with inline difference highlighting",
                tab: Tab::Text,
            },
            HomeItem {
                title: "Folder Compare",
                description: "Compare folder hierarchies, timestamps, and file sizes",
                tab: Tab::Folder,
            },
            HomeItem {
                title: "Hex Compare",
                description: "Binary byte-by-byte comparison with hex offset layout",
                tab: Tab::Hex,
            },
            HomeItem {
                title: "Text Merge",
                description: "Three-way text reconciliation with conflict markers and Take resolution",
                tab: Tab::Merge,
            },
            HomeItem {
                title: "Table Compare",
                description: "Structured row-and-column comparison for delimited tables",
                tab: Tab::Table,
            },
            HomeItem {
                title: "Folder Sync",
                description: "Two-way directory synchronization with previewed action steps",
                tab: Tab::Sync,
            },
            HomeItem {
                title: "Picture Compare",
                description: "Visual image diffing with difference metrics and overlap heatmap",
                tab: Tab::Picture,
            },
            HomeItem {
                title: "Folder Merge",
                description: "Three-way folder merge with automatic and conflict resolution",
                tab: Tab::FolderMerge,
            },
            HomeItem {
                title: "Audio Compare",
                description: "Audio stream waveform and channel difference analysis",
                tab: Tab::Audio,
            },
        ],
    }
}

/// Home card typography (family, weight, size only; colors out of
/// scope). Matches `views::paint_home`: title strong 14, description
/// small weak, button body scale. GEO/integration wave applies these
/// in paint; this module stays headless so the contract is testable.
pub const HOME_CARD_TITLE_SIZE: f32 = 14.0;
/// Home card title is strong.
pub const HOME_CARD_TITLE_STRONG: bool = true;
/// Home card description is small + weak.
pub const HOME_CARD_DESC_SMALL_WEAK: bool = true;
/// Home card button uses the body scale (default size, no small/strong).
pub const HOME_CARD_BUTTON_BODY_SIZE: f32 = crate::theme::TYPO_BODY_SIZE;

/// Title point size for Home cards (via role, single source: 14.0).
pub const fn home_card_title_size() -> f32 {
    crate::theme::TypoRole::HomeTitle.size()
}

/// Whether the Home card description paints small + weak.
pub const fn home_card_desc_is_small_weak() -> bool {
    HOME_CARD_DESC_SMALL_WEAK
}

/// Button point size for Home cards (via role, single source: body 13.0).
pub const fn home_card_button_size() -> f32 {
    crate::theme::TypoRole::HomeButton.size()
}

/// Home card title role: strong proportional 14. Typography only;
/// paint geometry (card widths, spacing) lives elsewhere.
pub const fn home_card_title_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::HomeTitle
}

/// Home card description role: small weak proportional 11.
pub const fn home_card_desc_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::HomeDesc
}

/// Home card button role: default proportional body scale.
pub const fn home_card_button_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::HomeButton
}

// ----------------------------------------------------------------------------
// Home geometry contract (headless; paint adoption `views.rs` owner ke paas).
// Geometry first: widths, heights, margins, padding, alignment — session
// cards, recent list, buttons. Sab values maujooda paint/rhythm tokens se
// derive hain taaki contract paint se jhooth na bole; colors untouched.
// ----------------------------------------------------------------------------

/// Session-card grid columns: 9 cards / 3 = 3 poori rows, section rhythm barkarar.
pub const HOME_GRID_COLUMNS: usize = 3;
/// Card min width px: `views::CARD_W` parity (paint wahi rakhta hai).
pub const HOME_CARD_MIN_W: f32 = 240.0;
/// Card outer pad px: section rhythm (`views::SECTION_GAP` = 8).
pub const HOME_CARD_PAD: f32 = 8.0;
/// Card inner gap px: tight rhythm (`views::TIGHT_GAP` = 4, title/desc/button beech).
pub const HOME_CARD_INNER_GAP: f32 = 4.0;
/// Card button min hit height px: find-nav 24px floor precedent
/// (`app::LAYOUT_FIND_HIT_MIN`), taaki "Open …" dabane layak rahe.
pub const HOME_CARD_BUTTON_MIN_H: f32 = 24.0;
/// Description wrap budget: do lines tak sab cards ek jaisi height rakhen.
pub const HOME_CARD_DESC_LINES: usize = 2;
/// Session-start recent list cap: 8 entries, naya sabse upar.
pub const HOME_RECENT_MAX: usize = 8;

/// Grid rows for `item_count` cards at [`HOME_GRID_COLUMNS`] (ceil; khali par 0).
/// Pure; paint `Grid` row count isi se aani chahiye.
pub const fn home_grid_rows(item_count: usize) -> usize {
    item_count.div_ceil(HOME_GRID_COLUMNS)
}

/// Row-major cell of card `index`: (row, col). Pure; paint `Grid` order mirror.
pub const fn home_grid_cell(index: usize) -> (usize, usize) {
    (index / HOME_GRID_COLUMNS, index % HOME_GRID_COLUMNS)
}

/// Card min height px: title ek line + desc [`HOME_CARD_DESC_LINES`] lines
/// (dono apne role ke line-height px se) + button hit floor + do inner gaps +
/// upar-neeche pad. Koi magic number nahi — badlav token se aata hai.
/// Pure; headless-tested.
pub fn home_card_min_height() -> f32 {
    use crate::theme::TypoRole;
    TypoRole::HomeTitle.line_height_px()
        + TypoRole::HomeDesc.line_height_px() * HOME_CARD_DESC_LINES as f32
        + HOME_CARD_BUTTON_MIN_H
        + HOME_CARD_INNER_GAP * 2.0
        + HOME_CARD_PAD * 2.0
}

/// Session-start recent list: haal ke sessions, naya sabse upar, cap tak.
/// Pure headless model; paint (recent list UI) isko read-only dikhata hai.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentSessions {
    entries: Vec<String>,
}

impl RecentSessions {
    /// Khali recent list.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Entry ko sabse upar rakhta hai: khali ignore, maujooda upar aata hai,
    /// cap se zyada girta hai. Pure state; kabhi fail nahi hota.
    pub fn push(&mut self, entry: impl Into<String>) {
        let text = entry.into();
        if text.trim().is_empty() {
            return;
        }
        self.entries.retain(|existing| existing != &text);
        self.entries.insert(0, text);
        self.entries.truncate(HOME_RECENT_MAX);
    }

    /// Naya-sabse-pehle order me entries.
    pub fn list(&self) -> &[String] {
        &self.entries
    }

    /// Entry ginti.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Khali hai ya nahi.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Sab entries hatata hai.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

// ----------------------------------------------------------------------------
// Session restore model (Wave AP-SESSION): recent list se purana session
// reopen — last state (tab, paths, cursor) snapshot me rehta hai.
// Geometry first (recent list rows), typography second, colors untouched.
// Paint (`views.rs` owner) isko read-only dikhata hai; record/restore pure
// headless hai taaki empty/missing paths graceful rahen, koi panic nahi.
// ----------------------------------------------------------------------------

/// Ek saved session ka last state: kaun sa tab, dono (ya teenon) paths,
/// aur us tab ka find cursor. `base` two-path tabs par khali rehta hai.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    /// Kaun sa session tab tha.
    pub tab: Tab,
    /// Left path (trimmed, khali ho sakta hai).
    pub left: String,
    /// Right path (trimmed, khali ho sakta hai).
    pub right: String,
    /// Base path (three-way tabs par, warna khali).
    pub base: String,
    /// Us tab ka cursor (text/table find cursor, folder cursor, hex offset
    /// cursor ka `usize` roop; baaki tabs par 0).
    pub cursor: usize,
}

impl SessionSnapshot {
    /// Snapshot banata hai; paths trim hote hain, cursor jaisa hai waisa.
    pub fn new(
        tab: Tab,
        left: impl Into<String>,
        right: impl Into<String>,
        base: impl Into<String>,
        cursor: usize,
    ) -> Self {
        Self {
            tab,
            left: left.into().trim().to_string(),
            right: right.into().trim().to_string(),
            base: base.into().trim().to_string(),
            cursor,
        }
    }

    /// Three-way session hai ya nahi (Merge / FolderMerge).
    pub const fn is_three_way(&self) -> bool {
        matches!(self.tab, Tab::Merge | Tab::FolderMerge)
    }

    /// Koi zaroori path khali hai ya nahi. Three-way par base bhi chahiye,
    /// two-path par sirf left/right. Paint isi se graceful hint dikhata hai.
    pub fn has_empty_paths(&self) -> bool {
        if self.left.trim().is_empty() || self.right.trim().is_empty() {
            return true;
        }
        if self.is_three_way() && self.base.trim().is_empty() {
            return true;
        }
        false
    }
}

/// Session-start recent history: snapshots, naya sabse upar, cap tak.
/// `RecentSessions` (titles) paint-compat ke liye rehta hai; ye full state
/// restore ke liye hai. Dono cap [`HOME_RECENT_MAX`] share karte hain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionHistory {
    entries: Vec<SessionSnapshot>,
}

impl SessionHistory {
    /// Khali history.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Snapshot sabse upar rakhta hai: Home tab kabhi record nahi hota,
    /// same (tab, paths) dedupe hokar upar aata hai, cap se zyada girta hai.
    pub fn push(&mut self, snapshot: SessionSnapshot) {
        if snapshot.tab == Tab::Home {
            return;
        }
        self.entries.retain(|existing| {
            existing.tab != snapshot.tab
                || existing.left != snapshot.left
                || existing.right != snapshot.right
                || existing.base != snapshot.base
        });
        self.entries.insert(0, snapshot);
        self.entries.truncate(HOME_RECENT_MAX);
    }

    /// Naya-sabse-pehle order me snapshots.
    pub fn list(&self) -> &[SessionSnapshot] {
        &self.entries
    }

    /// Entry ginti.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Khali hai ya nahi.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Sab entries hatata hai.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Index par snapshot, out-of-range par `None` (graceful, koi panic nahi).
    pub fn get(&self, index: usize) -> Option<&SessionSnapshot> {
        self.entries.get(index)
    }
}

// ----------------------------------------------------------------------------
// Recent list geometry (first): rows, gaps, pad — sab maujooda rhythm tokens
// se derive, koi magic number nahi, colors untouched.
// ----------------------------------------------------------------------------

/// Recent row gap px: card inner gap (`HOME_CARD_INNER_GAP` = 4) reuse.
pub const RECENT_ROW_GAP: f32 = HOME_CARD_INNER_GAP;
/// Recent list outer pad px: card outer pad (`HOME_CARD_PAD` = 8) reuse.
pub const RECENT_LIST_PAD: f32 = HOME_CARD_PAD;

/// Recent row min height px: ek desc-role line + inner gap + outer pad.
/// Single-line row text kabhi clip na ho, button floor (24px) se upar rahe.
pub fn recent_row_min_height() -> f32 {
    use crate::theme::TypoRole;
    TypoRole::HomeDesc.line_height_px() + HOME_CARD_INNER_GAP + HOME_CARD_PAD
}

/// `count` rows wali recent list ki kul height px: rows + beech gaps +
/// upar-neeche pad. Khali par 0 (paint khali list chhupata hai).
pub fn recent_list_height(count: usize) -> f32 {
    if count == 0 {
        return 0.0;
    }
    let rows = count as f32 * recent_row_min_height();
    let gaps = count.saturating_sub(1) as f32 * RECENT_ROW_GAP;
    rows + gaps + RECENT_LIST_PAD * 2.0
}

/// Row `index` ka top offset px (list pad ke andar). Pure; paint row isi se lagaye.
pub fn recent_row_top(index: usize) -> f32 {
    RECENT_LIST_PAD + index as f32 * (recent_row_min_height() + RECENT_ROW_GAP)
}

// ----------------------------------------------------------------------------
// Recent list typography (second): row text role reuse, koi naya size nahi.
// ----------------------------------------------------------------------------

/// Recent row text role: chhota kamzor proportional (secondary list text).
pub const fn recent_row_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::HomeDesc
}

/// Recent row text size px (single source: desc role).
pub fn recent_row_text_size() -> f32 {
    crate::theme::TypoRole::HomeDesc.size()
}

/// Recent row text line-height px (body 1.40 x desc size): row stack isi se.
pub fn recent_row_text_line_height_px() -> f32 {
    crate::theme::TypoRole::HomeDesc.line_height_px()
}

/// Recent row text tracking em (headers wala halka failao nahi, zero).
pub fn recent_row_text_tracking_em() -> f32 {
    crate::theme::TypoRole::HomeDesc.letter_spacing_em()
}

/// Recent row text numeric weight (regular 400).
pub fn recent_row_text_weight_value() -> u16 {
    crate::theme::TypoRole::HomeDesc.weight_value()
}

// ----------------------------------------------------------------------------
// Home typography layer-2 (second): family/weight/line-height/tracking —
// sab `TypoRole` precedent reuse, koi naya point size nahi, colors untouched.
// ----------------------------------------------------------------------------

/// Title line-height px (chrome 1.2 x 14): card stack isi se banta hai.
pub const fn home_card_title_line_height_px() -> f32 {
    crate::theme::TypoRole::HomeTitle.line_height_px()
}

/// Description line-height px (body 1.40 x 11): do-line budget isi se.
pub const fn home_card_desc_line_height_px() -> f32 {
    crate::theme::TypoRole::HomeDesc.line_height_px()
}

/// Title tracking em (0.02): headers wala halka failao, baaki zero.
pub const fn home_card_title_tracking_em() -> f32 {
    crate::theme::TypoRole::HomeTitle.letter_spacing_em()
}

/// Title numeric weight (medium 500).
pub const fn home_card_title_weight_value() -> u16 {
    crate::theme::TypoRole::HomeTitle.weight_value()
}

/// Description numeric weight (regular 400).
pub const fn home_card_desc_weight_value() -> u16 {
    crate::theme::TypoRole::HomeDesc.weight_value()
}

/// Button numeric weight (medium 500).
pub const fn home_card_button_weight_value() -> u16 {
    crate::theme::TypoRole::HomeButton.weight_value()
}

/// Teenon card roles chrome (proportional UI) hain, mono nahi: card text
/// code bankar nahi, launcher bankar dikhta hai. Pure; headless-tested.
pub fn home_card_roles_are_chrome_proportional() -> bool {
    use crate::theme::TypoRole;
    let roles = [
        TypoRole::HomeTitle,
        TypoRole::HomeDesc,
        TypoRole::HomeButton,
    ];
    roles
        .iter()
        .all(|role| role.is_chrome() && role.family() == "proportional")
}

/// Empty-state guidance for the Folder tab so the view never looks
/// blank: rows win, then errors, then the idle hint.
pub const fn folder_empty_hint(has_rows: bool, has_error: bool) -> &'static str {
    if has_rows {
        ""
    } else if has_error {
        "compare failed: fix the message above, then Compare again"
    } else {
        "type both folder paths, press Compare, then expand a folder"
    }
}

/// Routes a staged folder double-click to the tab that opens it. A
/// staged file path always lands in the Text session; with no staging
/// the host stays on Folder.
pub fn resolve_compare_tab(staged: &Option<String>) -> Tab {
    if staged.is_some() {
        Tab::Text
    } else {
        Tab::Folder
    }
}

/// Consent hint for the sync Apply gate.
pub const fn sync_consent_hint(consented: bool, steps: usize) -> &'static str {
    if steps == 0 {
        "nothing to apply: the preview is empty"
    } else if consented {
        "consented: Apply will run the planned steps"
    } else {
        "preview only: tick consent to enable Apply"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_model_covers_all_nine_session_types() {
        let model = home_model();
        assert_eq!(model.items.len(), 9);
        assert!(model.items.iter().any(|i| i.tab == Tab::Text));
        assert!(model.items.iter().any(|i| i.tab == Tab::Folder));
        assert!(model.items.iter().any(|i| i.tab == Tab::Hex));
        assert!(model.items.iter().any(|i| i.tab == Tab::Merge));
        assert!(model.items.iter().any(|i| i.tab == Tab::Table));
        assert!(model.items.iter().any(|i| i.tab == Tab::Sync));
        assert!(model.items.iter().any(|i| i.tab == Tab::Picture));
        assert!(model.items.iter().any(|i| i.tab == Tab::FolderMerge));
        assert!(model.items.iter().any(|i| i.tab == Tab::Audio));
    }

    #[test]
    fn folder_empty_hint_never_leaves_view_blank() {
        assert_eq!(folder_empty_hint(true, false), "");
        assert!(folder_empty_hint(false, true).contains("Compare again"));
        assert!(folder_empty_hint(false, false).contains("type both folder paths"));
    }

    #[test]
    fn sync_consent_hint_tracks_gate() {
        assert!(sync_consent_hint(false, 0).contains("nothing to apply"));
        assert!(sync_consent_hint(false, 3).contains("preview only"));
        assert!(sync_consent_hint(true, 3).contains("consented"));
    }

    #[test]
    fn staged_file_routes_to_text_otherwise_folder() {
        assert_eq!(
            resolve_compare_tab(&Some("src/a.txt".to_string())),
            Tab::Text
        );
        assert_eq!(resolve_compare_tab(&None), Tab::Folder);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn home_card_roles_carry_pane_hierarchy() {
        use crate::theme::{TYPO_LINE_HEIGHT_SOURCE, TypoRole};
        assert_eq!(home_card_title_role(), TypoRole::HomeTitle);
        assert_eq!(home_card_desc_role(), TypoRole::HomeDesc);
        assert_eq!(home_card_button_role(), TypoRole::HomeButton);
        assert_eq!(home_card_title_role().size(), HOME_CARD_TITLE_SIZE);
        assert!(home_card_title_role().is_strong());
        assert_eq!(home_card_title_role().family(), "proportional");
        assert!(home_card_desc_role().is_small());
        assert!(home_card_desc_role().is_weak());
        assert_eq!(home_card_button_role().size(), HOME_CARD_BUTTON_BODY_SIZE);
        assert!(!home_card_button_role().is_strong());
        assert_eq!(
            home_card_title_role().line_height_source(),
            TYPO_LINE_HEIGHT_SOURCE
        );
    }

    #[test]
    fn home_grid_columns_three_rows_ceil() {
        assert_eq!(super::HOME_GRID_COLUMNS, 3);
        assert_eq!(super::home_grid_rows(0), 0);
        assert_eq!(super::home_grid_rows(1), 1);
        assert_eq!(super::home_grid_rows(9), 3);
        assert_eq!(super::home_grid_rows(10), 4);
        assert_eq!(super::home_grid_rows(super::home_model().items.len()), 3);
    }

    #[test]
    fn home_grid_cell_is_row_major() {
        assert_eq!(super::home_grid_cell(0), (0, 0));
        assert_eq!(super::home_grid_cell(2), (0, 2));
        assert_eq!(super::home_grid_cell(3), (1, 0));
        assert_eq!(super::home_grid_cell(8), (2, 2));
    }

    #[test]
    fn home_card_min_height_fits_stack() {
        use crate::theme::TypoRole;
        let height = super::home_card_min_height();
        let expected = TypoRole::HomeTitle.line_height_px()
            + TypoRole::HomeDesc.line_height_px() * super::HOME_CARD_DESC_LINES as f32
            + super::HOME_CARD_BUTTON_MIN_H
            + super::HOME_CARD_INNER_GAP * 2.0
            + super::HOME_CARD_PAD * 2.0;
        assert!((height - expected).abs() < 0.001);
        // 14*1.2 + 2*11*1.40 + 24 + 8 + 16 = 95.6: button floor se upar,
        // purane 75px clip se mehfooz.
        assert!((height - 95.6).abs() < 0.01);
        assert!(height > super::HOME_CARD_BUTTON_MIN_H);
        assert_eq!(super::HOME_CARD_MIN_W, 240.0);
    }

    #[test]
    fn home_recent_push_dedupes_and_caps() {
        let mut recent = super::RecentSessions::new();
        assert!(recent.is_empty());
        assert_eq!(recent.len(), 0);
        recent.push("   ");
        assert!(recent.is_empty());
        for idx in 0..10 {
            recent.push(format!("session {idx}"));
        }
        assert_eq!(recent.len(), super::HOME_RECENT_MAX);
        assert_eq!(recent.list().first().map(String::as_str), Some("session 9"));
        recent.push("session 5");
        assert_eq!(recent.len(), super::HOME_RECENT_MAX);
        assert_eq!(recent.list().first().map(String::as_str), Some("session 5"));
        let dupes = recent
            .list()
            .iter()
            .filter(|entry| entry.as_str() == "session 5")
            .count();
        assert_eq!(dupes, 1);
        recent.clear();
        assert!(recent.is_empty());
    }

    #[test]
    fn home_card_typo_second_layer_reuses_roles() {
        use crate::theme::{
            TYPO_LINE_HEIGHT_BODY, TYPO_LINE_HEIGHT_CHROME, TYPO_TRACKING_HEADING_EM,
            TYPO_WEIGHT_MEDIUM, TYPO_WEIGHT_REGULAR, TypoRole,
        };
        assert_eq!(
            super::home_card_title_line_height_px(),
            14.0 * TYPO_LINE_HEIGHT_CHROME
        );
        assert_eq!(
            super::home_card_desc_line_height_px(),
            11.0 * TYPO_LINE_HEIGHT_BODY
        );
        assert_eq!(
            super::home_card_title_tracking_em(),
            TYPO_TRACKING_HEADING_EM
        );
        assert_eq!(super::home_card_title_weight_value(), TYPO_WEIGHT_MEDIUM);
        assert_eq!(super::home_card_desc_weight_value(), TYPO_WEIGHT_REGULAR);
        assert_eq!(super::home_card_button_weight_value(), TYPO_WEIGHT_MEDIUM);
        assert_eq!(TypoRole::HomeButton.family(), "proportional");
        assert!(super::home_card_roles_are_chrome_proportional());
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn home_card_typo_matches_paint_contract() {
        use crate::theme::{TYPO_BODY_SIZE, TYPO_SMALL_SIZE, TypoRole};
        // Title strong 14 = the single `.size(14.0)` in paint_home.
        assert_eq!(home_card_title_size(), 14.0);
        assert_eq!(HOME_CARD_TITLE_SIZE, 14.0);
        assert!(HOME_CARD_TITLE_STRONG);
        assert_eq!(TypoRole::HomeTitle.size(), 14.0);
        assert!(TypoRole::HomeTitle.is_strong());
        // Description small weak.
        assert!(home_card_desc_is_small_weak());
        assert!(HOME_CARD_DESC_SMALL_WEAK);
        assert_eq!(TypoRole::HomeDesc.size(), TYPO_SMALL_SIZE);
        assert!(TypoRole::HomeDesc.is_small());
        assert!(TypoRole::HomeDesc.is_weak());
        // Button body scale.
        assert_eq!(home_card_button_size(), TYPO_BODY_SIZE);
        assert_eq!(TypoRole::HomeButton.size(), TYPO_BODY_SIZE);
    }

    #[test]
    fn session_snapshot_trims_and_flags_empty_paths() {
        let full = super::SessionSnapshot::new(super::Tab::Text, "  left.txt ", "right.txt", "", 2);
        assert_eq!(full.left.as_str(), "left.txt");
        assert!(!full.has_empty_paths());
        assert!(!full.is_three_way());
        let empty = super::SessionSnapshot::new(super::Tab::Text, "", "right.txt", "", 0);
        assert!(empty.has_empty_paths());
        let no_base = super::SessionSnapshot::new(super::Tab::Merge, "l", "r", "   ", 0);
        assert!(no_base.is_three_way());
        assert!(no_base.has_empty_paths());
    }

    #[test]
    fn session_history_push_dedupes_caps_and_skips_home() {
        let mut history = super::SessionHistory::new();
        assert!(history.is_empty());
        history.push(super::SessionSnapshot::new(
            super::Tab::Home,
            "a",
            "b",
            "",
            0,
        ));
        assert!(history.is_empty());
        for idx in 0..10 {
            history.push(super::SessionSnapshot::new(
                super::Tab::Text,
                format!("left {idx}"),
                "right",
                "",
                idx,
            ));
        }
        assert_eq!(history.len(), super::HOME_RECENT_MAX);
        assert_eq!(history.list().first().map(|s| s.cursor), Some(9));
        assert!(history.get(100).is_none());
        history.push(super::SessionSnapshot::new(
            super::Tab::Text,
            "left 5",
            "right",
            "",
            55,
        ));
        assert_eq!(history.len(), super::HOME_RECENT_MAX);
        assert_eq!(history.list().first().map(|s| s.cursor), Some(55));
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn recent_list_geometry_stacks_rows_with_pad_and_gap() {
        assert_eq!(super::RECENT_ROW_GAP, super::HOME_CARD_INNER_GAP);
        assert_eq!(super::RECENT_LIST_PAD, super::HOME_CARD_PAD);
        let row = super::recent_row_min_height();
        assert!(row > super::HOME_CARD_BUTTON_MIN_H);
        assert_eq!(super::recent_list_height(0), 0.0);
        let one = super::recent_list_height(1);
        assert!((one - (row + super::RECENT_LIST_PAD * 2.0)).abs() < 0.001);
        let two = super::recent_list_height(2);
        assert!(
            (two - (row * 2.0 + super::RECENT_ROW_GAP + super::RECENT_LIST_PAD * 2.0)).abs()
                < 0.001
        );
        assert_eq!(super::recent_row_top(0), super::RECENT_LIST_PAD);
        assert!(
            (super::recent_row_top(1) - (super::RECENT_LIST_PAD + row + super::RECENT_ROW_GAP))
                .abs()
                < 0.001
        );
    }

    #[test]
    fn recent_row_typo_reuses_desc_role() {
        use crate::theme::TypoRole;
        assert_eq!(super::recent_row_text_role(), TypoRole::HomeDesc);
        assert_eq!(super::recent_row_text_size(), TypoRole::HomeDesc.size());
        assert_eq!(
            super::recent_row_text_line_height_px(),
            TypoRole::HomeDesc.line_height_px()
        );
        assert_eq!(
            super::recent_row_text_tracking_em(),
            TypoRole::HomeDesc.letter_spacing_em()
        );
        assert_eq!(
            super::recent_row_text_weight_value(),
            TypoRole::HomeDesc.weight_value()
        );
        assert!(super::recent_row_text_role().is_small());
        assert!(super::recent_row_text_role().is_weak());
        assert_eq!(super::recent_row_text_role().family(), "proportional");
    }
}
