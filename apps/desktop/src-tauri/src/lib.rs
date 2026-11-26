#![forbid(unsafe_code)]

//! Tauri v2 desktop shell library.
//!
//! Wave 1R: full IPC surface registered below (`commands/*`, 29 handlers).
//! Menu events route to their 1:1 commands via `menu::handle_menu_event`
//! (which also emits a `menu-event` for the frontend to `listen()` to);
//! the frontend supplies command arguments, so menu routing only resolves
//! the target command name and never crashes on unknown ids.

pub mod commands;
pub mod menu;
pub mod state;

use state::AppState;

/// Full IPC command table (Wave 1R audit trail).
///
/// Every entry here MUST have a matching path in the
/// `tauri::generate_handler![...]` table below. The smoke check
/// (`cargo test`, registry audit) asserts the two lists agree.
pub const COMMANDS: &[&str] = &[
    // Wave 1A: text + folder
    "text_compare",
    "text_get_rows",
    "text_get_overview",
    "text_find",
    "text_copy_section",
    "text_save_side",
    "text_apply_edit",
    "text_apply_hunk",
    "text_restore_rows",
    "text_update_side",
    "stat_paths",
    "folder_compare",
    "folder_get_tree",
    "folder_cancel",
    // Wave 1B: hex + table
    "hex_compare",
    "hex_get_page",
    "hex_find",
    "hex_apply_edit",
    "hex_save_side",
    "table_compare",
    "table_get_page",
    "table_find",
    // Wave 1C: merge + sync
    "merge_text_plan",
    "merge_text_apply",
    "merge_folder_plan",
    "merge_folder_apply",
    "sync_plan",
    "sync_apply",
    // Wave 1D: media + session/settings/report
    "picture_compare",
    "audio_compare",
    "session_list",
    "session_load",
    "session_save",
    "session_delete",
    "settings_get",
    "settings_set",
    "report_export",
];

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            match menu::build_menu(app.handle()) {
                Ok(menu) => {
                    let _ = app.set_menu(menu);
                }
                Err(err) => {
                    eprintln!("desktop: native menu unavailable, continuing: {err}");
                }
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            menu::handle_menu_event(app, event.id().0.as_str());
        })
        .invoke_handler(tauri::generate_handler![
            commands::text::text_compare,
            commands::text::text_get_rows,
            commands::text::text_get_overview,
            commands::text::text_find,
            commands::text::text_copy_section,
            commands::text::text_save_side,
            commands::text::text_apply_edit,
            commands::text::text_apply_hunk,
            commands::text::text_restore_rows,
            commands::text::text_update_side,
            commands::text::stat_paths,
            commands::folder::folder_compare,
            commands::folder::folder_get_tree,
            commands::folder::folder_cancel,
            commands::hex::hex_compare,
            commands::hex::hex_get_page,
            commands::hex::hex_find,
            commands::hex::hex_apply_edit,
            commands::hex::hex_save_side,
            commands::table::table_compare,
            commands::table::table_get_page,
            commands::table::table_find,
            commands::merge::merge_text_plan,
            commands::merge::merge_text_apply,
            commands::merge::merge_folder_plan,
            commands::merge::merge_folder_apply,
            commands::sync::sync_plan,
            commands::sync::sync_apply,
            commands::media::picture_compare,
            commands::media::audio_compare,
            commands::session::session_list,
            commands::session::session_load,
            commands::session::session_save,
            commands::session::session_delete,
            commands::session::settings_get,
            commands::session::settings_set,
            commands::session::report_export,
        ]);

    match builder.build(tauri::generate_context!()) {
        Ok(app) => {
            app.run(|_handle, _event| {});
        }
        Err(err) => {
            eprintln!("desktop: failed to build Tauri app: {err}");
        }
    }
}
