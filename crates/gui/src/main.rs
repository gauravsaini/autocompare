//! autocompare-gui: the desktop client over the comparison engine.
//!
//! A thin client by design: every comparison runs through the same
//! `autocompare-compare` and `autocompare-diff` libraries the CLI
//! uses. Errors render as panels; the app never panics on bad paths
//! or hostile input.

#![forbid(unsafe_code)]

fn main() {
    let options = eframe::NativeOptions::default();
    if let Err(err) = eframe::run_native(
        "autocompare",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    ) {
        eprintln!("autocompare-gui: {err}");
    }
}

#[derive(Default)]
struct App {}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("autocompare");
        ui.label("desktop client scaffold");
    }
}
