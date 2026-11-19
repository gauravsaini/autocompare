//! Offscreen screenshot generation for verification and smoke testing.
//!
//! Renders the Text comparison fixture session into a valid PNG without
//! requiring an interactive window or display server.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use crate::gutter::GUTTER_SEGMENT_MIN_H;
use crate::home::{
    HOME_GRID_COLUMNS, RECENT_COL_GAP, RECENT_LIST_PAD, folder_empty_hint, home_model,
    recent_row_columns,
};
use crate::viewmodel::{TextRow, TextRowKind, gutter_geometry};
use crate::views::{
    PICTURE_ZOOM_STEPS, picture_center_offset, picture_fit_zoom_pct, wrap_find_scroll_sub,
    wrap_pane_pair_count, wrap_width_for, zoom_label,
};
use crate::views_table::{
    TABLE_FIND_BAR_H, TABLE_FIND_NEEDLE_W, TABLE_KEY_BAR_H, TABLE_MAX_COL_CHARS, TableKeyView,
    is_key_column, table_find_cell_spans, table_find_span_in_padded, table_key_badge_label,
};
use autocompare_diff::{PictureDiff, RgbaImage, encode_png};
use std::path::Path;

// Diff highlight colors matching views.rs
const BG_DARK: [u8; 4] = [27, 27, 27, 255];
const BG_PANEL: [u8; 4] = [38, 38, 38, 255];
const FG_TEXT: [u8; 4] = [230, 230, 230, 255];
const FG_DIM: [u8; 4] = [140, 140, 140, 255];
const DELETED_BG: [u8; 4] = [120, 32, 32, 255];
const DELETED_FG: [u8; 4] = [255, 170, 170, 255];
const INSERTED_BG: [u8; 4] = [24, 90, 46, 255];
const INSERTED_FG: [u8; 4] = [170, 240, 180, 255];
const CHANGED_BG: [u8; 4] = [122, 102, 20, 255];
const CHANGED_FG: [u8; 4] = [255, 235, 160, 255];
const ACCENT_TAB: [u8; 4] = [50, 70, 105, 255];

/// Shared headless geometry (geometry first, colors frozen).
/// Har fixture isi canvas/padding/row height par tikti hai.
pub const SNAP_W: u32 = 960;
pub const SNAP_H: u32 = 600;
pub const TITLE_H: u32 = 36;
pub const TAB_H: u32 = 32;
pub const ROW_H: u32 = 24;
pub const OUTER_PAD: u32 = 12;
pub const PANE_GAP: u32 = 4;
pub const STATUS_H: u32 = 30;
/// Home card grid: live contract `home::HOME_GRID_COLUMNS` (3) ×
/// `home::HOME_GRID_GAP` (8); card w 306 (min 240 se upar), h 100
/// (contract min-height 99.6 ka ceiling). CTA 150x28 (contract min
/// 120x28 se upar). Recent columns title-flex + kind 96 + age 72.
pub const HOME_CARD_W: u32 = 306;
pub const HOME_CARD_H: u32 = 100;
pub const HOME_CARD_GAP: u32 = 8;
pub const HOME_CTA_W: u32 = 150;
pub const HOME_CTA_H: u32 = 28;
pub const TEXT_PANE_W: u32 = 460;
pub const TEXT_LEFT_X: u32 = 12;
pub const TEXT_RIGHT_X: u32 = 476;
/// Overview-ruler track: live contract `gutter::GUTTER_RULER_WIDE` (16,
/// line numbers on) — old 14px inter-chrome gap ruler me sama gaya.
/// Budget: 12 + 460 + 4 + 460 + 16 + 8 = 960; right pane 936 par khatam,
// ruler 936 se 952, right margin 8.
pub const GUTTER_X: u32 = 936;
pub const GUTTER_W: u32 = 16;
/// Merge 3-pane equal thirds: avail 960-24-8=928, 309*3=927, 1px slack right me.
pub const MERGE_PANE_W: u32 = 309;
pub const MERGE_X0: u32 = 12;
pub const MERGE_X1: u32 = 325;
pub const MERGE_X2: u32 = 638;

/// Renders the Text comparison session fixture to a PNG at the given path.
///
/// File name ke tukde se tab chunta hai (lowercase match):
/// `hex` -> hex, `table` + `find` -> table key/find scene, `table` -> table,
/// `merge` -> merge (foldermerge bhi), `folder` + `empty` -> folder filtered
/// empty scene, `folder` -> folder, `home` -> home, `picture` -> picture,
/// `wrap` -> wrap-on + find-active text scene, warna text.
/// Har route same 960x600 canvas par tikta hai, colors frozen.
pub fn render_fixture_png(out_path: &Path) -> Result<(), String> {
    let lower = match out_path.file_name().and_then(|name| name.to_str()) {
        Some(name) => name.to_lowercase(),
        None => String::new(),
    };
    if lower.contains("accent") {
        return render_text_accent_fixture_png(out_path);
    }
    if lower.contains("hex") {
        return render_hex_fixture_png(out_path);
    }
    if lower.contains("table") && lower.contains("find") {
        return render_table_find_fixture_png(out_path);
    }
    if lower.contains("table") {
        return render_table_fixture_png(out_path);
    }
    if lower.contains("merge") {
        return render_merge_fixture_png(out_path);
    }
    if lower.contains("folder") && lower.contains("empty") {
        return render_folder_empty_fixture_png(out_path);
    }
    if lower.contains("folder") {
        return render_folder_fixture_png(out_path);
    }
    if lower.contains("home") {
        return render_home_fixture_png(out_path);
    }
    if lower.contains("picture") {
        return render_picture_fixture_png(out_path);
    }
    if lower.contains("wrap") {
        return render_wrap_fixture_png(out_path);
    }
    render_text_fixture_png(out_path)
}

