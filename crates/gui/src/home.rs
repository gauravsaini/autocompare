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
}
