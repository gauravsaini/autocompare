#![forbid(unsafe_code)]

//! Native application menu driven by the single declarative schema shared with
//! `V2MenuBar.tsx` (`apps/desktop/src/v2/menus/menuSchema.json`).
//!
//! Because both Rust (`menu.rs`) and React (`V2MenuBar.tsx`) read the exact
//! same JSON definition, menu labels, accelerators, command intents, and
//! capability-driven enabled/disabled states never drift across desktop and
//! web modes.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use tauri::menu::{AboutMetadata, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};

/// Single source of truth shared with `apps/desktop/src/v2/menus/menuSchema.ts`.
pub const MENU_SCHEMA_JSON: &str = include_str!("../../src/v2/menus/menuSchema.json");

/// Top-level menu names, in order.
pub const MENU_TOP_LEVELS: &[&str] = &["AutoCompare", "Session", "Search", "View", "Tools", "Help"];

/// Frontend event channel emitted on every routed menu selection.
pub const MENU_EVENT_NAME: &str = "menu-event";

/// Payload emitted on [`MENU_EVENT_NAME`] for a routed menu selection.
#[derive(Debug, Clone, Serialize)]
pub struct MenuEventPayload {
    /// The selected menu item id from `menuSchema.json`.
    pub id: String,
    /// The canonical command intent from [`route_menu_event`].
    pub command: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MenuSchemaDef {
    menus: Vec<TopLevelMenuDef>,
}

#[derive(Debug, Clone, Deserialize)]
struct TopLevelMenuDef {
    label: String,
    items: Vec<MenuItemDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum MenuItemDef {
    Action {
        id: String,
        command: String,
        label: String,
        #[serde(default)]
        accelerator: Option<String>,
        #[serde(default)]
        #[serde(rename = "nativeRole")]
        native_role: Option<String>,
    },
    Separator {},
    DynamicSubmenu {
        #[serde(default)]
        id: String,
        #[serde(default)]
        label: String,
    },
}

fn parsed_schema() -> &'static MenuSchemaDef {
    static SCHEMA: OnceLock<MenuSchemaDef> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        serde_json::from_str::<MenuSchemaDef>(MENU_SCHEMA_JSON)
            .unwrap_or(MenuSchemaDef { menus: Vec::new() })
    })
}

fn schema_command_map() -> &'static HashMap<String, String> {
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut out = HashMap::new();
        for menu in &parsed_schema().menus {
            for item in &menu.items {
                if let MenuItemDef::Action { id, command, .. } = item {
                    out.insert(id.clone(), command.clone());
                }
            }
        }
        out
    })
}

/// Maps a menu item id (from `menuSchema.json` or legacy desktop ids) to its
/// canonical command intent.
pub fn route_menu_event(id: &str) -> Option<&'static str> {
    if let Some(cmd) = schema_command_map().get(id) {
        return Some(cmd.as_str());
    }
    // Backward-compatible aliases for legacy v1 menu ids
    match id {
        "file.open-folder" => Some("folder_compare"),
        "file.open-workspace" => Some("session_list"),
        "view.kind-text" => Some("text_compare"),
        "view.kind-folder" => Some("folder_compare"),
        "view.kind-hex" => Some("hex_compare"),
        "view.kind-table" => Some("table_compare"),
        "view.kind-media" => Some("picture_compare"),
        "view.kind-sync" => Some("sync_plan"),
        "view.kind-merge" => Some("merge_compare"),
        _ => None,
    }
}

/// `on_menu_event` entry point (called from `lib.rs`).
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

/// Synchronizes native menu item enabled/disabled states from the frontend's
/// `computeNativeMenuEnabledMap(...)` so OS accelerators stay in sync with
/// active ViewId and view capabilities.
pub fn sync_menu_states<R: Runtime>(app: &AppHandle<R>, enabled_map: &HashMap<String, bool>) {
    let Some(menu) = app.menu() else {
        return;
    };
    for (id, enabled) in enabled_map {
        if let Some(item) = menu.get(id)
            && let Some(menu_item) = item.as_menuitem()
        {
            let _ = menu_item.set_enabled(*enabled);
        }
    }
}

