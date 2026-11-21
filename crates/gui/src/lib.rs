//! autocompare-gui library: headless view-models over the comparison engine.
//!
//! The desktop client is a thin egui shell over this library. Every
//! comparison runs through the same `autocompare-compare` and
//! `autocompare-diff` libraries the CLI uses; [`viewmodel`] turns engine
//! results into pure row models (the crate's test surface — no egui
//! types, no I/O, no panics), [`views`] paints those models, and [`app`]
//! holds session state. Colors are frozen design tokens ([`theme`]); this
//! wave works geometry first, typography second.
//!
//! Binary target (`src/main.rs`) only parses CLI flags (`--help`,
//! `--screenshot`, `--open`) and launches the eframe shell; all logic
//! lives here so `pub` API never trips `dead_code` in the binary.

#![forbid(unsafe_code)]

/// Session state + engine wiring (tabs, compare runs, screenshot modes).
pub mod app;
/// Live editor copy mutations (left/right row transfer).
pub mod copy;
/// Thumbnail gutter geometry (diff ranges to bar segments).
pub mod gutter;
/// Home launcher model (session types matching CLI commands).
pub mod home;
/// Offscreen PNG screenshot generation (headless verification).
pub mod screenshot;
/// Three-way merge Take resolution (Left/Right/Base/Both).
pub mod take;
/// Frozen design tokens (palette, spacing, typography roles).
pub mod theme;
/// Folder tree drill-down (filtering, indentation, sync preview).
pub mod tree;
/// Pure row models over engine results (headless test surface).
pub mod viewmodel;
/// egui painting of the view models (text compare + chrome).
pub mod views;
/// Hex compare view model (byte-level aligned binary sessions).
pub mod views_hex;
/// Table key-compare view model (delimited key-aligned sessions).
pub mod views_table;
