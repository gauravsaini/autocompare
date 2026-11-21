//! Real-egui screenshot of the text-compare view (exq-kittest49 round-1).
//!
//! Boots the REAL [`App`] text-compare view on the parity fixtures via
//! `egui_kittest` (same egui 0.37 the shipped app uses), waits for the
//! background compare, renders headless with wgpu, and writes
//! `checks/gui-exq-kittest49/round-1/real-egui.png`.
//!
//! Machine-checked: PNG magic + size + dims + non-blank + pixel color
//! variance far above the flat software-rasterizer baseline (anti-aliased
//! font evidence), so a faked flat render cannot pass.

#![forbid(unsafe_code)]

use autocompare_diff::{decode_png, encode_png};
use autocompare_gui::app::{App, OpenRequest, Tab};
use eframe::egui::{Theme, Vec2};
use egui_kittest::Harness;
use std::collections::HashSet;
use std::path::PathBuf;

/// Kittest window size; matches the software rasterizer canvas (SNAP_W) so
/// pxdiff can compare real-egui.png against after.png pixel-for-pixel.
const SHOT_W: f32 = 960.0;
/// Kittest window height; matches the software rasterizer canvas (SNAP_H).
const SHOT_H: f32 = 600.0;
/// Minimum PNG bytes (mirrors the exquisite harness floor).
const MIN_PNG_BYTES: usize = 8192;
/// Distinct-RGB floor proving anti-aliased font rasterization.
const MIN_DISTINCT_RGB: usize = 500;
/// Real render must exceed the flat baseline by this factor.
const AA_RATIO: usize = 10;

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
    loop {
        match (iter.next(), iter.next(), iter.next(), iter.next()) {
            (Some(r), Some(g), Some(b), Some(_)) => {
                seen.insert((*r, *g, *b));
            }
            _ => break,
        }
    }
    seen
}

#[test]
fn real_egui_text_compare_shot() -> Result<(), String> {
    let root = workspace_root();
    let left = root.join("tools/parity-harness/fixtures/t_left.txt");
    let right = root.join("tools/parity-harness/fixtures/t_right.txt");
    for fixture in [&left, &right] {
        if !fixture.is_file() {
            return Err(format!("missing fixture {}", fixture.display()));
        }
    }

    let mut app = App::default();
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
        .with_theme(Theme::Dark)
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
    let frame = autocompare_diff::RgbaImage {
        width: shot.width(),
        height: shot.height(),
        pixels: shot.into_raw(),
    };
    let png = encode_png(&frame).ok_or_else(|| err("encode_png failed"))?;

    // PNG magic (machine-checked, no indexing).
    const MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if !png.starts_with(&MAGIC) {
        return Err(err("real-egui.png has bad PNG magic"));
    }
    if png.len() < MIN_PNG_BYTES {
        return Err(format!("real-egui.png too small: {} bytes", png.len()));
    }

    // Decode round-trip: dims + non-blank.
    let decoded = decode_png(&png).ok_or_else(|| err("decode_png failed"))?;
    if decoded.width != SHOT_W as u32 || decoded.height != SHOT_H as u32 {
        return Err(err("decoded dims mismatch"));
    }
    let real_colors = distinct_rgb(&decoded.pixels);
    if real_colors.len() < MIN_DISTINCT_RGB {
        return Err(format!("too few colors: {}", real_colors.len()));
    }

    // Baseline comparison: real render variance far above the flat rasterizer.
    let round = root.join("checks/gui-exq-kittest49/round-1");
    let baseline_bytes =
        std::fs::read(round.join("after.png")).map_err(|why| format!("read after.png: {why}"))?;
    let baseline = decode_png(&baseline_bytes).ok_or_else(|| err("decode after.png failed"))?;
    let base_colors = distinct_rgb(&baseline.pixels);
    if base_colors.is_empty() {
        return Err(err("baseline has no colors"));
    }
    let ratio = real_colors.len() / base_colors.len();
    if real_colors.len() < MIN_DISTINCT_RGB || ratio < AA_RATIO {
        return Err(format!(
            "no AA evidence: real={} base={} ratio={}",
            real_colors.len(),
            base_colors.len(),
            ratio
        ));
    }

    std::fs::create_dir_all(&round).map_err(|why| format!("mkdir round: {why}"))?;
    std::fs::write(round.join("real-egui.png"), &png)
        .map_err(|why| format!("write real-egui.png: {why}"))?;
    println!(
        "real-egui: rows={rows} bytes={} colors={} base={} ratio={}",
        png.len(),
        real_colors.len(),
        base_colors.len(),
        ratio
    );
    Ok(())
}