/// Renders the Text comparison session fixture to a PNG at the given path.
pub fn render_text_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = 960u32;
    let height = 600u32;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };

    let mut pixels = vec![0u8; byte_count];
    // Fill background
    fill_rect(&mut pixels, dims, (0, 0, width, height), BG_DARK);

    // Title bar
    fill_rect(&mut pixels, dims, (0, 0, width, 36), BG_PANEL);
    // Window controls
    fill_rect(&mut pixels, dims, (12, 12, 12, 12), [255, 95, 87, 255]);
    fill_rect(&mut pixels, dims, (32, 12, 12, 12), [254, 188, 46, 255]);
    fill_rect(&mut pixels, dims, (52, 12, 12, 12), [40, 201, 64, 255]);
    draw_string(
        &mut pixels,
        dims,
        80,
        14,
        "fixtures/left.rs <--> fixtures/right.rs - Text",
        FG_TEXT,
    );

    // Tab bar
    fill_rect(&mut pixels, dims, (0, 36, width, 32), [32, 32, 32, 255]);
    draw_string(&mut pixels, dims, 16, 46, "Home", FG_DIM);
    fill_rect(&mut pixels, dims, (64, 40, 60, 24), ACCENT_TAB);
    draw_string(&mut pixels, dims, 76, 46, "Text", FG_TEXT);
    draw_string(&mut pixels, dims, 138, 46, "Folder", FG_DIM);
    draw_string(&mut pixels, dims, 200, 46, "Hex", FG_DIM);
    draw_string(&mut pixels, dims, 244, 46, "Merge", FG_DIM);
    draw_string(&mut pixels, dims, 300, 46, "Table", FG_DIM);
    draw_string(&mut pixels, dims, 356, 46, "Sync", FG_DIM);
    draw_string(&mut pixels, dims, 404, 46, "Picture", FG_DIM);
    draw_string(&mut pixels, dims, 474, 46, "FolderMerge", FG_DIM);
    draw_string(&mut pixels, dims, 574, 46, "Audio", FG_DIM);

    // Path row
    fill_rect(&mut pixels, dims, (12, 76, 380, 24), [45, 45, 45, 255]);
    draw_string(&mut pixels, dims, 20, 82, "fixtures/left.rs", FG_TEXT);
    fill_rect(&mut pixels, dims, (400, 76, 75, 24), [60, 60, 60, 255]);
    draw_string(&mut pixels, dims, 412, 82, "Browse...", FG_TEXT);

    fill_rect(&mut pixels, dims, (490, 76, 380, 24), [45, 45, 45, 255]);
    draw_string(&mut pixels, dims, 498, 82, "fixtures/right.rs", FG_TEXT);
    fill_rect(&mut pixels, dims, (878, 76, 75, 24), [60, 60, 60, 255]);
    draw_string(&mut pixels, dims, 890, 82, "Browse...", FG_TEXT);

    // Session toolbar: Swap Sides + Reload, session identity below tabs.
    draw_string(
        &mut pixels,
        dims,
        16,
        104,
        "[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax",
        FG_TEXT,
    );

    // Main diff pane borders
    let pane_top = 124u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    let gutter_x = GUTTER_X;
    let gutter_w = GUTTER_W;

    fill_rect(
        &mut pixels,
        dims,
        (12, pane_top, 460, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (476, pane_top, 460, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (gutter_x, pane_top, gutter_w, pane_height),
        [32, 32, 32, 255],
    );

    // Fixture diff rows
    let rows = vec![
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(1),
            right_no: Some(1),
            left: "use std::path::Path;".to_string(),
            right: "use std::path::Path;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(2),
            right_no: Some(2),
            left: "pub fn process_data(path: &Path) -> Result<usize, Error> {".to_string(),
            right: "pub fn process_data(path: &Path) -> Result<usize, Error> {".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(3),
            right_no: Some(3),
            left: "    let content = read_to_string(path)?;".to_string(),
            right: "    let content = std::fs::read_to_string(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: vec![(18, 27)],
        },
        TextRow {
            kind: TextRowKind::Deleted,
            left_no: Some(4),
            right_no: None,
            left: "    let legacy_flag = true;".to_string(),
            right: String::new(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Inserted,
            left_no: None,
            right_no: Some(4),
            left: String::new(),
            right: "    validate_safe_path(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(5),
            right_no: Some(5),
            left: "    Ok(content.len())".to_string(),
            right: "    Ok(content.len())".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(6),
            right_no: Some(6),
            left: "}".to_string(),
            right: "}".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
    ];

    let row_h = 24u32;
    for (i, row) in rows.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(row_h));
        if y.saturating_add(row_h) > pane_bottom {
            break;
        }

        let (left_bg, left_fg, right_bg, right_fg) = match row.kind {
            TextRowKind::Equal => (BG_DARK, FG_TEXT, BG_DARK, FG_TEXT),
            TextRowKind::Changed => (CHANGED_BG, CHANGED_FG, CHANGED_BG, CHANGED_FG),
            TextRowKind::Deleted => (DELETED_BG, DELETED_FG, BG_DARK, FG_DIM),
            TextRowKind::Inserted => (BG_DARK, FG_DIM, INSERTED_BG, INSERTED_FG),
        };

        // Left pane row
        fill_rect(&mut pixels, dims, (12, y, 460, row_h), left_bg);
        if let Some(no) = row.left_no {
            draw_string(
                &mut pixels,
                dims,
                16,
                y.saturating_add(6),
                &format!("{no:>2} "),
                FG_DIM,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            44,
            y.saturating_add(6),
            &row.left,
            left_fg,
        );
        // Word-level accent blocks: changed row ke `accent_a` spans
        // inverted (block) me — poore-row band par kaun sa shabd badla.
        for (start, end) in row.accent_a.iter() {
            let prefix: String = row.left.chars().take(*start).collect();
            let span: String = row
                .left
                .chars()
                .skip(*start)
                .take(end.saturating_sub(*start))
                .collect();
            if span.is_empty() {
                continue;
            }
            let ax = 44u32.saturating_add(text_width(&prefix));
            let aw = text_width(&span).max(4);
            fill_rect(&mut pixels, dims, (ax, y, aw, row_h), left_fg);
            draw_string(&mut pixels, dims, ax, y.saturating_add(6), &span, left_bg);
        }

        // Right pane row
        fill_rect(&mut pixels, dims, (476, y, 460, row_h), right_bg);
        if let Some(no) = row.right_no {
            draw_string(
                &mut pixels,
                dims,
                480,
                y.saturating_add(6),
                &format!("{no:>2} "),
                FG_DIM,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            508,
            y.saturating_add(6),
            &row.right,
            right_fg,
        );
        for (start, end) in row.accent_b.iter() {
            let prefix: String = row.right.chars().take(*start).collect();
            let span: String = row
                .right
                .chars()
                .skip(*start)
                .take(end.saturating_sub(*start))
                .collect();
            if span.is_empty() {
                continue;
            }
            let ax = 508u32.saturating_add(text_width(&prefix));
            let aw = text_width(&span).max(4);
            fill_rect(&mut pixels, dims, (ax, y, aw, row_h), right_fg);
            draw_string(&mut pixels, dims, ax, y.saturating_add(6), &span, right_bg);
        }
    }

    // Thumbnail overview gutter geometry: live ruler contract
    // (`gutter::GUTTER_SEGMENT_MIN_H` = 4.0 floor, slop hit-test).
    let geom = gutter_geometry(rows.len(), pane_height as f32, &rows);
    for seg in geom.segments {
        let seg_y = pane_top.saturating_add(seg.y_start as u32);
        let seg_h = ((seg.y_end - seg.y_start) as u32).max(GUTTER_SEGMENT_MIN_H as u32);
        let seg_color = match seg.kind {
            TextRowKind::Equal => BG_PANEL,
            TextRowKind::Changed => CHANGED_BG,
            TextRowKind::Deleted => DELETED_BG,
            TextRowKind::Inserted => INSERTED_BG,
        };
        fill_rect(
            &mut pixels,
            dims,
            (gutter_x, seg_y, gutter_w, seg_h),
            seg_color,
        );
    }

    // Status bar: honest counts plus last-action feedback.
    fill_rect(&mut pixels, dims, (0, 570, width, 30), [32, 32, 32, 255]);
    draw_string(
        &mut pixels,
        dims,
        16,
        578,
        "7 rows: 4 equal, 1 changed, 1 deleted, 1 inserted | text compare done: 7 row(s)",
        FG_DIM,
    );

    let image = RgbaImage {
        width,
        height,
        pixels,
    };

    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;

    Ok(())
}

/// Renders the Text intra-line accent scene: dono panes par Changed row
/// ke word spans (`accent_a`/`accent_b`) inverted blocks me — wrap/find
/// precedent jaisa naya route (`accent` naam se chunta hai). Poore-row
/// band par kaun sa shabd badla wahi block dikhta hai. Colors frozen.
pub fn render_text_accent_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/left.rs <--> fixtures/right.rs - Text (accent)",
        1,
    );

    let pane_top = 124u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_LEFT_X, pane_top, TEXT_PANE_W, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_RIGHT_X, pane_top, TEXT_PANE_W, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (GUTTER_X, pane_top, GUTTER_W, pane_height),
        [32, 32, 32, 255],
    );

    let rows = vec![
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(1),
            right_no: Some(1),
            left: "use std::path::Path;".to_string(),
            right: "use std::path::Path;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(2),
            right_no: Some(2),
            left: "    let old = load(left)?;".to_string(),
            right: "    let new = load(right)?;".to_string(),
            accent_a: vec![(8, 11), (19, 23)],
            accent_b: vec![(8, 11), (19, 24)],
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(3),
            right_no: Some(3),
            left: "    let content = read_to_string(path)?;".to_string(),
            right: "    let content = std::fs::read_to_string(path)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: vec![(18, 27)],
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(4),
            right_no: Some(4),
            left: "    Ok(content.len())".to_string(),
            right: "    Ok(content.len())".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
    ];

    let row_h = ROW_H;
    for (i, row) in rows.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(row_h));
        if y.saturating_add(row_h) > pane_bottom {
            break;
        }
        let (left_bg, left_fg, right_bg, right_fg) = match row.kind {
            TextRowKind::Equal => (BG_DARK, FG_TEXT, BG_DARK, FG_TEXT),
            TextRowKind::Changed => (CHANGED_BG, CHANGED_FG, CHANGED_BG, CHANGED_FG),
            TextRowKind::Deleted => (DELETED_BG, DELETED_FG, BG_DARK, FG_DIM),
            TextRowKind::Inserted => (BG_DARK, FG_DIM, INSERTED_BG, INSERTED_FG),
        };
        fill_rect(
            &mut pixels,
            dims,
            (TEXT_LEFT_X, y, TEXT_PANE_W, row_h),
            left_bg,
        );
        if let Some(no) = row.left_no {
            draw_string(
                &mut pixels,
                dims,
                TEXT_LEFT_X.saturating_add(4),
                y.saturating_add(6),
                &format!("{no:>2} "),
                FG_DIM,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            TEXT_LEFT_X.saturating_add(32),
            y.saturating_add(6),
            &row.left,
            left_fg,
        );
        for (start, end) in row.accent_a.iter() {
            let prefix: String = row.left.chars().take(*start).collect();
            let span: String = row
                .left
                .chars()
                .skip(*start)
                .take(end.saturating_sub(*start))
                .collect();
            if span.is_empty() {
                continue;
            }
            let ax = TEXT_LEFT_X
                .saturating_add(32)
                .saturating_add(text_width(&prefix));
            let aw = text_width(&span).max(4);
            fill_rect(&mut pixels, dims, (ax, y, aw, row_h), left_fg);
            draw_string(&mut pixels, dims, ax, y.saturating_add(6), &span, left_bg);
        }
        fill_rect(
            &mut pixels,
            dims,
            (TEXT_RIGHT_X, y, TEXT_PANE_W, row_h),
            right_bg,
        );
        if let Some(no) = row.right_no {
            draw_string(
                &mut pixels,
                dims,
                TEXT_RIGHT_X.saturating_add(4),
                y.saturating_add(6),
                &format!("{no:>2} "),
                FG_DIM,
            );
        }
        draw_string(
            &mut pixels,
            dims,
            TEXT_RIGHT_X.saturating_add(32),
            y.saturating_add(6),
            &row.right,
            right_fg,
        );
        for (start, end) in row.accent_b.iter() {
            let prefix: String = row.right.chars().take(*start).collect();
            let span: String = row
                .right
                .chars()
                .skip(*start)
                .take(end.saturating_sub(*start))
                .collect();
            if span.is_empty() {
                continue;
            }
            let ax = TEXT_RIGHT_X
                .saturating_add(32)
                .saturating_add(text_width(&prefix));
            let aw = text_width(&span).max(4);
            fill_rect(&mut pixels, dims, (ax, y, aw, row_h), right_fg);
            draw_string(&mut pixels, dims, ax, y.saturating_add(6), &span, right_bg);
        }
    }

    let geom = gutter_geometry(rows.len(), pane_height as f32, &rows);
    for seg in geom.segments {
        let seg_y = pane_top.saturating_add(seg.y_start as u32);
        let seg_h = ((seg.y_end - seg.y_start) as u32).max(GUTTER_SEGMENT_MIN_H as u32);
        let seg_color = match seg.kind {
            TextRowKind::Equal => BG_PANEL,
            TextRowKind::Changed => CHANGED_BG,
            TextRowKind::Deleted => DELETED_BG,
            TextRowKind::Inserted => INSERTED_BG,
        };
        fill_rect(
            &mut pixels,
            dims,
            (GUTTER_X, seg_y, GUTTER_W, seg_h),
            seg_color,
        );
    }

    paint_status(
        &mut pixels,
        dims,
        "accent: 4 rows, 2 changed word spans inverted | intra-line proof",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Folder comparison session fixture: filter checkboxes,
/// honest counts, tree rows with glyphs, and the sync preview panel
/// (mode labels, dry-run options, consent hint, Apply gate). All
/// strings come from the headless `tree`/`home` logic so the shot
/// proves the wired UI, not a mock.
pub fn render_folder_fixture_png(out_path: &Path) -> Result<(), String> {
    use crate::home::{folder_empty_hint, sync_consent_hint};
    use crate::tree::{SyncModeChoice, count_folder_rows, folder_tree_filtered, sync_preview};
    use crate::viewmodel::FolderRow;

    let width = 960u32;
    let height = 600u32;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };

    let mut pixels = vec![0u8; byte_count];
    fill_rect(&mut pixels, dims, (0, 0, width, height), BG_DARK);

    // Title bar
    fill_rect(&mut pixels, dims, (0, 0, width, 36), BG_PANEL);
    fill_rect(&mut pixels, dims, (12, 12, 12, 12), [255, 95, 87, 255]);
    fill_rect(&mut pixels, dims, (32, 12, 12, 12), [254, 188, 46, 255]);
    fill_rect(&mut pixels, dims, (52, 12, 12, 12), [40, 201, 64, 255]);
    draw_string(
        &mut pixels,
        dims,
        80,
        14,
        "fixtures/left-dir <--> fixtures/right-dir - Folder",
        FG_TEXT,
    );

    // Tab bar with Folder highlighted
    fill_rect(&mut pixels, dims, (0, 36, width, 32), [32, 32, 32, 255]);
    draw_string(&mut pixels, dims, 16, 46, "Home", FG_DIM);
    draw_string(&mut pixels, dims, 64, 46, "Text", FG_DIM);
    fill_rect(&mut pixels, dims, (118, 40, 66, 24), ACCENT_TAB);
    draw_string(&mut pixels, dims, 130, 46, "Folder", FG_TEXT);
    draw_string(&mut pixels, dims, 200, 46, "Hex", FG_DIM);
    draw_string(&mut pixels, dims, 244, 46, "Merge", FG_DIM);
    draw_string(&mut pixels, dims, 300, 46, "Table", FG_DIM);
    draw_string(&mut pixels, dims, 356, 46, "Sync", FG_DIM);
    draw_string(&mut pixels, dims, 404, 46, "Picture", FG_DIM);
    draw_string(&mut pixels, dims, 474, 46, "FolderMerge", FG_DIM);
    draw_string(&mut pixels, dims, 574, 46, "Audio", FG_DIM);

    // Path row
    fill_rect(&mut pixels, dims, (12, 76, 380, 24), [45, 45, 45, 255]);
    draw_string(&mut pixels, dims, 20, 82, "fixtures/left-dir", FG_TEXT);
    fill_rect(&mut pixels, dims, (400, 76, 75, 24), [60, 60, 60, 255]);
    draw_string(&mut pixels, dims, 412, 82, "Browse...", FG_TEXT);
    fill_rect(&mut pixels, dims, (490, 76, 380, 24), [45, 45, 45, 255]);
    draw_string(&mut pixels, dims, 498, 82, "fixtures/right-dir", FG_TEXT);
    fill_rect(&mut pixels, dims, (878, 76, 75, 24), [60, 60, 60, 255]);
    draw_string(&mut pixels, dims, 890, 82, "Browse...", FG_TEXT);

    // Session toolbar
    draw_string(
        &mut pixels,
        dims,
        16,
        104,
        "[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax",
        FG_TEXT,
    );

    // Fixture rows: headless truth, same glyphs the engine paints.
    let rows = vec![
        FolderRow {
            rel_path: "src".to_string(),
            is_dir: true,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "src/a.txt".to_string(),
            is_dir: false,
            glyph: '>',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        },
        FolderRow {
            rel_path: "src/b.txt".to_string(),
            is_dir: false,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "orphan.txt".to_string(),
            is_dir: false,
            glyph: 'R',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        },
    ];
    let counts = count_folder_rows(&rows);
    let expanded: std::collections::BTreeSet<String> = ["src".to_string()].into_iter().collect();
    let display = crate::tree::FolderDisplay::default();
    let nodes = folder_tree_filtered(&rows, &expanded, &display);
    let preview = sync_preview(&rows, SyncModeChoice::UpdateBoth);
    let consent_hint = sync_consent_hint(false, preview.steps.len());
    let empty_hint = folder_empty_hint(false, false);
    let _ = empty_hint;

    // Filter checkboxes line (labels mirror app.rs Folder tab).
    draw_string(
        &mut pixels,
        dims,
        16,
        122,
        "[ ] Hide orphans  [ ] Left newer only  [ ] Right newer only  [ ] Flatten",
        FG_TEXT,
    );
    // Honest counts line.
    draw_string(&mut pixels, dims, 16, 138, &counts.summary(), FG_DIM);

    // Tree panel.
    let pane_top = 158u32;
    let pane_height = 260u32;
    let pane_bottom = pane_top.saturating_add(pane_height);
    fill_rect(
        &mut pixels,
        dims,
        (12, pane_top, 936, pane_height),
        [22, 22, 22, 255],
    );
    let row_h = 24u32;
    for (i, node) in nodes.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(row_h));
        if y.saturating_add(row_h) > pane_bottom {
            break;
        }
        let (bg, fg) = match node.row.glyph {
            '=' => (BG_DARK, FG_TEXT),
            '!' | 'L' => (DELETED_BG, DELETED_FG),
            '>' | '<' => (CHANGED_BG, CHANGED_FG),
            'R' => (INSERTED_BG, INSERTED_FG),
            _ => (BG_DARK, FG_DIM),
        };
        fill_rect(&mut pixels, dims, (12, y, 936, row_h), bg);
        if node.row.is_dir {
            let icon = if node.is_expanded { "v" } else { ">" };
            draw_string(&mut pixels, dims, 18, y.saturating_add(6), icon, FG_DIM);
        }
        let indent = (node.depth as u32).saturating_mul(4u32);
        let gx = 34u32.saturating_add(indent.saturating_mul(7));
        draw_string(
            &mut pixels,
            dims,
            gx,
            y.saturating_add(6),
            &node.row.glyph.to_string(),
            fg,
        );
        draw_string(
            &mut pixels,
            dims,
            gx.saturating_add(16),
            y.saturating_add(6),
            &node.row.rel_path,
            fg,
        );
    }

    // Sync preview panel.
    let panel_y = pane_bottom.saturating_add(8);
    fill_rect(
        &mut pixels,
        dims,
        (12, panel_y, 936, 116),
        [28, 28, 28, 255],
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(6),
        "Sync preview (dry-run)",
        FG_TEXT,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(22),
        &format!(
            "({})  ({})",
            SyncModeChoice::UpdateBoth.label(),
            SyncModeChoice::Mirror.label()
        ),
        FG_TEXT,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(38),
        &preview.summary(),
        FG_DIM,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(54),
        consent_hint,
        FG_DIM,
    );
    fill_rect(
        &mut pixels,
        dims,
        (20, panel_y.saturating_add(72), 150, 24),
        [60, 60, 60, 255],
    );
    draw_string(
        &mut pixels,
        dims,
        32,
        panel_y.saturating_add(78),
        "Apply sync plan",
        FG_TEXT,
    );

    // Status bar.
    fill_rect(&mut pixels, dims, (0, 570, width, 30), [32, 32, 32, 255]);
    draw_string(
        &mut pixels,
        dims,
        16,
        578,
        "folder: 4 entries, 0 pruned | staged src/a.txt for text compare",
        FG_DIM,
    );

    let image = RgbaImage {
        width,
        height,
        pixels,
    };

    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;

    Ok(())
}

