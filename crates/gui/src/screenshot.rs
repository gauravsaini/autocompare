//! Honest headless screenshot pipeline: the real app, rendered for real.
//!
//! Boots the shipped [`App`](crate::app::App) under `egui_kittest` (the same
//! eframe/egui 0.37 stack the desktop binary uses), seeds the route's fixture
//! pair, waits for the background compare to land, and saves the
//! wgpu-rendered frame as PNG. Output carries anti-aliased fonts, real
//! widgets, and live theme Visuals.
//!
//! What this module replaced: a hand-written 5x7 ASCII bitmap software
//! rasterizer (`fill_rect` + `draw_glyph`, square "traffic lights",
//! hardcoded dark bars). That rasterizer could never prove what the live
//! `views` paint; every byte it emitted was a drawing of a guess. The only
//! honest screenshot is the app itself, so this module renders the app
//! itself.
//!
//! CLI contract (unchanged, see `main.rs`): `--screenshot <png>` writes one
//! PNG and exits 0; `--screenshot-mode <dark|light>` picks the theme
//! (default dark for byte-stability of existing harness floors). The output
//! file name still selects the scene (lowercase match): `narrow`, `accent`,
//! `allsame`, `hex`, `table` (+`find`), `merge-take`, `merge`, `three-way` /
//! `threeway`, `sync`, `folder` (+`empty`), `home`, `picture`, `wrap` —
//! anything else renders the default Text scene. Every scene is a real tab
//! of the real app on real parity fixtures, never a mock bitmap.

#![forbid(unsafe_code)]

use crate::app::{App, OpenRequest, Tab};
use autocompare_diff::{RgbaImage, decode_png, encode_png};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Headless canvas width (px): the fixture canvas contract stays 960x600 so
/// `--screenshot` output dims never surprise existing harness consumers.
pub const SNAP_W: u32 = 960;
/// Headless canvas height (px): see [`SNAP_W`].
pub const SNAP_H: u32 = 600;
/// Narrow-viewport route width (px): the live minimum window width, so the
/// `narrow` scene proves the real responsive layout instead of a guess.
pub const NARROW_W: u32 = crate::theme::VIEWPORT_MIN_W;
/// Minimum honest PNG size (bytes): a real 960x600 egui frame compresses far
/// above this; a blank or truncated write never passes.
pub const MIN_PNG_BYTES: usize = 8192;
/// Distinct-RGB floor proving anti-aliased rasterization: the hand-drawn
/// bitmap backend topped out near a dozen flat tones, while a real egui
/// frame (font hinting + theme washes) measures in the hundreds.
pub const MIN_DISTINCT_RGB: usize = 500;
/// Dark-mode mean-luminance ceiling: the live dark Visuals render far below
/// this; a light frame mislabeled dark fails here.
pub const MEAN_DARK_MAX: f64 = 120.0;
/// Light-mode mean-luminance floor: the live light Visuals render far above
/// this; a dark frame mislabeled light fails here.
pub const MEAN_LIGHT_MIN: f64 = 160.0;

/// Fixture render mode: headless proof keeps the dark/light switch.
/// Default dark so existing harness floors keep passing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FixtureMode {
    /// Dark render (existing default).
    #[default]
    Dark,
    /// Light render.
    Light,
}

/// Which scene to render: one real app tab on one fixture pair.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ShotPlan {
    /// Real app tab backing this scene.
    tab: Tab,
    /// Left input path (empty when the scene needs none).
    left: String,
    /// Right input path (empty when the scene needs none).
    right: String,
    /// Base input path (three-way merge scene only).
    base: String,
    /// Capture width in px.
    width: u32,
    /// Capture height in px.
    height: u32,
    /// Word-wrap on for the wrap scene.
    word_wrap: bool,
    /// Table-find needle for the find scene (empty = no active find).
    table_needle: String,
}

/// Absolute path of one parity fixture by file name.
fn fixture_path(name: &str) -> String {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.pop();
    dir.push("tools");
    dir.push("parity-harness");
    dir.push("fixtures");
    dir.push(name);
    dir.to_string_lossy().into_owned()
}

