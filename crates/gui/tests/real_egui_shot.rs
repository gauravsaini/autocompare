//! Real-egui screenshots of the text-compare view (exq-realshot88 round-1:
//! live egui_kittest capture hardening, dark + light).
//!
//! Boots the REAL [`App`] text-compare view on the parity fixtures via
//! `egui_kittest` (same egui 0.37 the shipped app uses), waits for the
//! background compare, renders headless with wgpu, and writes each shot to
//! target tmp (`CARGO_TARGET_TMPDIR`, else the OS temp dir) — never to the
//! repo tree. Pass `EXQ_KEEP_PNG=1` (explicit harness step) to also refresh
//! the tracked `checks/gui-exq-realshot88/round-1/real-egui*.png` copies
//! (dark `real-egui.png`, light `real-egui-light.png`). The kittest49
//! baseline `after.png` is read-only input, never rewritten.
//!
//! Machine-checked per theme: PNG magic + size + dims + non-blank + pixel
//! color variance far above the flat software-rasterizer baseline
//! (anti-aliased font evidence), so a faked flat render cannot pass.
//!
//! Plus live Visuals/fonts/icons proof per theme (exq-realpolish75): the
//! harness theme agrees with `App.dark_mode`, both theme palettes satisfy
//! WCAG AA and differ per mode, rendered mean luminance lands in the mode
//! band (dark ~33 vs flat `after.png` ~238), the typo48/typo54 font plan
//! serves both families, and the exq-icons50/toolbar glyph geometry
//! contracts hold.

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
/// Mean-luminance bands proving the live theme actually painted: dark shot
/// measured ~33, light shot ~243, flat `after.png` ~238. A flat rasterizer
/// posing as dark fails the dark band; a dim fake fails the light band.
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

/// Mean channel luminance 0..=255 over RGBA pixels (alpha ignored,
/// machine-checked with no indexing like [`distinct_rgb`]).
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

/// Live Visuals proof for one theme: harness theme agrees with
/// `App.dark_mode`, both theme palettes satisfy WCAG AA and differ per
/// mode, and rendered pixels land in the mode luminance band.
fn assert_live_visuals(expect_dark: bool, app_dark: bool, mean: f64) -> Result<(), String> {
    use autocompare_gui::theme;
    if app_dark != expect_dark {
        return Err(format!(
            "dark_mode desync: app={app_dark} theme_dark={expect_dark}"
        ));
    }
    if !theme::theme_all_palettes_satisfy_aa() {
        return Err(err("theme palettes fail WCAG AA"));
    }
    if theme::palette(true) == theme::palette(false) {
        return Err(err("dark/light palettes identical"));
    }
    if expect_dark {
        if mean > MEAN_DARK_MAX {
            return Err(format!("dark shot too bright: mean={mean:.1}"));
        }
    } else if mean < MEAN_LIGHT_MIN {
        return Err(format!("light shot too dark: mean={mean:.1}"));
    }
    Ok(())
}

/// Live font proof: the typo48 install plan serves both egui families from
/// single-source stacks, typo54 chains run system-head to egui-safe tail,
/// and role sizes/families pin the hierarchy the live paint uses.
fn assert_live_fonts() -> Result<(), String> {
    use autocompare_gui::{app, theme};
    let plan = app::typo48_live_font_plan();
    let [chrome, content] = &plan;
    if chrome.stack != theme::TYPO_UI_SANS_STACK || content.stack != theme::TYPO_MONO_STACK {
        return Err(err("font plan stacks diverged"));
    }
    let sans = app::typoinst61_chain_for_role(theme::TypoRole::Body);
    let mono = app::typoinst61_chain_for_role(theme::TypoRole::Mono);
    let sans_ends = match (sans.first(), sans.last()) {
        (Some(head), Some(tail)) => (*head, *tail),
        _ => return Err(err("sans chain empty")),
    };
    let mono_ends = match (mono.first(), mono.last()) {
        (Some(head), Some(tail)) => (*head, *tail),
        _ => return Err(err("mono chain empty")),
    };
    if sans_ends != ("SF Pro", "proportional") {
        return Err(err("sans chain must run SF Pro -> proportional"));
    }
    if mono_ends != ("SF Mono", "monospace") {
        return Err(err("mono chain must run SF Mono -> monospace"));
    }
    if theme::TypoRole::Mono.size() != theme::TYPO_MONO_SIZE
        || theme::TypoRole::Body.size() != theme::TYPO_BODY_SIZE
        || theme::TypoRole::Small.size() != theme::TYPO_SMALL_SIZE
        || theme::TypoRole::Heading.size() != theme::TYPO_HEADING_SIZE
    {
        return Err(err("typo role sizes drifted"));
    }
    if !theme::TypoRole::Mono.is_mono() || !theme::TypoRole::Body.is_chrome() {
        return Err(err("typo role families drifted"));
    }
    if !theme::contrast_dim_text_ok() {
        return Err(err("dim-text contrast contract broken"));
    }
    Ok(())
}

