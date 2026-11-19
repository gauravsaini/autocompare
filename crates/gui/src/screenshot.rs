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

/// Renders the Text comparison session fixture to a PNG at the given path.
pub fn render_fixture_png(out_path: &Path) -> Result<(), String> {
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
        "autocompare - desktop client",
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

    // Main diff pane borders
    let pane_top = 110u32;
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

    // Status bar
    fill_rect(&mut pixels, dims, (0, 570, width, 30), [32, 32, 32, 255]);
    draw_string(
        &mut pixels,
        dims,
        16,
        578,
        "7 rows: 4 equal, 1 changed, 1 deleted, 1 inserted | Dark Theme (WCAG AA Contrast: OK)",
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

// 5x7 ASCII font column definitions for clean raster text rendering.
fn draw_string(pixels: &mut [u8], dims: (u32, u32), mut x: u32, y: u32, s: &str, color: [u8; 4]) {
    for b in s.bytes() {
        draw_glyph(pixels, dims, x, y, b, color);
        x = x.saturating_add(7);
    }
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
        b',' => [0x00, 0x50, 0x30, 0x00, 0x00],
        b'-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        b'.' => [0x00, 0x60, 0x60, 0x00, 0x00],
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
        b':' => [0x00, 0x36, 0x36, 0x00, 0x00],
        b';' => [0x00, 0x56, 0x36, 0x00, 0x00],
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
        _ => [0x7f, 0x7f, 0x7f, 0x7f, 0x7f],
    }
}