/// Absolute path of one parity fixture directory by name (`folder`, `sync`).
fn fixture_dir(name: &str, side: &str) -> String {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.pop();
    dir.push("tools");
    dir.push("parity-harness");
    dir.push("fixtures");
    dir.push(name);
    if !side.is_empty() {
        dir.push(side);
    }
    dir.to_string_lossy().into_owned()
}

/// Pure route mapping: output file name (lowercase match) to [`ShotPlan`].
/// Same scene keys as the retired bitmap router, so existing harness
/// consumers keep getting a sensible scene per name — now rendered by the
/// real tab instead of drawn by hand.
fn shot_plan_for(out_path: &Path) -> ShotPlan {
    let lower = match out_path.file_name().and_then(|name| name.to_str()) {
        Some(name) => name.to_lowercase(),
        None => String::new(),
    };
    let text = ShotPlan {
        tab: Tab::Text,
        left: fixture_path("t_left.txt"),
        right: fixture_path("t_right.txt"),
        base: String::new(),
        width: SNAP_W,
        height: SNAP_H,
        word_wrap: false,
        table_needle: String::new(),
    };
    if lower.contains("narrow") {
        return ShotPlan {
            width: NARROW_W,
            ..text
        };
    }
    if lower.contains("accent") {
        return text;
    }
    if lower.contains("allsame") {
        let same = fixture_path("t_same.txt");
        return ShotPlan {
            left: same.clone(),
            right: same,
            ..text
        };
    }
    if lower.contains("hex") {
        return ShotPlan {
            tab: Tab::Hex,
            left: fixture_path("h_left.bin"),
            right: fixture_path("h_right.bin"),
            ..text
        };
    }
    if lower.contains("table") {
        return ShotPlan {
            tab: Tab::Table,
            left: fixture_path("tab_left.csv"),
            right: fixture_path("tab_right.csv"),
            table_needle: if lower.contains("find") {
                String::from("amy")
            } else {
                String::new()
            },
            ..text
        };
    }
    if lower.contains("three-way") || lower.contains("threeway") {
        return ShotPlan {
            tab: Tab::FolderMerge,
            left: fixture_dir("fmerge-left", ""),
            right: fixture_dir("fmerge-right", ""),
            base: fixture_dir("fmerge-base", ""),
            ..text
        };
    }
    if lower.contains("merge") {
        return ShotPlan {
            tab: Tab::Merge,
            left: fixture_path("m_left.txt"),
            right: fixture_path("m_right.txt"),
            base: fixture_path("m_base.txt"),
            ..text
        };
    }
    if lower.contains("sync") {
        return ShotPlan {
            tab: Tab::Sync,
            left: fixture_dir("sync", "left"),
            right: fixture_dir("sync", "right"),
            ..text
        };
    }
    if lower.contains("folder") {
        if lower.contains("empty") {
            return ShotPlan {
                tab: Tab::Folder,
                left: empty_scene_dir("left"),
                right: empty_scene_dir("right"),
                ..text
            };
        }
        return ShotPlan {
            tab: Tab::Folder,
            left: fixture_dir("folder", "left"),
            right: fixture_dir("folder", "right"),
            ..text
        };
    }
    if lower.contains("home") {
        return ShotPlan {
            tab: Tab::Home,
            left: String::new(),
            right: String::new(),
            ..text
        };
    }
    if lower.contains("picture") {
        return ShotPlan {
            tab: Tab::Picture,
            left: fixture_path("pic_left.png"),
            right: fixture_path("pic_right.png"),
            ..text
        };
    }
    if lower.contains("wrap") {
        return ShotPlan {
            word_wrap: true,
            ..text
        };
    }
    text
}

/// Scratch directory for the `folder-empty` scene: two genuinely empty
/// directories under the process temp dir (headless-first: never the repo,
/// never HOME). Created on demand; the compare then proves the real empty
/// state instead of a hand-drawn one.
fn empty_scene_dir(side: &str) -> String {
    let mut dir = std::env::temp_dir();
    dir.push("autocompare-gui-shot-empty");
    dir.push(side);
    let _ = std::fs::create_dir_all(&dir);
    dir.to_string_lossy().into_owned()
}

