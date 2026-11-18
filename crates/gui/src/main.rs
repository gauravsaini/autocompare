//! autocompare-gui: the desktop client over the comparison engine.
//!
//! A thin client by design: every comparison runs through the same
//! `autocompare-compare` and `autocompare-diff` libraries the CLI
//! uses. Errors render as panels; the app never panics on bad paths
//! or hostile input.
//!
//! Layout: [`app`] holds state and drives the engine, [`viewmodel`]
//! turns engine results into pure row models (the crate's test
//! surface), [`views`] paints those models with egui.

#![forbid(unsafe_code)]

mod app;
mod viewmodel;
mod views;

fn main() {
    let options = eframe::NativeOptions::default();
    if let Err(err) = eframe::run_native(
        "autocompare",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::default()))),
    ) {
        eprintln!("autocompare-gui: {err}");
    }
}
