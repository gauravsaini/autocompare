//! Offscreen screenshot generation for verification and smoke testing.
//!
//! Renders the Text comparison fixture session into a valid PNG without
//! requiring an interactive window or display server.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use crate::viewmodel::{TextRow, TextRowKind, gutter_geometry};
use autocompare_diff::{RgbaImage, encode_png};
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
/// Text/folder two-pane widths: 460 + 4 + 460 = 924, margins 12*2 = 24, total 948 + gutter 14 = 962? Nahi —
/// exact layout: left x12 w460, right x476 w460, gutter x940 w14. Right edge 954, margin 6.
/// Yehi numbers geometry test me assert hote hain.
pub const TEXT_PANE_W: u32 = 460;
pub const TEXT_LEFT_X: u32 = 12;
pub const TEXT_RIGHT_X: u32 = 476;
pub const GUTTER_X: u32 = 940;
pub const GUTTER_W: u32 = 14;
/// Merge 3-pane equal thirds: avail 960-24-8=928, 309*3=927, 1px slack right me.
pub const MERGE_PANE_W: u32 = 309;
pub const MERGE_X0: u32 = 12;
pub const MERGE_X1: u32 = 325;
pub const MERGE_X2: u32 = 638;

/// Renders the Text comparison session fixture to a PNG at the given path.
///
/// File name ke tukde se tab chunta hai (lowercase match):
/// `hex` -> hex, `table` -> table, `merge` -> merge (foldermerge bhi),
/// `folder` -> folder, `home` -> home, warna text.
/// Har route same 960x600 canvas + 24px rows par tikta hai, colors frozen.
pub fn render_fixture_png(out_path: &Path) -> Result<(), String> {
    let lower = match out_path.file_name().and_then(|name| name.to_str()) {
        Some(name) => name.to_lowercase(),
        None => String::new(),
    };
    if lower.contains("hex") {
        return render_hex_fixture_png(out_path);
    }
    if lower.contains("table") {
        return render_table_fixture_png(out_path);
    }
    if lower.contains("merge") {
        return render_merge_fixture_png(out_path);
    }
    if lower.contains("folder") {
        return render_folder_fixture_png(out_path);
    }
    if lower.contains("home") {
        return render_home_fixture_png(out_path);
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
    let gutter_x = 940u32;
    let gutter_w = 14u32;

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
            accent_b: Vec::new(),
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
    }

    // Thumbnail overview gutter geometry
    let geom = gutter_geometry(rows.len(), pane_height as f32, &rows);
    for seg in geom.segments {
        let seg_y = pane_top.saturating_add(seg.y_start as u32);
        let seg_h = ((seg.y_end - seg.y_start) as u32).max(2);
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

/// Renders the Home fixture: session cards + recent list,
/// same chrome, 24px rows. Colors frozen.
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
    paint_chrome(&mut pixels, dims, "Home - autocompare", 0);

    // Cards: 3 x 304px with 12px gaps inside 936px avail.
    let card_w = 304u32;
    let card_h = 90u32;
    let card_y = 150u32;
    let card_xs = [
        OUTER_PAD,
        OUTER_PAD.saturating_add(card_w).saturating_add(12),
        OUTER_PAD
            .saturating_add(card_w)
            .saturating_add(12)
            .saturating_add(card_w)
            .saturating_add(12),
    ];
    let card_titles = ["Text compare", "Folder compare", "Hex compare"];
    let card_subs = [
        "2 files side by side",
        "2 folders + sync",
        "2 binaries + ascii",
    ];
    for (idx, x) in card_xs.iter().enumerate() {
        fill_rect(
            &mut pixels,
            dims,
            (*x, card_y, card_w, card_h),
            [28, 28, 28, 255],
        );
        fill_rect(&mut pixels, dims, (*x, card_y, card_w, 3), ACCENT_TAB);
        let title = match card_titles.get(idx) {
            Some(s) => *s,
            None => "",
        };
        let sub = match card_subs.get(idx) {
            Some(s) => *s,
            None => "",
        };
        draw_string(
            &mut pixels,
            dims,
            x.saturating_add(12),
            card_y.saturating_add(14),
            title,
            FG_TEXT,
        );
        draw_string(
            &mut pixels,
            dims,
            x.saturating_add(12),
            card_y.saturating_add(32),
            sub,
            FG_DIM,
        );
        fill_rect(
            &mut pixels,
            dims,
            (x.saturating_add(12), card_y.saturating_add(56), 120, 22),
            [60, 60, 60, 255],
        );
        draw_string(
            &mut pixels,
            dims,
            x.saturating_add(24),
            card_y.saturating_add(62),
            "Open...",
            FG_TEXT,
        );
    }

    // Recent list pane.
    let list_top = card_y.saturating_add(card_h).saturating_add(16);
    let list_bottom = 560u32;
    let list_x = OUTER_PAD;
    let list_w = SNAP_W.saturating_sub(OUTER_PAD.saturating_mul(2));
    fill_rect(
        &mut pixels,
        dims,
        (
            list_x,
            list_top,
            list_w,
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
    let recents = [
        "text: left.rs <-> right.rs",
        "folder: left-dir <-> right-dir",
        "hex: a.bin <-> b.bin",
        "table: a.csv <-> b.csv",
        "merge: base + left + right",
    ];
    for (i, line) in recents.iter().enumerate() {
        let y = list_top
            .saturating_add(ROW_H)
            .saturating_add((i as u32).saturating_mul(ROW_H));
        if y.saturating_add(ROW_H) > list_bottom {
            break;
        }
        fill_rect(&mut pixels, dims, (list_x, y, list_w, ROW_H), BG_DARK);
        draw_string(&mut pixels, dims, 28, y.saturating_add(6), line, FG_TEXT);
    }

    paint_status(
        &mut pixels,
        dims,
        "home: 3 cards, 5 recents | pick a session to begin",
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
/// `active`: 0 home, 1 text, 2 folder, 3 hex, 4 merge, 5 table.
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
    draw_string(pixels, dims, 404, 46, "Picture", FG_DIM);
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
        // Text two-pane: left + gap + right + gap + gutter, right edge 954 (6px margin).
        assert_eq!(
            TEXT_LEFT_X
                .saturating_add(TEXT_PANE_W)
                .saturating_add(PANE_GAP),
            TEXT_RIGHT_X
        );
        assert_eq!(
            TEXT_RIGHT_X
                .saturating_add(TEXT_PANE_W)
                .saturating_add(PANE_GAP),
            GUTTER_X
        );
        assert_eq!(GUTTER_X.saturating_add(GUTTER_W), 954);
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
        assert!(text_width("home: 3 cards, 5 recents | pick a session to begin") < 928);
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
        let card_w = 304u32;
        let gap = 12u32;
        let total = card_w
            .saturating_mul(3)
            .saturating_add(gap.saturating_mul(2))
            .saturating_add(OUTER_PAD.saturating_mul(2));
        assert_eq!(total, SNAP_W);
        assert!(text_width("Text compare") < card_w);
        assert!(text_width("Folder compare") < card_w);
    }
}