/// Seeds one [`App`] from a [`ShotPlan`]: theme flag, fixture paths, tab,
/// and the background compare. Missing fixture files never fake a scene —
/// the tab opens unseeded and the app paints its real missing-input state.
fn seed_app(plan: &ShotPlan, dark: bool) -> App {
    let mut app = App::default();
    app.dark_mode = dark;
    if plan.word_wrap {
        app.set_word_wrap(true);
    }
    if !plan.table_needle.is_empty() {
        app.table_find_needle = plan.table_needle.clone();
    }
    match plan.tab {
        Tab::Home => {
            app.set_tab(Tab::Home);
        }
        Tab::Merge => {
            if !plan.base.is_empty() && Path::new(&plan.base).is_file() {
                let left_ok = !plan.left.is_empty() && Path::new(&plan.left).is_file();
                let right_ok = !plan.right.is_empty() && Path::new(&plan.right).is_file();
                if left_ok && right_ok {
                    app.set_merge_paths(plan.base.clone(), plan.left.clone(), plan.right.clone());
                }
            }
            app.set_tab(Tab::Merge);
            app.run_active_tab();
        }
        tab => {
            let seeded = !plan.left.is_empty()
                && !plan.right.is_empty()
                && Path::new(&plan.left).exists()
                && Path::new(&plan.right).exists()
                && app.apply_open_request(&OpenRequest {
                    left: plan.left.clone(),
                    right: plan.right.clone(),
                    tab,
                });
            if !seeded {
                app.set_tab(tab);
                app.run_active_tab();
            }
        }
    }
    app
}

/// The tab's compare model has landed when its headline field is `Some`.
/// Home has no model; it is ready as soon as it paints.
fn model_ready(app: &App, tab: Tab) -> bool {
    match tab {
        Tab::Home => true,
        Tab::Text => app.text_model.is_some(),
        Tab::Folder => app.folder_model.is_some(),
        Tab::Hex => app.hex_model.is_some(),
        Tab::Merge => app.merge_model.is_some(),
        Tab::Table => app.table_model.is_some(),
        Tab::Sync => app.sync_model.is_some(),
        Tab::Picture => app.picture_model.is_some(),
        Tab::FolderMerge => app.folder_merge_model.is_some(),
        Tab::Audio => app.audio_model.is_some(),
    }
}

/// Renders one seeded scene through the real egui stack and returns the
/// opaque RGBA frame. Waits (bounded) for the background compare so data
/// rows — not an empty window — reach the frame; on timeout the app's real
/// loading/error state is captured instead of failing, because the CLI
/// contract is to always write what the app honestly shows.
fn render_plan(plan: &ShotPlan, dark: bool) -> Result<RgbaImage, String> {
    use eframe::egui::{Theme, Vec2};
    use egui_kittest::Harness;

    let width = plan.width.max(1);
    let height = plan.height.max(1);
    let app = seed_app(plan, dark);
    let theme = if dark { Theme::Dark } else { Theme::Light };
    let mut harness = Harness::builder()
        .with_size(Vec2::new(width as f32, height as f32))
        .with_accessibility_check(false)
        .allow_missing_glyphs()
        .with_max_steps(60)
        .with_step_dt(0.01)
        .with_theme(theme)
        .wgpu()
        .build_eframe(|_cc| app);

    let mut waited_ms = 0u64;
    while waited_ms < 10_000 {
        harness.run_steps(2);
        if model_ready(harness.state(), plan.tab) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        waited_ms = waited_ms.saturating_add(20);
    }
    // The live view repaints continuously, so a bounded step burst — not a
    // quiescent frame — is the honest settle for a live app.
    harness.run_steps(8);

    let shot = harness.render().map_err(|why| format!("render: {why}"))?;
    if shot.width() != width || shot.height() != height {
        return Err(format!("bad shot size {}x{}", shot.width(), shot.height()));
    }
    // Wgpu readback can leave alpha 0 on a fully-covered opaque window;
    // force honest opaque alpha (RGB untouched).
    Ok(RgbaImage {
        width: shot.width(),
        height: shot.height(),
        pixels: shot.into_raw(),
    }
    .opaque())
}

