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
    // CN-OPENER: `--help` GUI khole bina usage dekar exit (help-safe).
    if app::App::help_requested(&args) {
        print!("{}", app::App::help_text());
        return;
    }
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

    // CN-OPENER: `--open <left> <right> [--tab <name>]` non-interactive
    // preload. Parse fail par exit(2); success par launch ke waqt paths
    // preload + tab khula + auto-compare. Screenshot route untouched.
    let opener = match app::App::parse_open_args(&args) {
        Ok(request) => request,
        Err(err) => {
            eprintln!("autocompare-gui: {err}");
            std::process::exit(2);
        }
    };

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_title("Home - autocompare"),
        ..Default::default()
    };
    if let Err(err) = eframe::run_native(
        "autocompare",
        options,
        Box::new(move |cc| {
            let mut app = app::App::default();
            if let Some(storage) = cc.storage {
                app.dark_mode = app::App::dark_mode_from_storage(
                    storage
                        .get_string(app::App::DARK_MODE_STORAGE_KEY)
                        .as_deref(),
                );
            }
            // Opener paths/slots theme se alag hain, isliye order safe hai:
            // pehle theme, phir preload + auto-compare.
            if let Some(request) = opener.as_ref() {
                app.apply_open_request(request);
            }
            cc.egui_ctx.set_visuals(if app.dark_mode {
                eframe::egui::Visuals::dark()
            } else {
                eframe::egui::Visuals::light()
            });
            Ok(Box::new(app))
        }),
    ) {
        eprintln!("autocompare-gui: {err}");
    }
}
