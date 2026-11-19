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
mod copy;
mod gutter;
mod home;
mod screenshot;
mod take;
mod theme;
mod tree;
mod viewmodel;
mod views;
mod views_hex;
mod views_table;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut screenshot_target = None;
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--screenshot" {
            screenshot_target = iter.next().cloned();
        }
    }

    if let Some(path_str) = screenshot_target {
        if let Err(err) = screenshot::render_fixture_png(std::path::Path::new(&path_str)) {
            eprintln!("screenshot failed: {err}");
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_title("Home - autocompare"),
        ..Default::default()
    };
    if let Err(err) = eframe::run_native(
        "autocompare",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::default()))),
    ) {
        eprintln!("autocompare-gui: {err}");
    }
}