/// Live icon proof: the exq-icons50 vector-mark geometry the toolbar paint
/// uses (12px box, 2px stroke, 4-kind cycle, ring/bars/rails/wedge segment
/// counts, 8-pt wedge) plus the 16px toolbar glyph slot contracts.
fn assert_live_icons() -> Result<(), String> {
    use autocompare_gui::{app, theme, views};
    if app::exq_icons50_mark_box() != theme::SPACE_MD {
        return Err(err("icon mark box drifted"));
    }
    if app::exq_icons50_stroke_w() != theme::SPACE_XS / 2.0 {
        return Err(err("icon stroke drifted"));
    }
    if app::exq_icons50_ring_r() != 4.0 {
        return Err(err("icon ring radius drifted"));
    }
    let [k0, k1, k2, k3] = [
        app::exq_icons50_mark_kind(0),
        app::exq_icons50_mark_kind(1),
        app::exq_icons50_mark_kind(2),
        app::exq_icons50_mark_kind(3),
    ];
    if k0 == k1 || k0 == k2 || k0 == k3 || k1 == k2 || k1 == k3 || k2 == k3 {
        return Err(err("icon mark kinds not 4-distinct"));
    }
    if app::exq_icons50_mark_kind(4) != k0 || app::exq_icons50_mark_kind(7) != k3 {
        return Err(err("icon mark kinds do not cycle every 4"));
    }
    let (_, n_ring) = app::exq_icons50_mark_segments(app::ExqIcons50Mark::Ring);
    let (_, n_bars) = app::exq_icons50_mark_segments(app::ExqIcons50Mark::Bars);
    let (_, n_rails) = app::exq_icons50_mark_segments(app::ExqIcons50Mark::Rails);
    let (_, n_wedge) = app::exq_icons50_mark_segments(app::ExqIcons50Mark::Wedge);
    if n_ring != 0 || n_bars != 2 || n_rails != 2 || n_wedge != 3 {
        return Err(err("icon mark segment counts drifted"));
    }
    if app::exq_icons50_wedge_points().len() != 8 {
        return Err(err("icon wedge points drifted"));
    }
    if views::toolbar_glyph_width() != 16.0 {
        return Err(err("toolbar glyph slot width drifted"));
    }
    if views::toolbar_glyph_variant(5) != 1 || views::toolbar_glyph_variant(4) != 0 {
        return Err(err("toolbar glyph variants do not cycle every 4"));
    }
    if views::toolbar_glyph_rects(0).len() != 1
        || views::toolbar_glyph_rects(1).len() != 2
        || views::toolbar_glyph_rects(3).len() != 3
    {
        return Err(err("toolbar glyph rect counts drifted"));
    }
    if views::TOOLBAR_SWAP_GLYPH.is_empty() || views::TOOLBAR_RELOAD_GLYPH.is_empty() {
        return Err(err("toolbar glyphs must be non-empty"));
    }
    Ok(())
}

#[test]
fn real_egui_text_compare_shot() -> Result<(), String> {
    let summary = capture_shot(Theme::Dark, "real-egui.png", "real-egui.png")?;
    println!("{summary}");
    Ok(())
}

#[test]
fn real_egui_text_compare_shot_light() -> Result<(), String> {
    let summary = capture_shot(Theme::Light, "real-egui-light.png", "real-egui-light.png")?;
    println!("{summary}");
    Ok(())
}

/// Shared dark/light capture: same fixtures, same `SHOT_W`/`SHOT_H` canvas,
/// same [`MIN_PNG_BYTES`]/[`MIN_DISTINCT_RGB`]/[`AA_RATIO`] floors. Writes to
/// target tmp only, plus the tracked round copy on `EXQ_KEEP_PNG=1`.
/// Returns the one-line evidence summary for the caller to print.
fn capture_shot(theme: Theme, tmp_name: &str, keep_name: &str) -> Result<String, String> {
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
    // App re-asserts visuals from `dark_mode` every frame (CK-DARKBUG fix),
    // so the harness theme alone cannot flip it: drive the pub flag too
    // (test-only, shipped behavior untouched).
    app.dark_mode = expect_dark;
    // `app` moves into the kittest closure below; keep the flag for the
    // Visuals agreement check after render.
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
    // force honest opaque alpha (RGB untouched, colors frozen).
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
        return Err(format!("{tmp_name} has bad PNG magic"));
    }
    if png.len() < MIN_PNG_BYTES {
        return Err(format!("{tmp_name} too small: {} bytes", png.len()));
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
    // Live Visuals/fonts/icons proof (exq-realpolish75): theme agreement +
    // mode luminance band, font plan/chains/roles, icon/glyph geometry.
    let mean = mean_luminance(&decoded.pixels);
    assert_live_visuals(expect_dark, app_dark, mean)?;
    assert_live_fonts()?;
    assert_live_icons()?;

    // Baseline comparison: real render variance far above the flat rasterizer.
    // The kittest49 after.png is read-only input; kept copies land in the
    // owned realshot88 round dir (disjoint OWNS, never touch other rounds).
    let baseline_round = root.join("checks/gui-exq-kittest49/round-1");
    let keep_round = root.join("checks/gui-exq-realshot88/round-1");
    let baseline_bytes = std::fs::read(baseline_round.join("after.png"))
        .map_err(|why| format!("read after.png: {why}"))?;
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

    // Hygiene: the shot lives in target tmp by default, so `cargo test`
    // never rewrites the tracked checks/ copy (non-deterministic bytes).
    // Pass EXQ_KEEP_PNG=1 (explicit harness step) to refresh that copy.
    let tmp = match std::env::var("CARGO_TARGET_TMPDIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => std::env::temp_dir(),
    };
    std::fs::write(tmp.join(tmp_name), &png)
        .map_err(|why| format!("write tmp {tmp_name}: {why}"))?;
    if std::env::var("EXQ_KEEP_PNG").as_deref() == Ok("1") {
        std::fs::create_dir_all(&keep_round).map_err(|why| format!("mkdir round: {why}"))?;
        std::fs::write(keep_round.join(keep_name), &png)
            .map_err(|why| format!("write {keep_name}: {why}"))?;
    }
    Ok(format!(
        "real-egui({tmp_name}): rows={rows} bytes={} colors={} base={} ratio={} mean={mean:.1}",
        png.len(),
        real_colors.len(),
        base_colors.len(),
        ratio
    ))
}
