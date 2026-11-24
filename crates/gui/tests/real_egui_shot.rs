//! Live egui screenshots of the real app window (wave-realshot).
//!
//! Boots the REAL [`App`] text-compare view on the parity fixtures via
//! `egui_kittest` (same egui 0.37 the shipped app uses), waits for the
//! background compare, renders headless with wgpu at 1280x800, and writes
//! each shot to `checks/gui-realshot/` (dark `real-egui-dark.png`, light
//! `real-egui-light.png`).
//!
//! Machine-checked per theme: PNG magic + file size + decoded dims +
//! non-blank distinct-color floor (anti-aliased font evidence) + a mean
//! luminance band proving the live theme actually painted (dark shot is
//! dark, light shot is light). No fallback harness: the full `App`
//! (`eframe::App::ui`) boots headless, so the shots are the real widgets.

#![forbid(unsafe_code)]

use autocompare_diff::{decode_png, encode_png};
use autocompare_gui::app::{App, OpenRequest, Tab};
use eframe::egui::{Theme, Vec2};
use egui_kittest::Harness;
use std::collections::HashSet;
use std::path::PathBuf;

/// Kittest window size: 1280x800 live capture (wider than the 960x600
/// hand-written rasterizer canvas, so the shots prove a real window).
const SHOT_W: f32 = 1280.0;
/// Kittest window height (see [`SHOT_W`]).
const SHOT_H: f32 = 800.0;
/// Minimum PNG bytes (mirrors the exquisite harness floor).
const MIN_PNG_BYTES: usize = 8192;
/// Distinct-RGB floor proving anti-aliased font rasterization.
const MIN_DISTINCT_RGB: usize = 500;
/// Dark-mode mean-luminance ceiling: the live dark Visuals render far
/// below this; a flat light rasterizer posing as dark fails here.
const MEAN_DARK_MAX: f64 = 120.0;
/// Light-mode mean-luminance floor (see [`MEAN_DARK_MAX`]).
const MEAN_LIGHT_MIN: f64 = 160.0;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.pop();
    dir
}

fn err(msg: &str) -> String {
    String::from(msg)
}

/// Distinct RGB triples in an RGBA buffer (alpha ignored).
fn distinct_rgb(pixels: &[u8]) -> HashSet<(u8, u8, u8)> {
    let mut seen = HashSet::new();
    let mut iter = pixels.iter();
    while let (Some(r), Some(g), Some(b), Some(_)) =
        (iter.next(), iter.next(), iter.next(), iter.next())
    {
        seen.insert((*r, *g, *b));
    }
    seen
}

/// Mean channel luminance 0..=255 over RGBA pixels (alpha ignored).
fn mean_luminance(pixels: &[u8]) -> f64 {
    let mut sum: u64 = 0;
    let mut count: u64 = 0;
    let mut iter = pixels.iter();
    while let (Some(r), Some(g), Some(b), Some(_)) =
        (iter.next(), iter.next(), iter.next(), iter.next())
    {
        sum += u64::from(*r) + u64::from(*g) + u64::from(*b);
        count += 1;
    }
    if count == 0 {
        return 0.0;
    }
    sum as f64 / count as f64 / 3.0
}

#[test]
fn real_egui_dark_shot() -> Result<(), String> {
    let summary = capture_shot(Theme::Dark, "real-egui-dark.png")?;
    println!("{summary}");
    Ok(())
}

#[test]
fn real_egui_light_shot() -> Result<(), String> {
    let summary = capture_shot(Theme::Light, "real-egui-light.png")?;
    println!("{summary}");
    Ok(())
}

