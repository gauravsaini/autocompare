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

pub mod app;
pub mod copy;
pub mod gutter;
pub mod home;
pub mod screenshot;
pub mod take;
pub mod theme;
pub mod tree;
pub mod viewmodel;
pub mod views;
pub mod views_hex;
pub mod views_table;