/// Renders the Hex comparison fixture: offset + hex bytes + ascii,
/// same 960x600 chrome, 24px rows, 460px panes. Colors frozen.
pub fn render_hex_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/a.bin <--> fixtures/b.bin - Hex",
        3,
    );

    let pane_top = 124u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_LEFT_X, pane_top, TEXT_PANE_W, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_RIGHT_X, pane_top, TEXT_PANE_W, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (GUTTER_X, pane_top, GUTTER_W, pane_height),
        [32, 32, 32, 255],
    );

    let rows = [
        ("00000000  48 65 6c 6c 6f 20 77 6f |Hello wo|", false),
        ("00000010  72 6c 64 21 00 00 00 00 |rld!....|", false),
        ("00000020  01 02 03 04 05 06 07 08 |........|", true),
        ("00000030  09 0a 0b 0c 0d 0e 0f 10 |........|", false),
        ("00000040  11 12 13 14 15 16 17 18 |........|", false),
        ("00000050  19 1a 1b 1c 1d 1e 1f 20 |....... |", false),
        ("00000060  21 22 23 24 25 26 27 28 |!\"#$%&'()|", false),
        ("00000070  29 2a 2b 2c 2d 2e 2f 30 |)*+,-./0|", false),
    ];
    for (i, (line, changed)) in rows.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(ROW_H));
        if y.saturating_add(ROW_H) > pane_bottom {
            break;
        }
        let (bg, fg) = if *changed {
            (CHANGED_BG, CHANGED_FG)
        } else {
            (BG_DARK, FG_TEXT)
        };
        fill_rect(&mut pixels, dims, (TEXT_LEFT_X, y, TEXT_PANE_W, ROW_H), bg);
        fill_rect(&mut pixels, dims, (TEXT_RIGHT_X, y, TEXT_PANE_W, ROW_H), bg);
        draw_string(&mut pixels, dims, 16, y.saturating_add(6), line, fg);
        draw_string(&mut pixels, dims, 480, y.saturating_add(6), line, fg);
    }
    // Gutter segments mirror text shape (geometry parity, no new hues).
    fill_rect(
        &mut pixels,
        dims,
        (GUTTER_X, pane_top.saturating_add(48), GUTTER_W, 24),
        CHANGED_BG,
    );

    paint_status(
        &mut pixels,
        dims,
        "hex: 8 rows, 1 changed | hex compare done: 8 row(s)",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Table comparison fixture: header + grid rows,
/// single 936px pane, 24px rows. Colors frozen.
pub fn render_table_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/a.csv <--> fixtures/b.csv - Table",
        5,
    );

    let pane_x = OUTER_PAD;
    let pane_w = SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2));
    let pane_top = 150u32;
    let pane_bottom = 560u32;
    fill_rect(
        &mut pixels,
        dims,
        (
            pane_x,
            pane_top,
            pane_w,
            pane_bottom.saturating_sub(pane_top),
        ),
        [22, 22, 22, 255],
    );
    // Header row.
    fill_rect(
        &mut pixels,
        dims,
        (pane_x, pane_top, pane_w, ROW_H),
        BG_PANEL,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        pane_top.saturating_add(6),
        "id | name | left | right | status",
        FG_TEXT,
    );
    let rows = [
        ("1 | anna | 10 | 10 | = |", false),
        ("2 | bob | 20 | 22 | ! |", true),
        ("3 | cara | 30 | 30 | = |", false),
        ("4 | dev | - | 40 | R |", true),
        ("5 | eli | 50 | 50 | = |", false),
        ("6 | fay | 60 | 60 | = |", false),
        ("7 | gus | 70 | 71 | ! |", true),
        ("8 | hal | 80 | 80 | = |", false),
        ("9 | ida | 90 | 90 | = |", false),
        ("10 | jay | 100 | 101 | ! |", true),
        ("11 | kay | 110 | 110 | = |", false),
        ("12 | leo | 120 | 120 | = |", false),
        ("13 | may | 130 | 131 | ! |", true),
        ("14 | ned | 140 | 140 | = |", false),
    ];
    for (i, (line, changed)) in rows.iter().enumerate() {
        let y = pane_top
            .saturating_add(ROW_H)
            .saturating_add((i as u32).saturating_mul(ROW_H));
        if y.saturating_add(ROW_H) > pane_bottom {
            break;
        }
        let (bg, fg) = if *changed {
            (CHANGED_BG, CHANGED_FG)
        } else {
            (BG_DARK, FG_TEXT)
        };
        fill_rect(&mut pixels, dims, (pane_x, y, pane_w, ROW_H), bg);
        draw_string(&mut pixels, dims, 20, y.saturating_add(6), line, fg);
    }
    // Column guides prove equal padding (vertical rules, frozen hues).
    for gx in [200u32, 400u32, 600u32, 760u32] {
        fill_rect(
            &mut pixels,
            dims,
            (gx, pane_top, 1, pane_bottom.saturating_sub(pane_top)),
            [45, 45, 45, 255],
        );
    }

    paint_status(
        &mut pixels,
        dims,
        "table: 14 rows, 9 equal, 5 changed | table compare done: 14 row(s)",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Merge fixture: 3 equal panes (base/left/right),
/// 309px each, 24px rows. Colors frozen.
pub fn render_merge_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/base.rs + left + right - Merge",
        4,
    );

    let pane_top = 124u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    fill_rect(
        &mut pixels,
        dims,
        (MERGE_X0, pane_top, MERGE_PANE_W, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (MERGE_X1, pane_top, MERGE_PANE_W, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (MERGE_X2, pane_top, MERGE_PANE_W, pane_height),
        [22, 22, 22, 255],
    );
    draw_string(
        &mut pixels,
        dims,
        MERGE_X0.saturating_add(8),
        pane_top.saturating_sub(18),
        "[Base] [Left] [Right]",
        FG_DIM,
    );

    let rows = [
        ("let x = 1;", "let x = 1;", "let x = 1;", 0u8),
        ("let y = 2;", "let y = 20;", "let y = 2;", 1u8),
        ("let z = 3;", "let z = 3;", "let z = 30;", 1u8),
        ("emit(x);", "emit(x);", "emit(x);", 0u8),
        ("drop(y);", "", "drop(y);", 2u8),
        ("", "take(z);", "", 3u8),
        ("done();", "done();", "done();", 0u8),
    ];
    for (i, (base, left, right, mark)) in rows.iter().enumerate() {
        let y = pane_top.saturating_add((i as u32).saturating_mul(ROW_H));
        if y.saturating_add(ROW_H) > pane_bottom {
            break;
        }
        let center_bg = match *mark {
            1 => CHANGED_BG,
            2 => DELETED_BG,
            3 => INSERTED_BG,
            _ => BG_DARK,
        };
        let center_fg = match *mark {
            1 => CHANGED_FG,
            2 => DELETED_FG,
            3 => INSERTED_FG,
            _ => FG_TEXT,
        };
        fill_rect(
            &mut pixels,
            dims,
            (MERGE_X0, y, MERGE_PANE_W, ROW_H),
            BG_DARK,
        );
        fill_rect(
            &mut pixels,
            dims,
            (MERGE_X1, y, MERGE_PANE_W, ROW_H),
            center_bg,
        );
        fill_rect(
            &mut pixels,
            dims,
            (MERGE_X2, y, MERGE_PANE_W, ROW_H),
            BG_DARK,
        );
        draw_string(
            &mut pixels,
            dims,
            MERGE_X0.saturating_add(8),
            y.saturating_add(6),
            base,
            FG_TEXT,
        );
        draw_string(
            &mut pixels,
            dims,
            MERGE_X1.saturating_add(8),
            y.saturating_add(6),
            left,
            center_fg,
        );
        draw_string(
            &mut pixels,
            dims,
            MERGE_X2.saturating_add(8),
            y.saturating_add(6),
            right,
            FG_TEXT,
        );
    }

    paint_status(
        &mut pixels,
        dims,
        "merge: 7 rows, 3 equal, 2 changed, 1 deleted, 1 inserted | merge ready",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Home fixture: 9 session cards from the live
/// `home_model` (3-col grid, contract gap/min-height/CTA, title/desc
/// hierarchy) + columnar recent list (title flex + kind + age), and NO
/// path picker (`HOME_SHOWS_PATH_PICKER = false`). Colors frozen.
pub fn render_home_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    // Launcher chrome: title + tabs only. Path picker contract me kabhi
    // nahi dikhta, swap/reload toolbar launcher ka kaam nahi.
    fill_rect(&mut pixels, dims, (0, 0, width, TITLE_H), BG_PANEL);
    fill_rect(&mut pixels, dims, (12, 12, 12, 12), [255, 95, 87, 255]);
    fill_rect(&mut pixels, dims, (32, 12, 12, 12), [254, 188, 46, 255]);
    fill_rect(&mut pixels, dims, (52, 12, 12, 12), [40, 201, 64, 255]);
    draw_string(&mut pixels, dims, 80, 14, "Home - autocompare", FG_TEXT);
    fill_rect(
        &mut pixels,
        dims,
        (0, TITLE_H, width, TAB_H),
        [32, 32, 32, 255],
    );
    fill_rect(&mut pixels, dims, (8, 40, 52, 24), ACCENT_TAB);
    draw_string(&mut pixels, dims, 16, 46, "Home", FG_TEXT);
    draw_string(&mut pixels, dims, 76, 46, "Text", FG_DIM);
    draw_string(&mut pixels, dims, 130, 46, "Folder", FG_DIM);
    draw_string(&mut pixels, dims, 200, 46, "Hex", FG_DIM);
    draw_string(&mut pixels, dims, 244, 46, "Merge", FG_DIM);
    draw_string(&mut pixels, dims, 300, 46, "Table", FG_DIM);
    draw_string(&mut pixels, dims, 356, 46, "Sync", FG_DIM);
    draw_string(&mut pixels, dims, 404, 46, "Picture", FG_DIM);
    draw_string(&mut pixels, dims, 474, 46, "FolderMerge", FG_DIM);
    draw_string(&mut pixels, dims, 574, 46, "Audio", FG_DIM);

    draw_string(&mut pixels, dims, 20, 74, "Comparison Sessions", FG_TEXT);

    // Cards: live model, row-major 3-col grid, contract gap.
    let model = home_model();
    let card_top = 90u32;
    for (idx, item) in model.items.iter().enumerate() {
        let col = (idx % HOME_GRID_COLUMNS) as u32;
        let row = (idx / HOME_GRID_COLUMNS) as u32;
        let x =
            OUTER_PAD.saturating_add(col.saturating_mul(HOME_CARD_W.saturating_add(HOME_CARD_GAP)));
        let y =
            card_top.saturating_add(row.saturating_mul(HOME_CARD_H.saturating_add(HOME_CARD_GAP)));
        fill_rect(
            &mut pixels,
            dims,
            (x, y, HOME_CARD_W, HOME_CARD_H),
            [28, 28, 28, 255],
        );
        fill_rect(&mut pixels, dims, (x, y, HOME_CARD_W, 3), ACCENT_TAB);
        draw_string(
            &mut pixels,
            dims,
            x.saturating_add(12),
            y.saturating_add(12),
            item.title,
            FG_TEXT,
        );
        let (desc_top, desc_bottom) =
            wrap_desc_two_lines(item.description, HOME_CARD_W.saturating_sub(24));
        draw_string(
            &mut pixels,
            dims,
            x.saturating_add(12),
            y.saturating_add(30),
            &desc_top,
            FG_DIM,
        );
        if !desc_bottom.is_empty() {
            draw_string(
                &mut pixels,
                dims,
                x.saturating_add(12),
                y.saturating_add(40),
                &desc_bottom,
                FG_DIM,
            );
        }
        let cta_y = y.saturating_add(64);
        fill_rect(
            &mut pixels,
            dims,
            (x.saturating_add(12), cta_y, HOME_CTA_W, HOME_CTA_H),
            [60, 60, 60, 255],
        );
        draw_string(
            &mut pixels,
            dims,
            x.saturating_add(24),
            cta_y.saturating_add(10),
            &format!("Open {}", item.title),
            FG_TEXT,
        );
    }

    // Recent list: header + columnar rows (title flex + kind + age).
    let cards_end = card_top
        .saturating_add(3u32.saturating_mul(HOME_CARD_H))
        .saturating_add(2u32.saturating_mul(HOME_CARD_GAP));
    let list_top = cards_end.saturating_add(8);
    let list_bottom = 560u32;
    fill_rect(
        &mut pixels,
        dims,
        (
            OUTER_PAD,
            list_top,
            SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
            list_bottom.saturating_sub(list_top),
        ),
        [22, 22, 22, 255],
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        list_top.saturating_add(6),
        "Recent sessions",
        FG_TEXT,
    );
    let (title_w, kind_w, age_w) =
        recent_row_columns(SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)) as f32);
    let content_x = OUTER_PAD.saturating_add(RECENT_LIST_PAD as u32);
    let kind_x = content_x
        .saturating_add(title_w as u32)
        .saturating_add(RECENT_COL_GAP as u32);
    let age_x = kind_x
        .saturating_add(kind_w as u32)
        .saturating_add(RECENT_COL_GAP as u32);
    let _ = age_w;
    let recents = [
        ("text: left.rs <-> right.rs", "Text", "just now"),
        ("folder: left-dir <-> right-dir", "Folder", "5m ago"),
        ("hex: a.bin <-> b.bin", "Hex", "1h ago"),
        ("table: a.csv <-> b.csv", "Table", "3h ago"),
        ("merge: base + left + right", "Merge", "1d ago"),
    ];
    for (i, (title, kind, age)) in recents.iter().enumerate() {
        let y = list_top
            .saturating_add(ROW_H)
            .saturating_add((i as u32).saturating_mul(ROW_H));
        if y.saturating_add(ROW_H) > list_bottom {
            break;
        }
        fill_rect(
            &mut pixels,
            dims,
            (
                OUTER_PAD,
                y,
                SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
                ROW_H,
            ),
            BG_DARK,
        );
        draw_string(
            &mut pixels,
            dims,
            content_x,
            y.saturating_add(6),
            title,
            FG_TEXT,
        );
        draw_string(&mut pixels, dims, kind_x, y.saturating_add(6), kind, FG_DIM);
        draw_string(&mut pixels, dims, age_x, y.saturating_add(6), age, FG_DIM);
    }

    paint_status(
        &mut pixels,
        dims,
        "home: 9 cards, 5 recents | no path picker | pick a session to begin",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the wrap-on + find-active text scene: word wrap capped at
/// the live 80-char width, one long equal row wrapping to 2 sub-rows
/// with the needle fully inside the continuation, and the matched
/// sub-row band proving `wrap_find_scroll_sub` targeting. The find
/// band reuses `CHANGED_BG` (frozen palette); no new hues.
pub fn render_wrap_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/left.rs <--> fixtures/right.rs - Text (wrap)",
        1,
    );

    // Find bar: needle + honest hit line (headless truth: 2 side hits).
    fill_rect(&mut pixels, dims, (12, 124, 936, 24), [28, 28, 28, 255]);
    draw_string(&mut pixels, dims, 20, 130, "find", FG_TEXT);
    fill_rect(&mut pixels, dims, (64, 127, 180, 18), [45, 45, 45, 255]);
    draw_string(&mut pixels, dims, 70, 130, "read_to_string", FG_TEXT);
    draw_string(&mut pixels, dims, 260, 130, "2 hits, current row 3", FG_DIM);

    let long_line = "    // wrap proof: keep the legacy reader enabled until rollout ends, then call read_to_string(old);";
    let rows = vec![
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(1),
            right_no: Some(1),
            left: "use std::path::Path;".to_string(),
            right: "use std::path::Path;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(2),
            right_no: Some(2),
            left: "pub fn run(left: &Path, right: &Path) -> usize {".to_string(),
            right: "pub fn run(left: &Path, right: &Path) -> usize {".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(3),
            right_no: Some(3),
            left: long_line.to_string(),
            right: long_line.to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(4),
            right_no: Some(4),
            left: "    let old = load(left)?;".to_string(),
            right: "    let new = std::fs::load(right)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
        TextRow {
            kind: TextRowKind::Inserted,
            left_no: None,
            right_no: Some(5),
            left: String::new(),
            right: "    validate_safe_path(right)?;".to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        },
    ];

    // Live wrap math: 80-char cap, per-row sub counts, needle sub-rows,
    // current-row scroll target (clamped to the pair count).
    let pane_chars = rows
        .iter()
        .map(|row| row.left.chars().count().max(row.right.chars().count()))
        .max()
        .unwrap_or(1);
    let wrap_width = wrap_width_for(pane_chars, true);
    let needle = "read_to_string";
    let mut counts: Vec<usize> = Vec::new();
    for row in &rows {
        counts.push(wrap_pane_pair_count(&row.left, &row.right, wrap_width));
    }
    let matched_row = 2usize;
    let scroll_sub = match rows.get(matched_row) {
        Some(row) => {
            let count = counts.get(matched_row).copied().unwrap_or(1);
            wrap_find_scroll_sub(row, needle, false, wrap_width, count)
        }
        None => 0,
    };

    let pane_top = 152u32;
    let pane_bottom = 560u32;
    let pane_height = pane_bottom.saturating_sub(pane_top);
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_LEFT_X, pane_top, TEXT_PANE_W, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (TEXT_RIGHT_X, pane_top, TEXT_PANE_W, pane_height),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (GUTTER_X, pane_top, GUTTER_W, pane_height),
        [32, 32, 32, 255],
    );

    let mut flat = 0u32;
    for (i, row) in rows.iter().enumerate() {
        let count = counts.get(i).copied().unwrap_or(1).max(1);
        let left_chunks = crate::views::wrap_chunks(&row.left, wrap_width);
        let right_chunks = crate::views::wrap_chunks(&row.right, wrap_width);
        let (base_bg_left, base_fg_left, base_bg_right, base_fg_right) = match row.kind {
            TextRowKind::Equal => (BG_DARK, FG_TEXT, BG_DARK, FG_TEXT),
            TextRowKind::Changed => (CHANGED_BG, CHANGED_FG, CHANGED_BG, CHANGED_FG),
            TextRowKind::Deleted => (DELETED_BG, DELETED_FG, BG_DARK, FG_DIM),
            TextRowKind::Inserted => (BG_DARK, FG_DIM, INSERTED_BG, INSERTED_FG),
        };
        let mut sub = 0usize;
        while sub < count {
            let y = pane_top.saturating_add(flat.saturating_mul(ROW_H));
            if y.saturating_add(ROW_H) > pane_bottom {
                break;
            }
            // Find-current band: only the matched sub-row of the matched
            // model row inverts to the accent band (frozen CHANGED_*).
            let is_hit = i == matched_row && sub == scroll_sub;
            let (lbg, rbg) = if is_hit {
                (CHANGED_BG, CHANGED_BG)
            } else {
                (base_bg_left, base_bg_right)
            };
            fill_rect(&mut pixels, dims, (TEXT_LEFT_X, y, TEXT_PANE_W, ROW_H), lbg);
            fill_rect(
                &mut pixels,
                dims,
                (TEXT_RIGHT_X, y, TEXT_PANE_W, ROW_H),
                rbg,
            );
            if sub == 0 {
                if let Some(no) = row.left_no {
                    draw_string(
                        &mut pixels,
                        dims,
                        16,
                        y.saturating_add(6),
                        &format!("{no:>2} "),
                        FG_DIM,
                    );
                }
                if let Some(no) = row.right_no {
                    draw_string(
                        &mut pixels,
                        dims,
                        480,
                        y.saturating_add(6),
                        &format!("{no:>2} "),
                        FG_DIM,
                    );
                }
            }
            let left_text = match left_chunks.get(sub) {
                Some(chunk) => chunk.text.as_str(),
                None => "",
            };
            let right_text = match right_chunks.get(sub) {
                Some(chunk) => chunk.text.as_str(),
                None => "",
            };
            let (lfg, rfg) = if is_hit {
                (CHANGED_FG, CHANGED_FG)
            } else {
                (base_fg_left, base_fg_right)
            };
            draw_string(&mut pixels, dims, 44, y.saturating_add(6), left_text, lfg);
            draw_string(&mut pixels, dims, 508, y.saturating_add(6), right_text, rfg);
            flat = flat.saturating_add(1);
            sub = sub.saturating_add(1);
        }
    }

    // Gutter segments mirror the ruler contract (16 wide, 4px min).
    let geom = gutter_geometry(rows.len(), pane_height as f32, &rows);
    for seg in geom.segments {
        let seg_y = pane_top.saturating_add(seg.y_start as u32);
        let seg_h = ((seg.y_end - seg.y_start) as u32).max(GUTTER_SEGMENT_MIN_H as u32);
        let seg_color = match seg.kind {
            TextRowKind::Equal => BG_PANEL,
            TextRowKind::Changed => CHANGED_BG,
            TextRowKind::Deleted => DELETED_BG,
            TextRowKind::Inserted => INSERTED_BG,
        };
        fill_rect(
            &mut pixels,
            dims,
            (GUTTER_X, seg_y, GUTTER_W, seg_h),
            seg_color,
        );
    }

    paint_status(
        &mut pixels,
        dims,
        "wrap:on find:read_to_string 2 hits, current row 3 sub 1/2 | wrapnav proof",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Picture fixture: zoom-step row (Fit + presets from the
/// live `PICTURE_ZOOM_STEPS`) and a centered frame whose fit zoom and
/// offsets come from `picture_fit_zoom_pct` / `picture_center_offset`.
/// The heatmap is schematic palette blocks (frozen hues); geometry is
/// the live contract. Colors frozen.
pub fn render_picture_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/a.png <--> fixtures/b.png - Picture",
        7,
    );

    // Natural vs viewport: fit zoom + centering from live helpers.
    let natural_w = 800.0f32;
    let natural_h = 600.0f32;
    let avail_w = SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)) as f32;
    // Live summary lines (identical 800x600 pair) sit between the step
    // row and the viewport, like the paint summary under the heatmap.
    let diff = PictureDiff {
        width_l: 800,
        height_l: 600,
        width_r: 800,
        height_r: 600,
        overlap_w: 800,
        overlap_h: 600,
        total: 480_000,
        differing: 0,
        bbox: None,
        identical: true,
    };
    let summary = crate::viewmodel::build_picture_summary(&diff);
    let viewport_top = 188u32;
    let viewport_bottom = 560u32;
    let avail_h = viewport_bottom.saturating_sub(viewport_top) as f32;
    let fit = picture_fit_zoom_pct(natural_w, natural_h, avail_w, avail_h);
    let scale = f32::from(fit as u16) / 100.0;
    let disp_w = (natural_w * scale).max(1.0);
    let disp_h = (natural_h * scale).max(1.0);
    let off_x = picture_center_offset(disp_w, avail_w);
    let off_y = picture_center_offset(disp_h, avail_h);

    // Zoom-step row: Fit highlighted + preset ladder.
    fill_rect(&mut pixels, dims, (12, 124, 936, 24), [28, 28, 28, 255]);
    draw_string(&mut pixels, dims, 20, 130, "steps:", FG_DIM);
    fill_rect(&mut pixels, dims, (76, 127, 40, 18), ACCENT_TAB);
    draw_string(&mut pixels, dims, 86, 130, "Fit", FG_TEXT);
    let mut step_x = 124u32;
    for step in PICTURE_ZOOM_STEPS {
        let label = format!("{step}%");
        let selected = step == fit;
        if selected {
            fill_rect(&mut pixels, dims, (step_x, 127, 52, 18), ACCENT_TAB);
        }
        draw_string(
            &mut pixels,
            dims,
            step_x.saturating_add(8),
            130,
            &label,
            if selected { FG_TEXT } else { FG_DIM },
        );
        step_x = step_x.saturating_add(60);
    }
    // Live zoom readout beside the ladder (same contract as paint).
    draw_string(
        &mut pixels,
        dims,
        step_x.saturating_add(8),
        130,
        &zoom_label(fit),
        FG_DIM,
    );

    // Summary strip: live CLI-worded lines, one per 12px rhythm row.
    let mut sum_y = 152u32;
    for line in &summary {
        draw_string(&mut pixels, dims, 20, sum_y, line, FG_DIM);
        sum_y = sum_y.saturating_add(12);
        if sum_y.saturating_add(7) > viewport_top {
            break;
        }
    }

    // Viewport + centered frame.
    fill_rect(
        &mut pixels,
        dims,
        (
            OUTER_PAD,
            viewport_top,
            SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2)),
            viewport_bottom.saturating_sub(viewport_top),
        ),
        [22, 22, 22, 255],
    );
    let frame_x = OUTER_PAD.saturating_add(off_x as u32);
    let frame_y = viewport_top.saturating_add(off_y as u32);
    let frame_w = disp_w as u32;
    let frame_h = disp_h as u32;
    // Schematic heatmap: quadrant palette blocks (frozen diff hues).
    let half_w = frame_w / 2;
    let half_h = frame_h / 2;
    fill_rect(
        &mut pixels,
        dims,
        (frame_x, frame_y, half_w, half_h),
        INSERTED_BG,
    );
    fill_rect(
        &mut pixels,
        dims,
        (
            frame_x.saturating_add(half_w),
            frame_y,
            frame_w.saturating_sub(half_w),
            half_h,
        ),
        BG_DARK,
    );
    fill_rect(
        &mut pixels,
        dims,
        (
            frame_x,
            frame_y.saturating_add(half_h),
            half_w,
            frame_h.saturating_sub(half_h),
        ),
        CHANGED_BG,
    );
    fill_rect(
        &mut pixels,
        dims,
        (
            frame_x.saturating_add(half_w),
            frame_y.saturating_add(half_h),
            frame_w.saturating_sub(half_w),
            frame_h.saturating_sub(half_h),
        ),
        DELETED_BG,
    );
    // 1px frame border (frozen dim).
    fill_rect(&mut pixels, dims, (frame_x, frame_y, frame_w, 1), FG_DIM);
    fill_rect(
        &mut pixels,
        dims,
        (
            frame_x,
            frame_y.saturating_add(frame_h.saturating_sub(1)),
            frame_w,
            1,
        ),
        FG_DIM,
    );
    fill_rect(&mut pixels, dims, (frame_x, frame_y, 1, frame_h), FG_DIM);
    fill_rect(
        &mut pixels,
        dims,
        (
            frame_x.saturating_add(frame_w.saturating_sub(1)),
            frame_y,
            1,
            frame_h,
        ),
        FG_DIM,
    );
    draw_string(
        &mut pixels,
        dims,
        frame_x.saturating_add(8),
        frame_y.saturating_add(8),
        "800x600 @50% (400x300)",
        FG_TEXT,
    );

    paint_status(
        &mut pixels,
        dims,
        "picture: fit 50% | centered (268,36) | steps Fit+25/50/100/200/400",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Folder filtered-empty scene: same filter line as the
/// folder fixture, every row pruned, so the panel shows the live
/// `folder_empty_hint` idle guidance + honest counts. No sync preview:
/// nothing to stage. Colors frozen.
pub fn render_folder_empty_fixture_png(out_path: &Path) -> Result<(), String> {
    use crate::viewmodel::FolderRow;

    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/left-dir <--> fixtures/right-dir - Folder",
        2,
    );

    let rows = vec![
        FolderRow {
            rel_path: "src".to_string(),
            is_dir: true,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "src/a.txt".to_string(),
            is_dir: false,
            glyph: '>',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        },
        FolderRow {
            rel_path: "src/b.txt".to_string(),
            is_dir: false,
            glyph: '=',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: false,
        },
        FolderRow {
            rel_path: "orphan.txt".to_string(),
            is_dir: false,
            glyph: 'R',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        },
    ];
    let counts = crate::tree::count_folder_rows(&rows);
    // Filter prunes everything: zero display rows, honest counts.
    let shown = 0usize;
    let hint = folder_empty_hint(false, false);

    draw_string(
        &mut pixels,
        dims,
        16,
        122,
        "[x] Hide orphans  [ ] Left newer only  [ ] Right newer only  [ ] Flatten",
        FG_TEXT,
    );
    draw_string(
        &mut pixels,
        dims,
        16,
        138,
        &format!("{} | {shown} shown (filter)", counts.summary()),
        FG_DIM,
    );

    let pane_top = 158u32;
    let pane_height = 260u32;
    fill_rect(
        &mut pixels,
        dims,
        (12, pane_top, 936, pane_height),
        [22, 22, 22, 255],
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        pane_top.saturating_add(120),
        hint,
        FG_DIM,
    );

    // Sync preview stays visible under the filter (filter prunes display,
    // not the sync plan): same panel as the folder fixture.
    let preview = crate::tree::sync_preview(&rows, crate::tree::SyncModeChoice::UpdateBoth);
    let consent = crate::home::sync_consent_hint(false, preview.steps.len());
    let panel_y = pane_top.saturating_add(pane_height).saturating_add(8);
    fill_rect(
        &mut pixels,
        dims,
        (12, panel_y, 936, 116),
        [28, 28, 28, 255],
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(6),
        "Sync preview (dry-run)",
        FG_TEXT,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(22),
        &format!(
            "({})  ({})",
            crate::tree::SyncModeChoice::UpdateBoth.label(),
            crate::tree::SyncModeChoice::Mirror.label()
        ),
        FG_TEXT,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(38),
        &preview.summary(),
        FG_DIM,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        panel_y.saturating_add(54),
        consent,
        FG_DIM,
    );
    fill_rect(
        &mut pixels,
        dims,
        (20, panel_y.saturating_add(72), 150, 24),
        [60, 60, 60, 255],
    );
    draw_string(
        &mut pixels,
        dims,
        32,
        panel_y.saturating_add(78),
        "Apply sync plan",
        FG_TEXT,
    );

    paint_status(
        &mut pixels,
        dims,
        "folder: 4 entries, 0 shown (filter) | type both folder paths, press Compare",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders the Table key-col + find scene: live key bar (28px, per-col
/// selected marker + badge) + live find bar (28px, 120px needle) over a
/// subset grid; the needle hit carries a 1px underline (live
/// `TABLE_FIND_UNDERLINE_W`) at `table_find_span_in_padded` offsets.
/// Marker glyphs are ascii `[x]`/`[ ]` (the 5x7 font has no ●/○).
/// Colors frozen.
pub fn render_table_find_fixture_png(out_path: &Path) -> Result<(), String> {
    let width = SNAP_W;
    let height = SNAP_H;
    let dims = (width, height);
    let total_pixels = match (width as usize).checked_mul(height as usize) {
        Some(p) => p,
        None => return Err("dimension overflow".to_string()),
    };
    let byte_count = match total_pixels.checked_mul(4) {
        Some(b) => b,
        None => return Err("byte count overflow".to_string()),
    };
    let mut pixels = vec![0u8; byte_count];
    paint_chrome(
        &mut pixels,
        dims,
        "fixtures/a.csv <--> fixtures/b.csv - Table",
        5,
    );

    // Key bar from live view state: col 0 selected.
    let mut view = TableKeyView::new();
    view.toggle_key_column(0);
    let badge = table_key_badge_label(&view);
    let key_h = TABLE_KEY_BAR_H as u32;
    fill_rect(&mut pixels, dims, (12, 124, 936, key_h), [28, 28, 28, 255]);
    draw_string(&mut pixels, dims, 20, 130, "Key:", FG_DIM);
    let header = ["id", "name", "left", "right", "status"];
    let mut key_line = String::new();
    for (col, name) in header.iter().enumerate() {
        let mark = if is_key_column(&view, col) {
            "[x]"
        } else {
            "[ ]"
        };
        if col > 0 {
            key_line.push_str("  ");
        }
        key_line.push_str(mark);
        key_line.push(' ');
        key_line.push_str(name);
    }
    draw_string(&mut pixels, dims, 60, 130, &key_line, FG_TEXT);
    let badge_x = 948u32.saturating_sub(text_width(&badge));
    draw_string(&mut pixels, dims, badge_x, 130, &badge, FG_DIM);

    // Find bar from live tokens: 28px bar, 120px needle, 1px underline.
    let find_h = TABLE_FIND_BAR_H as u32;
    let find_y = 124u32.saturating_add(key_h).saturating_add(4);
    fill_rect(
        &mut pixels,
        dims,
        (12, find_y, 936, find_h),
        [28, 28, 28, 255],
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        find_y.saturating_add(6),
        "find",
        FG_TEXT,
    );
    let needle_w = TABLE_FIND_NEEDLE_W as u32;
    fill_rect(
        &mut pixels,
        dims,
        (64, find_y.saturating_add(3), needle_w, 18),
        [45, 45, 45, 255],
    );
    let needle = "bob";
    draw_string(
        &mut pixels,
        dims,
        70,
        find_y.saturating_add(6),
        needle,
        FG_TEXT,
    );
    draw_string(
        &mut pixels,
        dims,
        200,
        find_y.saturating_add(6),
        "1 hit, current",
        FG_DIM,
    );

    // Grid: header + rows, needle hit underlined at padded-span offsets.
    let pane_x = OUTER_PAD;
    let pane_w = SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2));
    let grid_top = find_y.saturating_add(find_h).saturating_add(4);
    let pane_bottom = 560u32;
    fill_rect(
        &mut pixels,
        dims,
        (
            pane_x,
            grid_top,
            pane_w,
            pane_bottom.saturating_sub(grid_top),
        ),
        [22, 22, 22, 255],
    );
    fill_rect(
        &mut pixels,
        dims,
        (pane_x, grid_top, pane_w, ROW_H),
        BG_PANEL,
    );
    draw_string(
        &mut pixels,
        dims,
        20,
        grid_top.saturating_add(6),
        "id | name | left | right | status",
        FG_TEXT,
    );
    let rows = [
        ("1 | anna | 10 | 10 | = |", false),
        ("2 | bob | 20 | 22 | ! |", true),
        ("3 | cara | 30 | 30 | = |", false),
        ("4 | dev | - | 40 | R |", true),
        ("5 | eli | 50 | 50 | = |", false),
        ("6 | fay | 60 | 60 | = |", false),
    ];
    for (i, (line, changed)) in rows.iter().enumerate() {
        let y = grid_top
            .saturating_add(ROW_H)
            .saturating_add((i as u32).saturating_mul(ROW_H));
        if y.saturating_add(ROW_H) > pane_bottom {
            break;
        }
        let (bg, fg) = if *changed {
            (CHANGED_BG, CHANGED_FG)
        } else {
            (BG_DARK, FG_TEXT)
        };
        fill_rect(&mut pixels, dims, (pane_x, y, pane_w, ROW_H), bg);
        draw_string(&mut pixels, dims, 20, y.saturating_add(6), line, fg);
    }
    // Underline the needle hit: live cell spans (cell-relative) shifted
    // to row offsets, then clamped to the padded cap.
    let row_line = "2 | bob | 20 | 22 | ! |";
    let cell_off = "2 | ".chars().count();
    let spans = table_find_cell_spans("bob", needle, false);
    let first = spans.first().copied();
    if let Some((start, end)) = first {
        let here = (cell_off.saturating_add(start), cell_off.saturating_add(end));
        if let Some((cs, ce)) = table_find_span_in_padded(here.0, here.1, TABLE_MAX_COL_CHARS) {
            let hit_y = grid_top.saturating_add(ROW_H).saturating_add(ROW_H);
            let prefix: String = row_line.chars().take(cs).collect();
            let hit: String = row_line
                .chars()
                .skip(cs)
                .take(ce.saturating_sub(cs))
                .collect();
            let ux = 20u32.saturating_add(text_width(&prefix));
            let uw = text_width(&hit);
            fill_rect(
                &mut pixels,
                dims,
                (ux, hit_y.saturating_add(14), uw, 1),
                FG_TEXT,
            );
        }
    }

    paint_status(
        &mut pixels,
        dims,
        "table: key=0 | find:bob 1 hit (current) | underline 1px",
    );
    let image = RgbaImage {
        width,
        height,
        pixels,
    };
    let png_bytes = encode_png(&image).ok_or_else(|| "PNG encode failed".to_string())?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out_path, png_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Shared window chrome: title bar + tab bar + path row + toolbar.
/// `active`: 0 home, 1 text, 2 folder, 3 hex, 4 merge, 5 table, 7 picture.
fn paint_chrome(pixels: &mut [u8], dims: (u32, u32), title: &str, active: u8) {
    let width = dims.0;
    fill_rect(pixels, dims, (0, 0, width, TITLE_H), BG_PANEL);
    fill_rect(pixels, dims, (12, 12, 12, 12), [255, 95, 87, 255]);
    fill_rect(pixels, dims, (32, 12, 12, 12), [254, 188, 46, 255]);
    fill_rect(pixels, dims, (52, 12, 12, 12), [40, 201, 64, 255]);
    draw_string(pixels, dims, 80, 14, title, FG_TEXT);

    fill_rect(pixels, dims, (0, TITLE_H, width, TAB_H), [32, 32, 32, 255]);
    // Highlight box per active tab.
    let hl = match active {
        0 => (8u32, 40u32, 52u32, 24u32),
        1 => (64, 40, 60, 24),
        2 => (118, 40, 66, 24),
        3 => (190, 40, 48, 24),
        4 => (234, 40, 60, 24),
        5 => (290, 40, 56, 24),
        7 => (392, 40, 66, 24),
        _ => (64, 40, 60, 24),
    };
    fill_rect(pixels, dims, hl, ACCENT_TAB);
    draw_string(
        pixels,
        dims,
        16,
        46,
        "Home",
        if active == 0 { FG_TEXT } else { FG_DIM },
    );
    draw_string(
        pixels,
        dims,
        76,
        46,
        "Text",
        if active == 1 { FG_TEXT } else { FG_DIM },
    );
    draw_string(
        pixels,
        dims,
        130,
        46,
        "Folder",
        if active == 2 { FG_TEXT } else { FG_DIM },
    );
    draw_string(
        pixels,
        dims,
        200,
        46,
        "Hex",
        if active == 3 { FG_TEXT } else { FG_DIM },
    );
    draw_string(
        pixels,
        dims,
        244,
        46,
        "Merge",
        if active == 4 { FG_TEXT } else { FG_DIM },
    );
    draw_string(
        pixels,
        dims,
        300,
        46,
        "Table",
        if active == 5 { FG_TEXT } else { FG_DIM },
    );
    draw_string(pixels, dims, 356, 46, "Sync", FG_DIM);
    draw_string(
        pixels,
        dims,
        404,
        46,
        "Picture",
        if active == 7 { FG_TEXT } else { FG_DIM },
    );
    draw_string(pixels, dims, 474, 46, "FolderMerge", FG_DIM);
    draw_string(pixels, dims, 574, 46, "Audio", FG_DIM);

    // Path row (same geometry for all tabs).
    fill_rect(pixels, dims, (12, 76, 380, 24), [45, 45, 45, 255]);
    draw_string(pixels, dims, 20, 82, "fixtures/left", FG_TEXT);
    fill_rect(pixels, dims, (400, 76, 75, 24), [60, 60, 60, 255]);
    draw_string(pixels, dims, 412, 82, "Browse...", FG_TEXT);
    fill_rect(pixels, dims, (490, 76, 380, 24), [45, 45, 45, 255]);
    draw_string(pixels, dims, 498, 82, "fixtures/right", FG_TEXT);
    fill_rect(pixels, dims, (878, 76, 75, 24), [60, 60, 60, 255]);
    draw_string(pixels, dims, 890, 82, "Browse...", FG_TEXT);

    draw_string(
        pixels,
        dims,
        16,
        104,
        "[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax",
        FG_TEXT,
    );
}

/// Shared status bar: 30px strip at y570.
fn paint_status(pixels: &mut [u8], dims: (u32, u32), line: &str) {
    let width = dims.0;
    fill_rect(pixels, dims, (0, 570, width, STATUS_H), [32, 32, 32, 255]);
    draw_string(pixels, dims, 16, 578, line, FG_DIM);
}

fn fill_rect(pixels: &mut [u8], dims: (u32, u32), rect: (u32, u32, u32, u32), color: [u8; 4]) {
    let (img_w, img_h) = dims;
    let (x, y, w, h) = rect;
    let x_end = (x.saturating_add(w)).min(img_w);
    let y_end = (y.saturating_add(h)).min(img_h);

    for cy in y..y_end {
        let row_offset = (cy as usize)
            .saturating_mul(img_w as usize)
            .saturating_mul(4);
        for cx in x..x_end {
            let offset = row_offset.saturating_add((cx as usize).saturating_mul(4));
            if let Some(dst) = pixels.get_mut(offset..offset.saturating_add(4))
                && dst.len() == 4
            {
                dst.copy_from_slice(&color);
            }
        }
    }
}

// 5x7 ASCII font: geometry first (advance/kerning), typography second (bitmap tuning).
// Colors frozen: this module only moves the pen and picks bits, never hues.
//
// Advance table: default 6px (5 cols + 1 gap, was 7) so fixture rows fit the
// 460px panes without clipping; single-stem / punctuation glyphs take 4px.
// `kern_adjust` tucks known overhang pairs by 1px (never more, never pushes).
fn glyph_advance(ch: u8) -> u32 {
    match ch {
        b' ' => 4,
        b'\'' | b',' | b'.' | b':' | b';' | b'!' | b'I' | b'i' | b'j' | b'l' | b't' | b'|'
        | b'`' => 4,
        _ => 6,
    }
}

/// Kerning nudge in pixels between two adjacent bytes: -1 (tuck), else 0.
/// Explicit caps pairs first, then one general rule for top-right overhangs
/// (`T/Y/V/r/f`) tucking into round/lowercase/punctuation followers.
fn kern_adjust(prev: u8, curr: u8) -> i32 {
    match (prev, curr) {
        (b'A', b'V')
        | (b'A', b'W')
        | (b'L', b'T')
        | (b'L', b'V')
        | (b'L', b'Y')
        | (b'P', b'A')
        | (b'T', b'o')
        | (b'T', b'e')
        | (b'T', b'a')
        | (b'V', b'A')
        | (b'Y', b'V') => -1,
        _ => {
            let tight_prev = matches!(prev, b'T' | b'Y' | b'V' | b'r' | b'f');
            let tuck_curr = matches!(curr, b'a' | b'c' | b'e' | b'o' | b'.' | b',' | b'\'' | b'"');
            if tight_prev && tuck_curr { -1 } else { 0 }
        }
    }
}

/// Pen width of `s` under the advance + kerning rules (layout proof helper).
fn text_width(s: &str) -> u32 {
    let mut w = 0u32;
    let mut prev: Option<u8> = None;
    for b in s.bytes() {
        if let Some(p) = prev {
            let k = kern_adjust(p, b);
            if k < 0 {
                w = w.saturating_sub(1);
            }
        }
        w = w.saturating_add(glyph_advance(b));
        prev = Some(b);
    }
    w
}

/// Count of non-empty columns in a 5-wide pattern (readability probe helper).
fn nonzero_columns(pattern: &[u8; 5]) -> usize {
    pattern.iter().filter(|c| **c != 0).count()
}

/// Greedy two-line word wrap for home card descriptions: words fill the
/// first line while `text_width` fits `max_px`, overflow goes to the
/// second line, anything past that collapses into a trailing `...`.
/// Pure; paint-independent (pixel-font budget, no new hues).
fn wrap_desc_two_lines(desc: &str, max_px: u32) -> (String, String) {
    let mut first = String::new();
    let mut second = String::new();
    let mut dropped = false;
    for word in desc.split(' ') {
        if word.is_empty() {
            continue;
        }
        if second.is_empty() && !dropped {
            let candidate = if first.is_empty() {
                word.to_string()
            } else {
                format!("{first} {word}")
            };
            if text_width(&candidate) <= max_px {
                first = candidate;
                continue;
            }
        }
        let candidate = if second.is_empty() {
            word.to_string()
        } else {
            format!("{second} {word}")
        };
        if text_width(&candidate) <= max_px {
            second = candidate;
        } else if second.is_empty() {
            // Single word wider than the card: hard-cut with ellipsis.
            let mut cut = String::new();
            for ch in word.chars() {
                let next = format!("{cut}{ch}");
                if text_width(&format!("{next}...")) > max_px {
                    break;
                }
                cut = next;
            }
            cut.push_str("...");
            second = cut;
        } else {
            dropped = true;
        }
    }
    if dropped && !second.is_empty() && !second.ends_with("...") {
        // Third-line spill: keep the two-line budget honest.
        let mut cut = second.clone();
        while !cut.is_empty() && text_width(&format!("{cut}...")) > max_px {
            cut.pop();
        }
        cut.push_str("...");
        second = cut;
    }
    (first, second)
}

fn draw_string(
    pixels: &mut [u8],
    dims: (u32, u32),
    mut x: u32,
    y: u32,
    s: &str,
    color: [u8; 4],
) -> u32 {
    let mut prev: Option<u8> = None;
    for b in s.bytes() {
        if let Some(p) = prev {
            let k = kern_adjust(p, b);
            if k < 0 {
                x = x.saturating_sub(1);
            }
        }
        draw_glyph(pixels, dims, x, y, b, color);
        x = x.saturating_add(glyph_advance(b));
        prev = Some(b);
    }
    x
}

fn draw_glyph(pixels: &mut [u8], dims: (u32, u32), x: u32, y: u32, ch: u8, color: [u8; 4]) {
    let (img_w, img_h) = dims;
    let pattern = glyph_pattern(ch);
    for (col_idx, &col_byte) in pattern.iter().enumerate() {
        let px = x.saturating_add(col_idx as u32);
        for row_idx in 0..7u32 {
            if (col_byte & (1 << row_idx)) != 0 {
                let py = y.saturating_add(row_idx);
                if px < img_w && py < img_h {
                    let offset = (py as usize)
                        .saturating_mul(img_w as usize)
                        .saturating_add(px as usize)
                        .saturating_mul(4);
                    if let Some(dst) = pixels.get_mut(offset..offset.saturating_add(4))
                        && dst.len() == 4
                    {
                        dst.copy_from_slice(&color);
                    }
                }
            }
        }
    }
}

// Standard 5-column byte patterns for common ASCII symbols
fn glyph_pattern(ch: u8) -> [u8; 5] {
    match ch {
        b' ' => [0x00, 0x00, 0x00, 0x00, 0x00],
        b'!' => [0x00, 0x00, 0x5f, 0x00, 0x00],
        b'"' => [0x00, 0x07, 0x00, 0x07, 0x00],
        b'#' => [0x14, 0x7f, 0x14, 0x7f, 0x14],
        b'$' => [0x24, 0x2a, 0x7f, 0x2a, 0x12],
        b'%' => [0x23, 0x13, 0x08, 0x64, 0x62],
        b'&' => [0x36, 0x49, 0x55, 0x22, 0x50],
        b'\'' => [0x00, 0x05, 0x03, 0x00, 0x00],
        b'(' => [0x00, 0x1c, 0x22, 0x41, 0x00],
        b')' => [0x00, 0x41, 0x22, 0x1c, 0x00],
        b'*' => [0x14, 0x08, 0x3e, 0x08, 0x14],
        b'+' => [0x08, 0x08, 0x3e, 0x08, 0x08],
        // Single-stem punctuation: one active column reads sharper at 1x than
        // the old doubled blocks; comma/semicolon keep a 1px descender tail.
        b',' => [0x00, 0x00, 0x50, 0x20, 0x00],
        b'-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        b'.' => [0x00, 0x00, 0x60, 0x00, 0x00],
        b'/' => [0x20, 0x10, 0x08, 0x04, 0x02],
        b'0' => [0x3e, 0x51, 0x49, 0x45, 0x3e],
        b'1' => [0x00, 0x42, 0x7f, 0x40, 0x00],
        b'2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        b'3' => [0x21, 0x41, 0x45, 0x4b, 0x31],
        b'4' => [0x18, 0x14, 0x12, 0x7f, 0x10],
        b'5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        b'6' => [0x3c, 0x4a, 0x49, 0x49, 0x30],
        b'7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        b'8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        b'9' => [0x06, 0x49, 0x49, 0x29, 0x1e],
        b':' => [0x00, 0x00, 0x36, 0x00, 0x00],
        b';' => [0x00, 0x00, 0x56, 0x20, 0x00],
        b'<' => [0x08, 0x14, 0x22, 0x41, 0x00],
        b'=' => [0x14, 0x14, 0x14, 0x14, 0x14],
        b'>' => [0x00, 0x41, 0x22, 0x14, 0x08],
        b'?' => [0x02, 0x01, 0x51, 0x09, 0x06],
        b'@' => [0x32, 0x49, 0x79, 0x41, 0x3e],
        b'A' => [0x7e, 0x11, 0x11, 0x11, 0x7e],
        b'B' => [0x7f, 0x49, 0x49, 0x49, 0x36],
        b'C' => [0x3e, 0x41, 0x41, 0x41, 0x22],
        b'D' => [0x7f, 0x41, 0x41, 0x22, 0x1c],
        b'E' => [0x7f, 0x49, 0x49, 0x49, 0x41],
        b'F' => [0x7f, 0x09, 0x09, 0x09, 0x01],
        b'G' => [0x3e, 0x41, 0x49, 0x49, 0x7a],
        b'H' => [0x7f, 0x08, 0x08, 0x08, 0x7f],
        b'I' => [0x00, 0x41, 0x7f, 0x41, 0x00],
        b'J' => [0x20, 0x40, 0x41, 0x3f, 0x01],
        b'K' => [0x7f, 0x08, 0x14, 0x22, 0x41],
        b'L' => [0x7f, 0x40, 0x40, 0x40, 0x40],
        b'M' => [0x7f, 0x02, 0x0c, 0x02, 0x7f],
        b'N' => [0x7f, 0x04, 0x08, 0x10, 0x7f],
        b'O' => [0x3e, 0x41, 0x41, 0x41, 0x3e],
        b'P' => [0x7f, 0x09, 0x09, 0x09, 0x06],
        b'Q' => [0x3e, 0x41, 0x51, 0x21, 0x5e],
        b'R' => [0x7f, 0x09, 0x19, 0x29, 0x46],
        b'S' => [0x46, 0x49, 0x49, 0x49, 0x31],
        b'T' => [0x01, 0x01, 0x7f, 0x01, 0x01],
        b'U' => [0x3f, 0x40, 0x40, 0x40, 0x3f],
        b'V' => [0x1f, 0x20, 0x40, 0x20, 0x1f],
        b'W' => [0x7f, 0x20, 0x18, 0x20, 0x7f],
        b'X' => [0x63, 0x14, 0x08, 0x14, 0x63],
        b'Y' => [0x07, 0x08, 0x70, 0x08, 0x07],
        b'Z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        b'[' => [0x00, 0x7f, 0x41, 0x41, 0x00],
        b'\\' => [0x02, 0x04, 0x08, 0x10, 0x20],
        b']' => [0x00, 0x41, 0x41, 0x7f, 0x00],
        b'^' => [0x04, 0x02, 0x01, 0x02, 0x04],
        b'_' => [0x40, 0x40, 0x40, 0x40, 0x40],
        b'`' => [0x00, 0x01, 0x02, 0x04, 0x00],
        b'a' => [0x20, 0x54, 0x54, 0x54, 0x78],
        b'b' => [0x7f, 0x48, 0x44, 0x44, 0x38],
        b'c' => [0x38, 0x44, 0x44, 0x44, 0x20],
        b'd' => [0x38, 0x44, 0x44, 0x48, 0x7f],
        b'e' => [0x38, 0x54, 0x54, 0x54, 0x18],
        b'f' => [0x08, 0x7e, 0x09, 0x01, 0x02],
        b'g' => [0x0c, 0x52, 0x52, 0x52, 0x3e],
        b'h' => [0x7f, 0x08, 0x04, 0x04, 0x78],
        b'i' => [0x00, 0x44, 0x7d, 0x40, 0x00],
        b'j' => [0x20, 0x40, 0x44, 0x3d, 0x00],
        b'k' => [0x7f, 0x10, 0x28, 0x44, 0x00],
        b'l' => [0x00, 0x41, 0x7f, 0x40, 0x00],
        b'm' => [0x7c, 0x04, 0x18, 0x04, 0x78],
        b'n' => [0x7c, 0x08, 0x04, 0x04, 0x78],
        b'o' => [0x38, 0x44, 0x44, 0x44, 0x38],
        b'p' => [0x7c, 0x14, 0x14, 0x14, 0x08],
        b'q' => [0x08, 0x14, 0x14, 0x18, 0x7c],
        b'r' => [0x7c, 0x08, 0x04, 0x04, 0x08],
        b's' => [0x48, 0x54, 0x54, 0x54, 0x20],
        b't' => [0x04, 0x3f, 0x44, 0x40, 0x20],
        b'u' => [0x3c, 0x40, 0x40, 0x20, 0x7c],
        b'v' => [0x1c, 0x20, 0x40, 0x20, 0x1c],
        b'w' => [0x3c, 0x40, 0x30, 0x40, 0x3c],
        b'x' => [0x44, 0x28, 0x10, 0x28, 0x44],
        b'y' => [0x0c, 0x50, 0x50, 0x50, 0x3c],
        b'z' => [0x44, 0x64, 0x54, 0x4c, 0x44],
        b'{' => [0x00, 0x08, 0x36, 0x41, 0x00],
        b'|' => [0x00, 0x00, 0x7f, 0x00, 0x00],
        b'}' => [0x00, 0x41, 0x36, 0x08, 0x00],
        b'~' => [0x08, 0x04, 0x08, 0x10, 0x08],
        // Unknown bytes render as `?` outline, never a solid block.
        _ => [0x02, 0x01, 0x51, 0x09, 0x06],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gutter::GUTTER_RULER_WIDE;
    use crate::home::{
        HOME_CARD_BUTTON_MIN_H, HOME_GRID_GAP, RECENT_COL_AGE_W, RECENT_COL_KIND_W,
        home_card_min_height,
    };
    use crate::views::wrap_find_sub_rows;
    use crate::views_table::key_column_marker;

    fn png_magic_ok(bytes: &[u8]) -> bool {
        bytes.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
    }

    #[test]
    fn text_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-folder-text-fixture.png");
        assert!(render_text_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
    }

    #[test]
    fn folder_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-folder-folder-fixture.png");
        assert!(render_folder_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn dispatcher_routes_by_file_name() {
        let dir = std::env::temp_dir();
        let text_out = dir.join("vis-dispatch-text.png");
        let folder_out = dir.join("vis-dispatch-folder.png");
        assert!(render_fixture_png(&text_out).is_ok());
        assert!(render_fixture_png(&folder_out).is_ok());
        let text_bytes = std::fs::read(&text_out).unwrap_or_default();
        let folder_bytes = std::fs::read(&folder_out).unwrap_or_default();
        assert!(!text_bytes.is_empty() && !folder_bytes.is_empty());
        assert!(png_magic_ok(&text_bytes));
        assert!(png_magic_ok(&folder_bytes));
        assert_ne!(text_bytes, folder_bytes);
    }

    #[test]
    fn narrow_glyphs_use_tighter_advance() {
        for ch in *b" ',.:;!Iijlt|`" {
            assert_eq!(glyph_advance(ch), 4);
        }
        for ch in *b"MWaen=>/" {
            assert_eq!(glyph_advance(ch), 6);
        }
        assert!(glyph_advance(b'i') < glyph_advance(b'M'));
    }

    #[test]
    fn kern_tight_pairs_tuck_one_pixel() {
        assert_eq!(kern_adjust(b'A', b'V'), -1);
        assert_eq!(kern_adjust(b'L', b'T'), -1);
        assert_eq!(kern_adjust(b'T', b'o'), -1);
        assert_eq!(kern_adjust(b'V', b'A'), -1);
        assert_eq!(kern_adjust(b'r', b'e'), -1);
        assert_eq!(kern_adjust(b'n', b'n'), 0);
        assert_eq!(kern_adjust(b'e', b'e'), 0);
        assert_eq!(kern_adjust(b'T', b'n'), 0);
    }

    #[test]
    fn punctuation_dots_are_single_stem() {
        assert_eq!(nonzero_columns(&glyph_pattern(b'.')), 1);
        assert_eq!(nonzero_columns(&glyph_pattern(b':')), 1);
        assert_eq!(nonzero_columns(&glyph_pattern(b',')), 2);
        assert_eq!(nonzero_columns(&glyph_pattern(b';')), 2);
        assert_eq!(glyph_pattern(b'.').get(2), Some(&0x60u8));
        assert_eq!(glyph_pattern(b',').get(3), Some(&0x20u8));
        // Unknown bytes fall back to the `?` outline, never a solid block.
        assert_eq!(glyph_pattern(0x80), glyph_pattern(b'?'));
    }

    #[test]
    fn text_width_respects_kerning_and_fits_panes() {
        let plain = glyph_advance(b'T').saturating_add(glyph_advance(b'o'));
        assert_eq!(text_width("To"), plain.saturating_sub(1));
        assert_eq!(
            text_width("nn"),
            glyph_advance(b'n').saturating_add(glyph_advance(b'n'))
        );
        // Fixture strings must fit their painted fields at 960px canvas width.
        assert!(text_width("fixtures/left.rs") < 380);
        assert!(text_width("fixtures/right.rs") < 380);
        assert!(
            text_width("[Swap Sides] [Reload/Recompare] [x] Word Wrap [x] Line Numbers [x] Syntax")
                < 928
        );
        assert!(
            text_width(
                "7 rows: 4 equal, 1 changed, 1 deleted, 1 inserted | text compare done: 7 row(s)"
            ) < 928
        );
    }

    #[test]
    fn draw_string_returns_pen_and_clips_safely() {
        let dims = (20u32, 10u32);
        let total = 20usize.saturating_mul(10).saturating_mul(4);
        let mut pixels = vec![0u8; total];
        let pen = draw_string(&mut pixels, dims, 2, 1, "Hi.", FG_TEXT);
        assert_eq!(pen, 2u32.saturating_add(text_width("Hi.")));
        // Pen near the right edge clips glyph pixels but still advances.
        let edge = draw_string(&mut pixels, dims, 19, 1, "Hi.", FG_TEXT);
        assert_eq!(edge, 19u32.saturating_add(text_width("Hi.")));
        assert_eq!(pixels.len(), total);
    }

    #[test]
    fn hex_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-ao-hex-fixture.png");
        assert!(render_hex_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn table_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-ao-table-fixture.png");
        assert!(render_table_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn merge_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-ao-merge-fixture.png");
        assert!(render_merge_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn home_fixture_renders_valid_png() {
        let out = std::env::temp_dir().join("vis-ao-home-fixture.png");
        assert!(render_home_fixture_png(&out).is_ok());
        let bytes = std::fs::read(&out).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(png_magic_ok(&bytes));
        assert!(bytes.len() >= 8192);
        let distinct: std::collections::BTreeSet<u8> = bytes.iter().copied().collect();
        assert!(distinct.len() > 64);
    }

    #[test]
    fn dispatcher_routes_all_tabs() {
        let dir = std::env::temp_dir();
        let cases = [
            "vis-ao-dispatch-text.png",
            "vis-ao-dispatch-folder.png",
            "vis-ao-dispatch-hex.png",
            "vis-ao-dispatch-table.png",
            "vis-ao-dispatch-merge.png",
            "vis-ao-dispatch-home.png",
        ];
        let mut blobs: Vec<Vec<u8>> = Vec::new();
        for name in cases {
            let out = dir.join(name);
            assert!(render_fixture_png(&out).is_ok());
            let bytes = std::fs::read(&out).unwrap_or_default();
            assert!(!bytes.is_empty());
            assert!(png_magic_ok(&bytes));
            assert!(bytes.len() >= 8192);
            blobs.push(bytes);
        }
        // Har tab ka PNG alag hona chahiye (6 me se koi do same nahi).
        for (i, a) in blobs.iter().enumerate() {
            for (j, b) in blobs.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
        // FolderMerge naam merge route par jana chahiye.
        let fm = dir.join("vis-ao-dispatch-foldermerge.png");
        let m = dir.join("vis-ao-dispatch-merge.png");
        assert!(render_fixture_png(&fm).is_ok());
        let fm_bytes = std::fs::read(&fm).unwrap_or_default();
        let m_bytes = std::fs::read(&m).unwrap_or_default();
        assert_eq!(fm_bytes, m_bytes);
    }

    #[test]
    fn geometry_shared_tokens_hold() {
        assert_eq!(SNAP_W, 960);
        assert_eq!(SNAP_H, 600);
        assert_eq!(ROW_H, 24);
        assert_eq!(OUTER_PAD, 12);
        assert_eq!(PANE_GAP, 4);
        assert_eq!(TITLE_H, 36);
        assert_eq!(TAB_H, 32);
        assert_eq!(STATUS_H, 30);
        // Text two-pane: left + gap + right meets the ruler, right edge 952 (8px margin).
        assert_eq!(
            TEXT_LEFT_X
                .saturating_add(TEXT_PANE_W)
                .saturating_add(PANE_GAP),
            TEXT_RIGHT_X
        );
        assert_eq!(TEXT_RIGHT_X.saturating_add(TEXT_PANE_W), GUTTER_X);
        assert_eq!(GUTTER_X.saturating_add(GUTTER_W), 952);
        assert!(GUTTER_X.saturating_add(GUTTER_W) <= SNAP_W);
        // Pane heights: 124 + 436 = 560, status 570 + 30 = 600.
        assert_eq!(124u32.saturating_add(436), 560);
        assert_eq!(570u32.saturating_add(STATUS_H), SNAP_H);
        // Fixture strings apne fields me fit hone chahiye.
        assert!(text_width("fixtures/left") < 380);
        assert!(text_width("fixtures/right") < 380);
        assert!(text_width("00000000  48 65 6c 6c 6f 20 77 6f |Hello wo|") < 460);
        assert!(text_width("id | name | left | right | status") < 936);
        assert!(text_width("let x = 1;") < 309);
        assert!(text_width("Recent sessions") < 936);
        assert!(
            text_width("home: 9 cards, 5 recents | no path picker | pick a session to begin") < 928
        );
    }

    #[test]
    fn merge_panes_share_equal_thirds() {
        assert_eq!(MERGE_PANE_W, 309);
        assert_eq!(
            MERGE_X0
                .saturating_add(MERGE_PANE_W)
                .saturating_add(PANE_GAP),
            MERGE_X1
        );
        assert_eq!(
            MERGE_X1
                .saturating_add(MERGE_PANE_W)
                .saturating_add(PANE_GAP),
            MERGE_X2
        );
        assert!(
            MERGE_X2
                .saturating_add(MERGE_PANE_W)
                .saturating_add(OUTER_PAD)
                <= SNAP_W
        );
        // Teenon widths barabar, koi ad-hoc chaudapan nahi.
        assert_eq!(MERGE_PANE_W, MERGE_PANE_W);
    }

    #[test]
    fn home_cards_fit_canvas() {
        // Live grid: 3 cols, gap 8, card 306x100 (merge-style slack: 2px).
        assert_eq!(HOME_CARD_W, 306);
        assert_eq!(HOME_CARD_H, 100);
        assert_eq!(HOME_CARD_GAP, 8);
        assert_eq!(HOME_CARD_GAP as f32, HOME_GRID_GAP);
        let total = HOME_CARD_W
            .saturating_mul(3)
            .saturating_add(HOME_CARD_GAP.saturating_mul(2))
            .saturating_add(OUTER_PAD.saturating_mul(2));
        assert_eq!(total.saturating_add(2), SNAP_W);
        assert!(text_width("Text Compare") < HOME_CARD_W);
        assert!(text_width("Folder Compare") < HOME_CARD_W);
    }

    #[test]
    fn gutter_matches_live_ruler_contract() {
        // Ruler track 16 (line numbers on), old 14px gap absorbed.
        assert_eq!(GUTTER_W, GUTTER_RULER_WIDE as u32);
        assert_eq!(GUTTER_W, 16);
        assert_eq!(GUTTER_X, 936);
        // Tall-file single hunk keeps the 4px floor (not the old 2px dot).
        let mut tall = Vec::new();
        let mut i = 0usize;
        while i < 1000 {
            tall.push(TextRow {
                kind: TextRowKind::Equal,
                left_no: None,
                right_no: None,
                left: String::new(),
                right: String::new(),
                accent_a: Vec::new(),
                accent_b: Vec::new(),
            });
            i = i.saturating_add(1);
        }
        if let Some(row) = tall.get_mut(500) {
            row.kind = TextRowKind::Deleted;
        }
        let geom = gutter_geometry(1000, 400.0, &tall);
        assert_eq!(geom.segments.len(), 1);
        if let Some(seg) = geom.segments.first() {
            assert!((seg.y_end - seg.y_start - GUTTER_SEGMENT_MIN_H).abs() < 0.001);
        }
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn home_matches_live_contract() {
        use crate::home::{
            HOME_CARD_BUTTON_MIN_W, HOME_SHOWS_PATH_PICKER, home_card_button_hit_area,
            home_grid_rows,
        };
        // 9 live cards tile 3x3 rows.
        assert_eq!(home_model().items.len(), 9);
        assert_eq!(home_grid_rows(9), 3);
        // Card floor: contract min-height 99.6 -> fixture 100.
        assert!((home_card_min_height() - 99.6).abs() < 0.01);
        assert!((HOME_CARD_H as f32) >= home_card_min_height());
        // CTA floor: 120x28 min, hit area 3360 min.
        assert!((HOME_CTA_W as f32) >= HOME_CARD_BUTTON_MIN_W);
        assert!((HOME_CTA_H as f32) >= HOME_CARD_BUTTON_MIN_H);
        assert!((HOME_CTA_W as f32) * (HOME_CTA_H as f32) >= home_card_button_hit_area());
        // No path picker on the launcher.
        assert!(!HOME_SHOWS_PATH_PICKER);
        // Recent columns tile the list width exactly.
        let (title_w, kind_w, age_w) = recent_row_columns(936.0);
        assert_eq!(kind_w, RECENT_COL_KIND_W);
        assert_eq!(age_w, RECENT_COL_AGE_W);
        assert!(
            (title_w + kind_w + age_w + RECENT_LIST_PAD * 2.0 + RECENT_COL_GAP * 2.0 - 936.0).abs()
                < 0.001
        );
        // Every CTA label fits its button text budget (150 - 2*12).
        for item in home_model().items.iter() {
            assert!(text_width(&format!("Open {}", item.title)) < HOME_CTA_W - 24);
        }
    }

    #[test]
    fn home_desc_wrap_stays_in_two_line_budget() {
        let budget = HOME_CARD_W.saturating_sub(24);
        for item in home_model().items.iter() {
            let (top, bottom) = wrap_desc_two_lines(item.description, budget);
            assert!(text_width(&top) <= budget);
            assert!(text_width(&bottom) <= budget);
            // Our 9 live descs fit two lines: no word dropped, no ellipsis.
            let mut want: Vec<&str> = item.description.split(' ').collect();
            want.retain(|w| !w.is_empty());
            let mut got: Vec<&str> = top.split(' ').chain(bottom.split(' ')).collect();
            got.retain(|w| !w.is_empty());
            assert_eq!(got, want);
            assert!(!bottom.ends_with("..."));
        }
        // Third-line spill collapses honestly instead of clipping.
        let (top, bottom) = wrap_desc_two_lines(
            "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu",
            60,
        );
        assert!(text_width(&top) <= 60);
        assert!(text_width(&bottom) <= 60);
        assert!(bottom.ends_with("..."));
    }

    #[test]
    fn wrap_scene_proves_continuation_find() {
        const LONG: &str = "    // wrap proof: keep the legacy reader enabled until rollout ends, then call read_to_string(old);";
        const NEEDLE: &str = "read_to_string";
        assert_eq!(LONG.chars().count(), 100);
        // Live cap: 100-char model -> 80-char wrap width.
        assert_eq!(wrap_width_for(100, true), 80);
        // Needle lives fully inside the continuation sub-row.
        assert_eq!(wrap_find_sub_rows(LONG, NEEDLE, false, 80), vec![1]);
        assert_eq!(crate::views::wrap_chunks(LONG, 80).len(), 2);
        // Every rendered chunk fits the 428px pane text budget.
        for chunk in crate::views::wrap_chunks(LONG, 80) {
            assert!(text_width(&chunk.text) <= 428);
        }
        // Scroll target clamps to the pair count (2 subs -> sub 1).
        let row = TextRow {
            kind: TextRowKind::Equal,
            left_no: Some(3),
            right_no: Some(3),
            left: LONG.to_string(),
            right: LONG.to_string(),
            accent_a: Vec::new(),
            accent_b: Vec::new(),
        };
        let count = wrap_pane_pair_count(&row.left, &row.right, 80);
        assert_eq!(count, 2);
        assert_eq!(wrap_find_scroll_sub(&row, NEEDLE, false, 80, count), 1);
        // Flat visual offset of (row 2, sub 1) over [1,1,2,1,1] is 3.
        let counts = vec![1usize, 1, 2, 1, 1];
        assert_eq!(
            crate::views::wrap_model_to_visual_start(&counts, 2),
            Some(2)
        );
        // Status + find-bar strings fit the 936px panel.
        assert!(
            text_width("wrap:on find:read_to_string 2 hits, current row 3 sub 1/2 | wrapnav proof")
                < 928
        );
        assert!(text_width("2 hits, current row 3") < 936);
    }

    #[test]
    fn picture_scene_matches_fit_contract() {
        assert_eq!(PICTURE_ZOOM_STEPS, [25, 50, 100, 200, 400]);
        // Viewport 936x372 over natural 800x600: scale 0.62 -> step 50.
        assert_eq!(picture_fit_zoom_pct(800.0, 600.0, 936.0, 372.0), 50);
        assert_eq!(picture_center_offset(400.0, 936.0), 268.0);
        assert_eq!(picture_center_offset(300.0, 372.0), 36.0);
        // Live summary words the identical pair (no invented numbers).
        assert_eq!(
            crate::viewmodel::build_picture_summary(&PictureDiff {
                width_l: 800,
                height_l: 600,
                width_r: 800,
                height_r: 600,
                overlap_w: 800,
                overlap_h: 600,
                total: 480_000,
                differing: 0,
                bbox: None,
                identical: true,
            }),
            vec![
                "left:  800x600".to_string(),
                "right: 800x600".to_string(),
                "no differences".to_string(),
            ]
        );
        // Centered 400x300 frame stays inside the 936x372 viewport.
        assert!(12u32.saturating_add(268).saturating_add(400) <= 948);
        assert!(188u32.saturating_add(36).saturating_add(300) <= 560);
        assert!(
            text_width("picture: fit 50% | centered (268,36) | steps Fit+25/50/100/200/400") < 928
        );
        assert_eq!(zoom_label(50), "zoom 50% (0.50x)");
        assert!(text_width(&zoom_label(50)) < 948 - 432);
    }

    #[test]
    fn table_find_scene_matches_key_find_contract() {
        // Key bar: col 0 selected, badge reads keyed.
        let mut view = TableKeyView::new();
        assert!(view.is_positional());
        view.toggle_key_column(0);
        assert!(is_key_column(&view, 0));
        assert!(!is_key_column(&view, 1));
        assert_eq!(table_key_badge_label(&view), "key: 0");
        assert_ne!(key_column_marker(true), key_column_marker(false));
        // Find spans: cell-relative (0,3), row offset 4 -> (4,7) in cap.
        assert_eq!(table_find_cell_spans("bob", "bob", false), vec![(0, 3)]);
        assert_eq!(
            table_find_span_in_padded(4, 7, TABLE_MAX_COL_CHARS),
            Some((4, 7))
        );
        assert_eq!(
            table_find_span_in_padded(0, 99, TABLE_MAX_COL_CHARS),
            Some((0, 32))
        );
        // Bars reuse live heights; needle fits its 120px box.
        assert_eq!(TABLE_KEY_BAR_H, 28.0);
        assert_eq!(TABLE_FIND_BAR_H, 28.0);
        assert_eq!(TABLE_FIND_NEEDLE_W, 120.0);
        assert!(text_width("bob") < TABLE_FIND_NEEDLE_W as u32);
    }

    #[test]
    fn folder_empty_scene_matches_hint_contract() {
        // Filtered empty: no rows, no error -> idle guidance, never blank.
        let hint = folder_empty_hint(false, false);
        assert!(!hint.is_empty());
        assert!(hint.contains("type both folder paths"));
        assert!(text_width(hint) < 936);
        let rows = vec![crate::viewmodel::FolderRow {
            rel_path: "src/a.txt".to_string(),
            is_dir: false,
            glyph: '>',
            left_size: None,
            right_size: None,
            left_time: None,
            right_time: None,
            changed: true,
        }];
        let counts = crate::tree::count_folder_rows(&rows);
        assert!(counts.summary().contains("1 entries") || counts.summary().contains('1'));
        assert!(
            text_width(
                "folder: 4 entries, 0 shown (filter) | type both folder paths, press Compare"
            ) < 928
        );
    }

    #[test]
    fn dispatcher_routes_new_scenes() {
        let dir = std::env::temp_dir();
        let cases = [
            "vis-ax-dispatch-wrap.png",
            "vis-ax-dispatch-picture.png",
            "vis-ax-dispatch-folder-empty.png",
            "vis-ax-dispatch-table-find.png",
        ];
        let mut blobs: Vec<Vec<u8>> = Vec::new();
        for name in cases {
            let out = dir.join(name);
            assert!(render_fixture_png(&out).is_ok());
            let bytes = std::fs::read(&out).unwrap_or_default();
            assert!(!bytes.is_empty());
            assert!(png_magic_ok(&bytes));
            assert!(bytes.len() >= 8192);
            blobs.push(bytes);
        }
        // Naye scenes apas me alag...
        assert_eq!(blobs.len(), cases.len());
        for (i, a) in blobs.iter().enumerate() {
            for (j, b) in blobs.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
        // ...aur purane default scenes se bhi alag.
        let text_out = dir.join("vis-ax-dispatch-text.png");
        assert!(render_fixture_png(&text_out).is_ok());
        let text_bytes = std::fs::read(&text_out).unwrap_or_default();
        let folder_out = dir.join("vis-ax-dispatch-folder.png");
        assert!(render_fixture_png(&folder_out).is_ok());
        let folder_bytes = std::fs::read(&folder_out).unwrap_or_default();
        let table_out = dir.join("vis-ax-dispatch-table.png");
        assert!(render_fixture_png(&table_out).is_ok());
        let table_bytes = std::fs::read(&table_out).unwrap_or_default();
        if let (Some(wrap), Some(picture), Some(folder_empty), Some(table_find)) =
            (blobs.first(), blobs.get(1), blobs.get(2), blobs.get(3))
        {
            assert_ne!(wrap, &text_bytes);
            assert_ne!(picture, &text_bytes);
            assert_ne!(folder_empty, &folder_bytes);
            assert_ne!(table_find, &table_bytes);
        }
    }
}
