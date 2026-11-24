#![forbid(unsafe_code)]

//! Native application menu (Wave 3A: bundle packaging + native menu).
//!
//! Top levels: File, Edit, Session, View, Tools, Help. Custom items carry
//! stable `id`s; every id in [`MENU_ITEM_IDS`] maps to a real IPC command
//! via [`route_menu_event`]. Menu events carry no payload, so the frontend
//! owns argument supply and actual invocation — routing resolves intent only.
//!
//! Selection also emits a [`MENU_EVENT_NAME`] event carrying
//! [`MenuEventPayload`] so the frontend can react through `listen()`
//! (e.g. View kind switches). Unknown ids log and are ignored.
//! Never panics, never blocks.
//!
//! Standard Edit/predefined items use the OS-provided roles so keyboard
//! shortcuts and localizations behave natively.

use serde::Serialize;
use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};

/// Top-level menu names, in order. Exported so tests and Wave 1 routing can
/// assert the skeleton without reaching into Tauri internals.
pub const MENU_TOP_LEVELS: &[&str] = &["File", "Edit", "Session", "View", "Tools", "Help"];

/// Menu item ids issued by this skeleton. Wave 1 binds each id to a command
/// via [`route_menu_event`].
pub const MENU_ITEM_IDS: &[&str] = &[
    "file.open-session",
    "file.open-folder",
    "file.save-session",
    "file.open-workspace",
    "session.new",
    "session.find",
    "session.take-left",
    "session.take-right",
    "view.kind-text",
    "view.kind-folder",
    "view.kind-hex",
    "view.kind-table",
    "view.kind-media",
    "view.kind-sync",
    "view.kind-merge",
    "view.toggle-theme",
    "view.toggle-layout",
    "tools.settings",
];

/// Frontend event channel emitted on every routed menu selection.
/// Payload is [`MenuEventPayload`]; listeners are optional.
pub const MENU_EVENT_NAME: &str = "menu-event";

/// Payload emitted on [`MENU_EVENT_NAME`] for a routed menu selection.
#[derive(Debug, Clone, Serialize)]
pub struct MenuEventPayload {
    /// The selected menu item id (one of [`MENU_ITEM_IDS`]).
    pub id: String,
    /// The 1:1 IPC command intent from [`route_menu_event`].
    pub command: String,
}

/// Maps a menu item id to its 1:1 IPC command.
///
/// Menu events carry no payload, so the frontend owns argument supply and
/// actual invocation; routing resolves intent only. Returns `None` for
/// unknown ids (caller logs and ignores — never crashes).
pub fn route_menu_event(id: &str) -> Option<&'static str> {
    match id {
        "file.open-session" => Some("session_load"),
        "file.open-folder" => Some("folder_compare"),
        "file.save-session" => Some("session_save"),
        "file.open-workspace" => Some("session_list"),
        "session.new" => Some("session_new"),
        "session.find" => Some("session_find"),
        "session.take-left" => Some("text_copy_section"),
        "session.take-right" => Some("text_copy_section"),
        "view.kind-text" => Some("text_compare"),
        "view.kind-folder" => Some("folder_compare"),
        "view.kind-hex" => Some("hex_compare"),
        "view.kind-table" => Some("table_compare"),
        "view.kind-media" => Some("picture_compare"),
        "view.kind-sync" => Some("sync_plan"),
        "view.kind-merge" => Some("merge_compare"),
        "view.toggle-theme" => Some("theme_toggle"),
        "view.toggle-layout" => Some("layout_toggle"),
        "tools.settings" => Some("settings_open"),
        _ => None,
    }
}

/// `on_menu_event` entry point (called from `lib.rs`).
///
/// Known ids log their target command and emit [`MENU_EVENT_NAME`] for the
/// frontend; unknown ids log and are ignored. Emit failures are swallowed
/// (listeners are optional). Never panics, never blocks.
pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match route_menu_event(id) {
        Some(command) => {
            eprintln!("desktop: menu '{id}' -> command '{command}'");
            let payload = MenuEventPayload {
                id: id.to_string(),
                command: command.to_string(),
            };
            if let Err(err) = app.emit(MENU_EVENT_NAME, payload) {
                eprintln!("desktop: menu event emit failed for '{id}': {err}");
            }
        }
        None => eprintln!("desktop: ignoring unknown menu id '{id}'"),
    }
}

