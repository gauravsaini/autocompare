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
}