/// Dynamically updates the enabled state of native Take Left / Take Right menu items.
pub fn set_take_menu_enabled<R: Runtime>(app: &AppHandle<R>, enabled: bool) {
    let mut map = HashMap::new();
    map.insert("session.take-left".to_string(), enabled);
    map.insert("session.take-right".to_string(), enabled);
    sync_menu_states(app, &map);
}

/// Builds the native application menu directly from `menuSchema.json`.
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let schema = parsed_schema();
    let mut submenus: Vec<Submenu<R>> = Vec::with_capacity(schema.menus.len());

    for top in &schema.menus {
        let mut owned_items: Vec<Box<dyn IsMenuItem<R>>> = Vec::new();
        for item in &top.items {
            match item {
                MenuItemDef::Separator {} => {
                    owned_items.push(Box::new(PredefinedMenuItem::separator(app)?));
                }
                MenuItemDef::DynamicSubmenu { id, label } => {
                    if !id.is_empty() && !label.is_empty() {
                        owned_items.push(Box::new(MenuItem::with_id(
                            app,
                            id.as_str(),
                            label.as_str(),
                            true,
                            None::<&str>,
                        )?));
                    }
                }
                MenuItemDef::Action {
                    id,
                    label,
                    accelerator,
                    native_role,
                    ..
                } => match native_role.as_deref() {
                    Some("about") => {
                        owned_items.push(Box::new(PredefinedMenuItem::about(
                            app,
                            Some(label.as_str()),
                            Some(AboutMetadata {
                                name: Some("AutoCompare".to_string()),
                                version: Some(env!("CARGO_PKG_VERSION").to_string()),
                                ..Default::default()
                            }),
                        )?));
                    }
                    Some("quit") => {
                        owned_items.push(Box::new(PredefinedMenuItem::quit(
                            app,
                            Some(label.as_str()),
                        )?));
                    }
                    Some("fullscreen") => {
                        owned_items.push(Box::new(PredefinedMenuItem::fullscreen(
                            app,
                            Some(label.as_str()),
                        )?));
                    }
                    _ => {
                        owned_items.push(Box::new(MenuItem::with_id(
                            app,
                            id.as_str(),
                            label.as_str(),
                            true,
                            accelerator.as_deref(),
                        )?));
                    }
                },
            }
        }
        let refs: Vec<&dyn IsMenuItem<R>> = owned_items.iter().map(|b| b.as_ref()).collect();
        submenus.push(Submenu::with_items(app, &top.label, true, &refs)?);
    }

    let top_refs: Vec<&dyn IsMenuItem<R>> = submenus.iter().map(|s| s as &dyn IsMenuItem<R>).collect();
    Menu::with_items(app, &top_refs)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    #[test]
    fn shared_schema_parses_and_matches_top_levels() {
        let schema = parsed_schema();
        let labels: Vec<&str> = schema.menus.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, MENU_TOP_LEVELS);
    }

    #[test]
    fn every_schema_item_is_namespaced_and_routable() {
        let schema = parsed_schema();
        for menu in &schema.menus {
            for item in &menu.items {
                if let MenuItemDef::Action { id, command, .. } = item {
                    assert!(id.contains('.'), "menu id must be namespaced: {id}");
                    assert_eq!(route_menu_event(id), Some(command.as_str()));
                }
            }
        }
    }

    #[test]
    fn route_menu_event_take_left_right_and_unknown() {
        assert_eq!(route_menu_event("session.take-left"), Some("text_copy_left"));
        assert_eq!(route_menu_event("session.take-right"), Some("text_copy_right"));
        assert_eq!(route_menu_event("tools.file-formats"), Some("file_formats_open"));
        assert_eq!(route_menu_event("tools.profiles"), Some("profiles_open"));
        assert_eq!(route_menu_event("nope.bogus"), None);
    }
}
