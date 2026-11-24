//! IPC command registry root (Wave 1R).
//!
//! # Ownership
//!
//! Wave 1R owns this file. The eight domain modules are owned by their
//! respective leaves (1A/1B/1C/1D) — this file only re-exports them and
//! must not add logic. `lib.rs` registers every `#[tauri::command]` below
//! in `tauri::generate_handler![...]` and mirrors the names in `COMMANDS`
//! (audit trail); keep the three lists in sync.
//!
//! Domain map:
//! - `text` / `folder` — Wave 1A (text+folder compare)
//! - `hex` / `table` — Wave 1B (hex+table compare)
//! - `merge` / `sync` — Wave 1C (merge+sync)
//! - `media` / `session` — Wave 1D (media compare + session/settings/report)

#![forbid(unsafe_code)]

pub mod folder;
pub mod hex;
pub mod media;
pub mod merge;
pub mod session;
pub mod sync;
pub mod table;
pub mod text;

pub use text::{read_file_meta, FileMetaDto};