/// Observed facts about one PNG file, decoded — never trusted from headers
/// alone except via the decoder round-trip.
#[derive(Debug, Clone, PartialEq)]
pub struct PngStats {
    /// Decoded width in px.
    pub width: u32,
    /// Decoded height in px.
    pub height: u32,
    /// Encoded file size in bytes.
    pub bytes: usize,
    /// Distinct RGB triples (alpha ignored).
    pub distinct_rgb: usize,
    /// Mean channel luminance 0.0..=255.0 (alpha ignored).
    pub mean_luminance: f64,
}

/// Decodes one PNG and reports its [`PngStats`]. Errors on bad magic or an
/// undecodable body — no aborts, no direct indexing.
pub fn inspect_png(png: &[u8]) -> Result<PngStats, String> {
    const MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if !png.starts_with(&MAGIC) {
        return Err(String::from("bad PNG magic"));
    }
    let decoded = decode_png(png).ok_or_else(|| String::from("PNG decode failed"))?;
    let mut distinct = HashSet::new();
    let mut sum = 0u64;
    let mut count = 0u64;
    let (quads, _) = decoded.pixels.as_chunks::<4>();
    for quad in quads.iter() {
        let (r, g, b) = match (quad.first(), quad.get(1), quad.get(2)) {
            (Some(r), Some(g), Some(b)) => (*r, *g, *b),
            _ => continue,
        };
        distinct.insert((r, g, b));
        sum = sum
            .saturating_add(u64::from(r))
            .saturating_add(u64::from(g))
            .saturating_add(u64::from(b));
        count = count.saturating_add(1);
    }
    if count == 0 {
        return Err(String::from("PNG has no pixels"));
    }
    Ok(PngStats {
        width: decoded.width,
        height: decoded.height,
        bytes: png.len(),
        distinct_rgb: distinct.len(),
        mean_luminance: sum as f64 / count as f64 / 3.0,
    })
}

/// Full honesty gate for one screenshot file: valid PNG, big enough to be a
/// real frame, varied enough to be anti-aliased (not a flat bitmap), and in
/// the luminance band of the theme it claims.
pub fn check_screenshot_png(png: &[u8], want_dark: bool) -> Result<PngStats, String> {
    let stats = inspect_png(png)?;
    if stats.bytes < MIN_PNG_BYTES {
        return Err(format!("screenshot too small: {} bytes", stats.bytes));
    }
    if stats.distinct_rgb < MIN_DISTINCT_RGB {
        return Err(format!(
            "screenshot looks flat ({} distinct RGB): not a real render",
            stats.distinct_rgb
        ));
    }
    if want_dark {
        if stats.mean_luminance > MEAN_DARK_MAX {
            return Err(format!(
                "dark screenshot too bright: mean={:.1}",
                stats.mean_luminance
            ));
        }
    } else if stats.mean_luminance < MEAN_LIGHT_MIN {
        return Err(format!(
            "light screenshot too dark: mean={:.1}",
            stats.mean_luminance
        ));
    }
    Ok(stats)
}

/// Renders the Text comparison session fixture to a PNG at the given path.
///
/// The scene is the real Text tab on the real parity fixtures, captured
/// through the live egui stack. Default mode is dark; use
/// [`render_fixture_png_with_mode`] for the light scene. File-name scene
/// keys are documented on the module root.
pub fn render_fixture_png(out_path: &Path) -> Result<(), String> {
    render_fixture_png_with_mode(out_path, FixtureMode::Dark)
}

/// Mode-aware screenshot entry: same scene keys as [`render_fixture_png`],
/// rendered under the requested theme.
pub fn render_fixture_png_with_mode(out_path: &Path, mode: FixtureMode) -> Result<(), String> {
    let plan = shot_plan_for(out_path);
    let dark = matches!(mode, FixtureMode::Dark);
    let frame = render_plan(&plan, dark)?;
    let png_bytes = encode_png(&frame).ok_or_else(|| String::from("PNG encode failed"))?;
    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|why| format!("mkdir: {why}"))?;
    }
    std::fs::write(out_path, png_bytes).map_err(|why| format!("write: {why}"))?;
    Ok(())
}