/// Builds the native application menu. Returns the platform menu on success;
/// callers fall back to no menu (never crash) when the backend is unavailable.
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let file = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &MenuItem::with_id(
                app,
                "file.open-session",
                "Open Session\u{2026}",
                true,
                Some("CmdOrCtrl+O"),
            )?,
            &MenuItem::with_id(
                app,
                "file.open-folder",
                "Open Folder\u{2026}",
                true,
                Some("CmdOrCtrl+Shift+O"),
            )?,
            &MenuItem::with_id(
                app,
                "file.open-workspace",
                "Open Workspace\u{2026}",
                true,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "file.save-session",
                "Save Session",
                true,
                Some("CmdOrCtrl+S"),
            )?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, Some("Close Window"))?,
        ],
    )?;

    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, Some("Undo"))?,
            &PredefinedMenuItem::redo(app, Some("Redo"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some("Cut"))?,
            &PredefinedMenuItem::copy(app, Some("Copy"))?,
            &PredefinedMenuItem::paste(app, Some("Paste"))?,
            &PredefinedMenuItem::select_all(app, Some("Select All"))?,
        ],
    )?;

    let session = Submenu::with_items(
        app,
        "Session",
        true,
        &[
            &MenuItem::with_id(app, "session.new", "New Session", true, Some("CmdOrCtrl+N"))?,
            &MenuItem::with_id(
                app,
                "session.find",
                "Find Session",
                true,
                Some("CmdOrCtrl+F"),
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "session.take-left",
                "Take Left",
                true,
                Some("CmdOrCtrl+D"),
            )?,
            &MenuItem::with_id(
                app,
                "session.take-right",
                "Take Right",
                true,
                Some("CmdOrCtrl+E"),
            )?,
        ],
    )?;

    let view = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &MenuItem::with_id(
                app,
                "view.kind-text",
                "Text Compare",
                true,
                Some("CmdOrCtrl+1"),
            )?,
            &MenuItem::with_id(
                app,
                "view.kind-folder",
                "Folder Compare",
                true,
                Some("CmdOrCtrl+2"),
            )?,
            &MenuItem::with_id(
                app,
                "view.kind-hex",
                "Hex Compare",
                true,
                Some("CmdOrCtrl+3"),
            )?,
            &MenuItem::with_id(
                app,
                "view.kind-table",
                "Table Compare",
                true,
                Some("CmdOrCtrl+4"),
            )?,
            &MenuItem::with_id(
                app,
                "view.kind-media",
                "Media Compare",
                true,
                Some("CmdOrCtrl+5"),
            )?,
            &MenuItem::with_id(app, "view.kind-sync", "Sync", true, Some("CmdOrCtrl+6"))?,
            &MenuItem::with_id(
                app,
                "view.kind-merge",
                "Merge Compare",
                true,
                Some("CmdOrCtrl+7"),
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "view.toggle-theme",
                "Toggle Theme (unsupported)",
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "view.toggle-layout",
                "Toggle Layout (unsupported)",
                true,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::fullscreen(app, Some("Toggle Full Screen"))?,
        ],
    )?;

    let tools = Submenu::with_items(
        app,
        "Tools",
        true,
        &[&MenuItem::with_id(
            app,
            "tools.settings",
            "Settings\u{2026} (unsupported)",
            true,
            None::<&str>,
        )?],
    )?;

    let help = Submenu::with_items(
        app,
        "Help",
        true,
        &[
            &PredefinedMenuItem::about(
                app,
                Some("About AutoCompare"),
                Some(AboutMetadata {
                    name: Some("AutoCompare".to_string()),
                    version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    ..Default::default()
                }),
            )?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some("Quit AutoCompare"))?,
        ],
    )?;

    Menu::with_items(app, &[&file, &edit, &session, &view, &tools, &help])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_levels_match_spec() {
        assert_eq!(
            MENU_TOP_LEVELS,
            &["File", "Edit", "Session", "View", "Tools", "Help"]
        );
    }

    #[test]
    fn item_ids_are_namespaced() {
        for id in MENU_ITEM_IDS {
            assert!(id.contains('.'), "menu id must be namespaced: {id}");
        }
    }

    #[test]
    fn every_menu_id_routes_to_a_command() {
        for id in MENU_ITEM_IDS {
            assert!(
                route_menu_event(id).is_some(),
                "menu id has no command mapping: {id}"
            );
        }
    }

    #[test]
    fn route_menu_event_honest_intents() {
        assert_eq!(route_menu_event("session.new"), Some("session_new"));
        assert_eq!(route_menu_event("session.find"), Some("session_find"));
        assert_eq!(route_menu_event("view.kind-merge"), Some("merge_compare"));
        assert_eq!(route_menu_event("view.toggle-theme"), Some("theme_toggle"));
        assert_eq!(route_menu_event("view.toggle-layout"), Some("layout_toggle"));
        assert_eq!(route_menu_event("tools.settings"), Some("settings_open"));
    }

    #[test]
    fn route_menu_event_take_left_right() {
        // Wave-131: Take Left/Right ride the existing text copy IPC
        // intent (no new IPC); the frontend supplies the hunk args.
        assert_eq!(route_menu_event("session.take-left"), Some("text_copy_section"));
        assert_eq!(route_menu_event("session.take-right"), Some("text_copy_section"));
    }

    #[test]
    fn unknown_menu_id_routes_to_none() {
        assert_eq!(route_menu_event("nope.bogus"), None);
        assert_eq!(route_menu_event(""), None);
        // `handle_menu_event` logs and ignores these ids: unknown ids never
        // reach `emit`, and emit failures are swallowed, so no input panics.
    }
}