/// Shared dark/light capture: same fixtures, same 1280x800 canvas, same
/// [`MIN_PNG_BYTES`]/[`MIN_DISTINCT_RGB`] floors plus a per-theme
/// luminance band. Writes to `checks/gui-realshot/`.
/// Returns the one-line evidence summary for the caller to print.
fn capture_shot(theme: Theme, file_name: &str) -> Result<String, String> {
    let root = workspace_root();
    let left = root.join("tools/parity-harness/fixtures/t_left.txt");
    let right = root.join("tools/parity-harness/fixtures/t_right.txt");
    for fixture in [&left, &right] {
        if !fixture.is_file() {
            return Err(format!("missing fixture {}", fixture.display()));
        }
    }

    let expect_dark = !matches!(theme, Theme::Light);
    let mut app = App::default();
    // App re-asserts visuals from `dark_mode` every frame, so the harness
    // theme alone cannot flip it: drive the pub flag too (test-only,
    // shipped behavior untouched).
    app.dark_mode = expect_dark;
    let app_dark = app.dark_mode;
    let req = OpenRequest {
        left: left.to_string_lossy().into_owned(),
        right: right.to_string_lossy().into_owned(),
        tab: Tab::Text,
    };
    if !app.apply_open_request(&req) {
        return Err(err("apply_open_request refused the text tab"));
    }

    let mut harness = Harness::builder()
        .with_size(Vec2::new(SHOT_W, SHOT_H))
        .with_accessibility_check(false)
        .allow_missing_glyphs()
        .with_max_steps(60)
        .with_step_dt(0.01)
        .with_theme(theme)
        .wgpu()
        .build_eframe(|_cc| app);

    // Background compare thread: pump frames until the model lands.
    let mut rows = 0usize;
    let mut waited_ms = 0u64;
    while waited_ms < 10_000 {
        harness.run_steps(2);
        match harness.state().text_model.as_ref() {
            Some(model) => {
                rows = model.rows.len();
                break;
            }
            None => {
                std::thread::sleep(std::time::Duration::from_millis(20));
                waited_ms += 20;
            }
        }
    }
    if rows == 0 {
        return Err(err("text compare produced no rows in 10s"));
    }
    // Live view repaints continuously (clock/progress), so `run()` never
    // settles; a bounded step burst is the honest settle for a live app.
    harness.run_steps(8);

    let shot = harness.render().map_err(|why| format!("render: {why}"))?;
    if shot.width() != SHOT_W as u32 || shot.height() != SHOT_H as u32 {
        return Err(format!(
            "unexpected shot size {}x{}",
            shot.width(),
            shot.height()
        ));
    }
    // Wgpu readback leaves alpha 0 on this fully-covered opaque window;
    // force honest opaque alpha (RGB untouched).
    let frame = autocompare_diff::RgbaImage {
        width: shot.width(),
        height: shot.height(),
        pixels: shot.into_raw(),
    }
    .opaque();
    let png = encode_png(&frame).ok_or_else(|| err("encode_png failed"))?;

    // PNG magic (machine-checked, no indexing).
    const MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if !png.starts_with(&MAGIC) {
        return Err(format!("{file_name} has bad PNG magic"));
    }
    if png.len() < MIN_PNG_BYTES {
        return Err(format!("{file_name} too small: {} bytes", png.len()));
    }

    // Decode round-trip: dims + non-blank + theme luminance band.
    let decoded = decode_png(&png).ok_or_else(|| err("decode_png failed"))?;
    if decoded.width != SHOT_W as u32 || decoded.height != SHOT_H as u32 {
        return Err(err("decoded dims mismatch"));
    }
    let colors = distinct_rgb(&decoded.pixels);
    if colors.len() < MIN_DISTINCT_RGB {
        return Err(format!("too few colors: {}", colors.len()));
    }
    let mean = mean_luminance(&decoded.pixels);
    if app_dark != expect_dark {
        return Err(format!(
            "dark_mode desync: app={app_dark} theme_dark={expect_dark}"
        ));
    }
    if expect_dark {
        if mean > MEAN_DARK_MAX {
            return Err(format!("dark shot too bright: mean={mean:.1}"));
        }
    } else if mean < MEAN_LIGHT_MIN {
        return Err(format!("light shot too dark: mean={mean:.1}"));
    }

    let out_dir = root.join("checks/gui-realshot");
    std::fs::create_dir_all(&out_dir).map_err(|why| format!("mkdir realshot: {why}"))?;
    std::fs::write(out_dir.join(file_name), &png)
        .map_err(|why| format!("write {file_name}: {why}"))?;
    Ok(format!(
        "real-egui({file_name}): rows={rows} bytes={} colors={} mean={mean:.1}",
        png.len(),
        colors.len()
    ))
}