/// CP-LIGHTFLAG: `--screenshot-mode <dark|light>` CLI value. Default dark
/// so the flag-free render keeps passing existing harness floors. Unknown
/// or missing value is `Err` (caller exits 2, opener precedent).
/// Pure; headless-tested, never opens the GUI.
pub fn screenshot_mode_from_args(args: &[String]) -> Result<FixtureMode, String> {
    let mut mode = FixtureMode::Dark;
    let mut rest: &[String] = match args.get(1..) {
        Some(rest) => rest,
        None => return Ok(mode),
    };
    while let Some((head, tail)) = rest.split_first() {
        if head.as_str() == "--screenshot-mode" {
            match tail.split_first() {
                Some((value, after)) if !value.starts_with("--") => {
                    match value.as_str() {
                        "dark" => mode = FixtureMode::Dark,
                        "light" => mode = FixtureMode::Light,
                        _ => {
                            return Err(format!("unknown --screenshot-mode '{value}': dark/light"));
                        }
                    }
                    rest = after;
                }
                _ => {
                    return Err(String::from(
                        "--screenshot-mode needs a value: --screenshot-mode dark|light",
                    ));
                }
            }
        } else {
            rest = tail;
        }
    }
    Ok(mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_of(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_string()).collect()
    }

    #[test]
    fn fixture_mode_defaults_dark() {
        assert_eq!(FixtureMode::default(), FixtureMode::Dark);
    }

    #[test]
    fn screenshot_mode_flag_defaults_dark_without_flag() {
        let prog = String::from("autocompare-gui");
        assert_eq!(
            screenshot_mode_from_args(std::slice::from_ref(&prog)),
            Ok(FixtureMode::Dark)
        );
        assert_eq!(
            screenshot_mode_from_args(&args_of(&["autocompare-gui", "--screenshot", "o.png"])),
            Ok(FixtureMode::Dark)
        );
    }

    #[test]
    fn screenshot_mode_flag_parses_dark_and_light() {
        assert_eq!(
            screenshot_mode_from_args(&args_of(&[
                "autocompare-gui",
                "--screenshot",
                "o.png",
                "--screenshot-mode",
                "dark"
            ])),
            Ok(FixtureMode::Dark)
        );
        assert_eq!(
            screenshot_mode_from_args(&args_of(&[
                "autocompare-gui",
                "--screenshot",
                "o.png",
                "--screenshot-mode",
                "light"
            ])),
            Ok(FixtureMode::Light)
        );
    }

    #[test]
    fn screenshot_mode_flag_rejects_unknown_and_missing_values() {
        let prog = String::from("autocompare-gui");
        assert!(
            screenshot_mode_from_args(&[
                prog.clone(),
                String::from("--screenshot-mode"),
                String::from("sepia"),
            ])
            .is_err()
        );
        assert!(screenshot_mode_from_args(&[prog, String::from("--screenshot-mode")]).is_err());
    }

    #[test]
    fn shot_plan_routes_filenames_to_real_tabs() {
        let text = shot_plan_for(Path::new("shot.png"));
        assert_eq!(text.tab, Tab::Text);
        assert_eq!((text.width, text.height), (SNAP_W, SNAP_H));
        assert!(text.left.contains("t_left.txt"));

        let narrow = shot_plan_for(Path::new("shot-narrow.png"));
        assert_eq!(narrow.tab, Tab::Text);
        assert_eq!(narrow.width, NARROW_W);

        let allsame = shot_plan_for(Path::new("shot-allsame.png"));
        assert_eq!(allsame.left, allsame.right);
        assert!(allsame.left.contains("t_same.txt"));

        assert_eq!(shot_plan_for(Path::new("shot-hex.png")).tab, Tab::Hex);
        assert_eq!(shot_plan_for(Path::new("shot-table.png")).tab, Tab::Table);
        let find = shot_plan_for(Path::new("shot-table-find.png"));
        assert_eq!(find.tab, Tab::Table);
        assert!(!find.table_needle.is_empty());

        assert_eq!(shot_plan_for(Path::new("shot-merge.png")).tab, Tab::Merge);
        assert_eq!(
            shot_plan_for(Path::new("shot-merge-take.png")).tab,
            Tab::Merge
        );
        assert_eq!(
            shot_plan_for(Path::new("shot-three-way.png")).tab,
            Tab::FolderMerge
        );
        assert_eq!(shot_plan_for(Path::new("shot-sync.png")).tab, Tab::Sync);
        assert_eq!(shot_plan_for(Path::new("shot-folder.png")).tab, Tab::Folder);
        assert_eq!(shot_plan_for(Path::new("shot-home.png")).tab, Tab::Home);
        assert_eq!(
            shot_plan_for(Path::new("shot-picture.png")).tab,
            Tab::Picture
        );
        let wrap = shot_plan_for(Path::new("shot-wrap.png"));
        assert!(wrap.word_wrap);
        assert_eq!(wrap.tab, Tab::Text);
    }

    #[test]
    fn inspect_png_reports_synthetic_frame_stats() -> Result<(), String> {
        let frame = RgbaImage {
            width: 4,
            height: 2,
            pixels: vec![
                10, 20, 30, 255, 200, 210, 220, 255, 10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90,
                255, 200, 210, 220, 255, 100, 110, 120, 255, 130, 140, 150, 255,
            ],
        };
        let png = encode_png(&frame).ok_or_else(|| String::from("encode failed"))?;
        assert!(!png.is_empty());
        let stats = inspect_png(&png)?;
        assert_eq!((stats.width, stats.height), (4, 2));
        assert_eq!(stats.bytes, png.len());
        // (10,20,30) and (200,210,220) each repeat, so 8 pixels
        // collapse to 6 distinct triples.
        assert_eq!(stats.distinct_rgb, 6);
        assert!(stats.mean_luminance > 0.0);
        Ok(())
    }

    #[test]
    fn inspect_png_rejects_bad_magic_and_truncated_body() {
        assert!(inspect_png(&[]).is_err());
        assert!(inspect_png(b"definitely not a png").is_err());
        let mut png = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 1, 2, 3];
        assert!(inspect_png(&png).is_err());
        png.clear();
        assert!(inspect_png(&png).is_err());
    }

    #[test]
    fn check_screenshot_png_enforces_size_and_flatness_floors() -> Result<(), String> {
        // Tiny real PNG: valid bytes, but far below the honest-frame floor.
        let tiny = RgbaImage {
            width: 4,
            height: 4,
            pixels: vec![99u8; 4 * 4 * 4],
        };
        let png = encode_png(&tiny).ok_or_else(|| String::from("encode failed"))?;
        assert!(inspect_png(&png).is_ok());
        assert!(check_screenshot_png(&png, true).is_err());

        // Perfectly flat frame: a flat field compresses under the byte
        // floor and carries one tone, so whichever gate bites, the verdict
        // refuses it as a render.
        let flat = RgbaImage {
            width: 200,
            height: 200,
            pixels: vec![200u8; 200 * 200 * 4],
        };
        let png = encode_png(&flat).ok_or_else(|| String::from("encode failed"))?;
        assert!(check_screenshot_png(&png, true).is_err());
        Ok(())
    }

    /// Live smoke: the default dark scene renders real pixels through the
    /// full egui stack (boots one headless harness, like the CLI does).
    #[test]
    fn smoke_dark_route_renders_real_pixels() -> Result<(), String> {
        let mut out = std::env::temp_dir();
        out.push("autocompare-gui-shot-smoke-dark.png");
        render_fixture_png(&out)?;
        let bytes = std::fs::read(&out).map_err(|why| format!("read smoke shot: {why}"))?;
        let stats = check_screenshot_png(&bytes, true)?;
        assert_eq!((stats.width, stats.height), (SNAP_W, SNAP_H));
        let _ = std::fs::remove_file(&out);
        Ok(())
    }
}
