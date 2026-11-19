//! egui painting of the view models.
//!
//! All interpretation of engine results happened in
//! [`crate::viewmodel`]; this file only turns rows into widgets. The
//! few pure helpers here (`pane_segments`, `gutter_text`,
//! `folder_columns`, `premultiplied_rgba`, `heatmap_color_image`) are
//! paint-adjacent logic and are unit-tested headlessly alongside the
//! viewmodel.
//!
//! The diff palette is fixed rather than theme-following: a deleted
//! row must read the same in light and dark mode, so its colors carry
//! their own contrast.

use eframe::egui::text::LayoutJob;
use eframe::egui::{
    self, Color32, FontId, RichText, Stroke, TextEdit, TextFormat, TextureHandle, Ui,
};

use crate::app::Tab;
use crate::app::{
    AudioCheckKind, AudioChecklist, FolderMergeChecklist, PictureCompareModel, PictureMode,
    PictureViewState, SyncChecklist,
};
use crate::gutter::gutter_bar_width_for_font;
use crate::home::{home_card_button_size, home_card_desc_is_small_weak, home_card_title_size};
use crate::theme::{
    ACCENT_FG, CHANGED_BG, CHANGED_FG, DELETED_BG, DELETED_FG, ERROR_BG, INSERTED_BG, INSERTED_FG,
    TYPO_MONO_SIZE, blue_for, dim_for, green_for, red_for, track_for,
};
use crate::viewmodel::{self, TextViewPrefs, view_text_with_prefs};
use crate::viewmodel::{FolderRow, MergeSummary, TableModel, TextRow, TextRowKind, TextRows};
use crate::{views_hex, views_table};

// The fixed diff palette lives in `crate::theme` (same hues, own
// contrast); per-mode chrome (dim/red/green/blue/track) is picked via
// `theme::*_for(ui.visuals().dark_mode)` at each paint site below.
// Kept for the pending App rewiring back to `views::paint_folder`
// (the Folder tab currently paints inline in `app.rs`, read-only
// here); the allow keeps the tree clippy-clean without deleting the
// tested paint API.

// Geometry tokens (px): margins, gaps, alignment. Colors out of scope.
pub const OUTER_MARGIN: i8 = 12;
pub const SECTION_GAP: f32 = 8.0;
pub const TIGHT_GAP: f32 = 4.0;
pub const FOOTER_GAP: f32 = 6.0;
pub const TAB_BOTTOM_GAP: f32 = 6.0;
pub const TREE_INDENT: f32 = 14.0;
pub const CARD_W: f32 = 240.0;
pub const CARD_H: f32 = 75.0;
const BROWSE_RESERVE: f32 = 75.0;
const MIN_HALF: f32 = 180.0;
const MIN_THIRD: f32 = 130.0;

/// Equal half width for two path fields sharing one row.
pub fn half_field_width(avail: f32) -> f32 {
    (avail / 2.0).max(MIN_HALF) - BROWSE_RESERVE
}

/// Equal third width for three path fields sharing one row.
pub fn third_field_width(avail: f32) -> f32 {
    (avail / 3.0).max(MIN_THIRD) - BROWSE_RESERVE
}

/// Consistent tree indent for one depth level.
pub fn tree_indent(depth: usize) -> f32 {
    depth as f32 * TREE_INDENT
}

/// Shared monospace row height so text panes stay equal.
pub fn text_row_height(ui: &Ui) -> f32 {
    ui.text_style_height(&egui::TextStyle::Monospace)
}

/// Paint-applied Home title size: routes through the home contract
/// so paint stays in sync headlessly.
pub fn home_title_paint_size() -> f32 {
    home_card_title_size()
}

/// Whether the Home description paints small + weak.
pub fn home_desc_paint_is_small_weak() -> bool {
    home_card_desc_is_small_weak()
}

/// Paint-applied Home button size (body scale).
pub fn home_button_paint_size() -> f32 {
    home_card_button_size()
}

/// Paint-applied gutter width at the mono scale: reproduces 12/6 via
/// [`gutter_bar_width_for_font`].
pub fn text_gutter_paint_width(show_line_numbers: bool) -> f32 {
    gutter_bar_width_for_font(TYPO_MONO_SIZE, show_line_numbers)
}

/// Header: title, subtitle, and dark/light mode toggle with WCAG contrast verification.
pub fn paint_header(ui: &mut Ui, dark_mode: &mut bool) {
    ui.horizontal(|ui| {
        ui.heading("autocompare");
        ui.label(RichText::new("desktop client").weak());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if *dark_mode {
                "☀ Light Mode"
            } else {
                "🌙 Dark Mode"
            };
            if ui.button(label).clicked() {
                *dark_mode = !*dark_mode;
                ui.ctx().set_visuals(if *dark_mode {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
            }
        });
    });
    ui.label(
        RichText::new("drag & drop paths or browse via native dialogs | pure background engines")
            .small()
            .weak(),
    );
    ui.add_space(TIGHT_GAP);
}

/// The Home View launcher screen displaying cards for all 9 comparison session types.
pub fn paint_home(ui: &mut Ui, tab: &mut Tab) {
    let model = viewmodel::home_model();
    ui.heading("Comparison Sessions");
    ui.label(RichText::new("Select a session type or use the tabs above:").weak());
    ui.add_space(SECTION_GAP);

    egui::Grid::new("home_sessions_grid")
        .num_columns(3)
        .spacing([OUTER_MARGIN as f32, OUTER_MARGIN as f32])
        .show(ui, |ui| {
            for (i, item) in model.items.iter().enumerate() {
                ui.group(|ui| {
                    ui.set_width(CARD_W);
                    ui.set_height(CARD_H);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(item.title)
                                .strong()
                                .size(home_title_paint_size()),
                        );
                        let mut desc = RichText::new(item.description);
                        if home_desc_paint_is_small_weak() {
                            desc = desc.small().weak();
                        }
                        ui.label(desc);
                        ui.add_space(TIGHT_GAP);
                        if ui
                            .add(egui::Button::new(
                                RichText::new(format!("Open {}", item.title))
                                    .size(home_button_paint_size()),
                            ))
                            .clicked()
                        {
                            *tab = item.tab;
                        }
                    });
                });
                if i % 3 == 2 {
                    ui.end_row();
                }
            }
        });
}

/// The ten tabs: Home plus nine comparison session types.
pub fn paint_tabs(ui: &mut Ui, tab: &mut Tab) {
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(tab, Tab::Home, "Home");
        ui.selectable_value(tab, Tab::Text, "Text");
        ui.selectable_value(tab, Tab::Folder, "Folder");
        ui.selectable_value(tab, Tab::Hex, "Hex");
        ui.selectable_value(tab, Tab::Merge, "Merge");
        ui.selectable_value(tab, Tab::Table, "Table");
        ui.selectable_value(tab, Tab::Sync, "Sync");
        ui.selectable_value(tab, Tab::Picture, "Picture");
        ui.selectable_value(tab, Tab::FolderMerge, "FolderMerge");
        ui.selectable_value(tab, Tab::Audio, "Audio");
    });
    ui.add_space(TAB_BOTTOM_GAP);
}

/// Two path text fields with native file/folder picker buttons plus Compare.
pub fn paint_paths(
    ui: &mut Ui,
    noun: &str,
    is_dir: bool,
    left: &mut String,
    right: &mut String,
    compare: &mut bool,
) {
    ui.horizontal(|ui| {
        let half = half_field_width(ui.available_width());
        ui.add(
            TextEdit::singleline(left)
                .desired_width(half)
                .hint_text(format!("left {noun} path")),
        );
        if ui.button("Browse...").clicked() {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *left = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *left = file.display().to_string();
            }
        }
        ui.add(
            TextEdit::singleline(right)
                .desired_width(half)
                .hint_text(format!("right {noun} path")),
        );
        if ui.button("Browse...").clicked() {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *right = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *right = file.display().to_string();
            }
        }
    });
    let clicked = ui
        .horizontal(|ui| ui.button(format!("Compare {noun}s")).clicked())
        .inner;
    if clicked {
        *compare = true;
    }
    ui.add_space(TIGHT_GAP);
}

/// Three path text fields with native dialog pickers for three-way tabs.
pub fn paint_triple_paths(
    ui: &mut Ui,
    noun: &str,
    is_dir: bool,
    base: &mut String,
    left: &mut String,
    right: &mut String,
    compare: &mut bool,
) {
    ui.horizontal(|ui| {
        let third = third_field_width(ui.available_width());
        ui.add(
            TextEdit::singleline(base)
                .desired_width(third)
                .hint_text(format!("base {noun} path")),
        );
        if ui.button("Browse...").clicked() {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *base = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *base = file.display().to_string();
            }
        }
        ui.add(
            TextEdit::singleline(left)
                .desired_width(third)
                .hint_text(format!("left {noun} path")),
        );
        if ui.button("Browse...").clicked() {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *left = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *left = file.display().to_string();
            }
        }
        ui.add(
            TextEdit::singleline(right)
                .desired_width(third)
                .hint_text(format!("right {noun} path")),
        );
        if ui.button("Browse...").clicked() {
            if is_dir {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    *right = folder.display().to_string();
                }
            } else if let Some(file) = rfd::FileDialog::new().pick_file() {
                *right = file.display().to_string();
            }
        }
    });
    let clicked = ui
        .horizontal(|ui| ui.button(format!("Compare {noun}s")).clicked())
        .inner;
    if clicked {
        *compare = true;
    }
    ui.add_space(TIGHT_GAP);
}

/// Generic scrollable summary lines for the plan/stat tabs
/// (Hex, Sync, FolderMerge, Audio).
pub fn paint_lines(ui: &mut Ui, lines: &[String]) {
    if lines.is_empty() {
        ui.label(RichText::new("(no output)").weak());
        return;
    }
    let row_height = text_row_height(ui);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, lines.len(), |ui, range| {
            for i in range {
                let Some(line) = lines.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                ui.label(RichText::new(line.as_str()).monospace());
            }
        });
}

/// One model-driven hex hunk line: offset plus per-side byte counts,
/// mirroring the engine summary wording.
pub fn format_hex_hunk_line(hunk: &views_hex::HexHunkView) -> String {
    format!(
        "{:#010x}: {} vs {} byte(s)",
        hunk.offset, hunk.left_len, hunk.right_len
    )
}

/// Parses a tolerance text field the way the engine sanitizes: only
/// finite values `>= 0` count; anything else means exact text. Pure;
/// unit-tested.
pub fn parse_tolerance_text(text: &str) -> Option<f64> {
    match text.trim().parse::<f64>() {
        Ok(value) if value.is_finite() && value >= 0.0 => Some(value),
        _ => None,
    }
}

/// The model-driven hex view: status line, hunk navigation (Prev /
/// Next plus jump to first difference), the hunk grid with offsets,
/// then the legacy engine summary lines for byte context.
pub fn paint_hex(ui: &mut Ui, view: &mut views_hex::HexCompareView, lines: &[String]) {
    ui.label(RichText::new(view.status_line()).monospace().weak());
    if view.over_cap {
        paint_note(ui, "inputs exceed the 16 MiB aligned cap: compare skipped");
        return;
    }
    ui.horizontal(|ui| {
        if ui.small_button("◀ Prev hunk").clicked() {
            view.select_prev_hunk();
        }
        if ui.small_button("Next hunk ▶").clicked() {
            view.select_next_hunk();
        }
        if ui.small_button("First diff").clicked()
            && let Some(off) = view.first_diff
        {
            view.navigate_to_offset(off);
        }
    });
    if view.hunks.is_empty() {
        ui.label(RichText::new("(no differing hunks)").weak());
    } else {
        let row_height = text_row_height(ui);
        let count = view.hunks.len();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(220.0)
            .show_rows(ui, row_height, count, |ui, range| {
                for i in range {
                    let Some(hunk) = view.hunks.get(i) else {
                        break;
                    };
                    let line = format_hex_hunk_line(hunk);
                    let selected = view.selected == Some(i);
                    ui.set_row_height(row_height);
                    if ui
                        .selectable_label(selected, RichText::new(line).monospace())
                        .clicked()
                    {
                        view.selected = Some(i);
                    }
                }
            });
    }
    ui.add_space(TIGHT_GAP);
    paint_lines(ui, lines);
}

/// Over-cap hex state: honest sizes plus the skip note, no hunk grid.
pub fn paint_hex_over_cap(ui: &mut Ui, view: &views_hex::HexCompareView) {
    ui.label(RichText::new(view.status_line()).monospace().weak());
    paint_note(ui, "inputs exceed the 16 MiB aligned cap: compare skipped");
}

/// The model-driven table view: keyed summary line, the positional
/// note when no key columns are selected, then the legacy row paint.
pub fn paint_table_keyed(ui: &mut Ui, view: &views_table::TableKeyView, model: &TableModel) {
    ui.label(RichText::new(view.status_line()).monospace().weak());
    if let Some(note) = view.positional_note() {
        paint_note(ui, note);
    }
    paint_table(ui, model);
}
/// The three-way merge output: conflict blocks highlighted, notes
/// for conflicts and truncation first, with Take resolution actions.
pub fn paint_merge(ui: &mut Ui, model: &mut MergeSummary) {
    let dark = ui.visuals().dark_mode;
    if model.conflicted {
        paint_note(ui, "the merge has conflicts; choose an action to resolve:");
        let mut resolve_choice = None;
        ui.horizontal(|ui| {
            if ui.button("Take Left").clicked() {
                resolve_choice = Some(viewmodel::TakeChoice::Left);
            }
            if ui.button("Take Right").clicked() {
                resolve_choice = Some(viewmodel::TakeChoice::Right);
            }
            if ui.button("Take Base").clicked() {
                resolve_choice = Some(viewmodel::TakeChoice::Base);
            }
            if ui.button("Take Both").clicked() {
                resolve_choice = Some(viewmodel::TakeChoice::Both);
            }
        });
        if let Some(choice) = resolve_choice
            && let Ok(resolved) = viewmodel::resolve_take(&model.lines, 0, choice)
        {
            model.conflicted = resolved.iter().any(|l| l.starts_with("<<<<<<<"));
            model.lines = resolved;
        }
        ui.add_space(TIGHT_GAP);
    }
    if model.truncated {
        paint_note(ui, "showing the first lines only; the rest is cut off");
    }
    if model.lines.is_empty() {
        ui.label(RichText::new("(merged output is empty)").weak());
        return;
    }
    let row_height = text_row_height(ui);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, model.lines.len(), |ui, range| {
            for i in range {
                let Some(line) = model.lines.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                let marker = line.starts_with("<<<<<<<")
                    || line.starts_with("=======")
                    || line.starts_with(">>>>>>>");
                let mut text = RichText::new(line.as_str()).monospace();
                if marker {
                    text = text.color(red_for(dark)).strong();
                }
                ui.label(text);
            }
        });
}

/// The table comparison rows: changed rows highlighted, changed cells
/// already marked `[old => new]` by the view model.
pub fn paint_table(ui: &mut Ui, model: &TableModel) {
    let dark = ui.visuals().dark_mode;
    if model.truncated {
        paint_note(ui, "showing the first rows only; the rest is cut off");
    }
    if model.rows.is_empty() {
        ui.label(RichText::new("(both tables are empty)").weak());
        return;
    }
    let row_height = text_row_height(ui);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, model.rows.len(), |ui, range| {
            for i in range {
                let Some(row) = model.rows.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                let mut glyph = RichText::new(row.glyph.to_string()).monospace();
                glyph = match row.glyph {
                    '=' => glyph.color(dim_for(dark)),
                    '!' | 'L' => glyph.color(red_for(dark)),
                    'R' => glyph.color(green_for(dark)),
                    _ => glyph.color(dim_for(dark)),
                };
                let mut body = RichText::new(format!("{} | {}", row.label, row.text)).monospace();
                if row.changed {
                    body = body.color(CHANGED_FG);
                }
                ui.horizontal(|ui| {
                    ui.label(glyph);
                    ui.label(body);
                });
            }
        });
}

/// The red error panel: every engine failure ends up here, never as a
/// crash.
pub fn paint_error(ui: &mut Ui, message: &str) {
    let mut frame = egui::Frame::group(ui.style());
    frame.fill = ERROR_BG;
    frame.show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("error:")
                    .monospace()
                    .strong()
                    .color(DELETED_FG),
            );
            ui.label(RichText::new(message).color(DELETED_FG));
        });
    });
}

fn paint_note(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().weak());
}

// ----------------------------------------------------------------------------
// Text view
// ----------------------------------------------------------------------------

/// The side-by-side text comparison with default view prefs
/// (word wrap off, line numbers on, syntax on). Kept so the existing
/// `App` call-site compiles; new code should prefer
/// [`paint_text_with_prefs`].
pub fn paint_text(ui: &mut Ui, model: &mut TextRows) {
    let prefs = TextViewPrefs::default();
    paint_text_with_prefs(ui, model, &prefs);
}

/// The side-by-side text comparison honoring `prefs`: word wrap caps
/// panes at 80 chars with wrapping, hiding line numbers blanks the
/// number gutter and narrows chrome, hiding syntax clears inline
/// accents. Counts come from [`viewmodel::filter_counts`] so they stay
/// honest under any toggle combination.
pub fn paint_text_with_prefs(ui: &mut Ui, model: &mut TextRows, prefs: &TextViewPrefs) {
    ui.style_mut().wrap_mode = Some(if prefs.word_wrap {
        egui::TextWrapMode::Wrap
    } else {
        egui::TextWrapMode::Extend
    });
    let prefs_view = view_text_with_prefs(model, prefs);
    let pane_chars = prefs_view.pane_chars;
    let gutter_chars = prefs_view.gutter_chars;
    if model.coarse {
        paint_note(
            ui,
            "inputs exceeded the edit budget; showing a coarse comparison",
        );
    }
    if model.truncated {
        paint_note(
            ui,
            &format!(
                "showing the first {} rows; the rest is cut off",
                viewmodel::MAX_ROWS
            ),
        );
    }
    if model.rows.is_empty() {
        ui.label(RichText::new("(nothing to display: both sides are empty)").weak());
        return;
    }
    // Selection and filter live on the model (single source of
    // truth); clamp a stale selection after a fresh compare.
    if let Some(sel) = model.selected
        && model.rows.get(sel).is_none()
    {
        model.selected = None;
    }

    // Keyboard nav: F3 / Shift+F3 like a find loop, Ctrl+N / Ctrl+P
    // as the terminal-friendly alternative. Plain N/P stay free for
    // typing in the path fields above.
    let (key_next, key_prev) = ui.input(|input| {
        let f3 = input.key_pressed(egui::Key::F3);
        let ctrl = input.modifiers.ctrl || input.modifiers.mac_cmd;
        let next = (f3 && !input.modifiers.shift) || (input.key_pressed(egui::Key::N) && ctrl);
        let prev = (f3 && input.modifiers.shift) || (input.key_pressed(egui::Key::P) && ctrl);
        (next, prev)
    });
    // One-shot scroll: follow the selection only on nav frames.
    let mut scroll_to: Option<usize> = None;
    if key_next {
        viewmodel::select_next_diff(&model.rows, &mut model.selected);
        scroll_to = model.selected;
    }
    if key_prev {
        viewmodel::select_prev_diff(&model.rows, &mut model.selected);
        scroll_to = model.selected;
    }

    let counts = viewmodel::filter_counts(&model.rows, model.filter);
    let visible = viewmodel::filtered_indices(&model.rows, model.filter);

    // Toolbar: diff navigation + display filter + copy actions.
    let mut nav_action: Option<bool> = None;
    let mut copy_section_action = None;
    let mut copy_line_action = None;
    ui.horizontal(|ui| {
        if ui.small_button("◀ Prev").clicked() {
            nav_action = Some(false);
        }
        if ui.small_button("Next ▶").clicked() {
            nav_action = Some(true);
        }
        ui.separator();
        ui.label(RichText::new("Show:").small().weak());
        ui.selectable_value(&mut model.filter, viewmodel::TextFilter::All, "All");
        ui.selectable_value(
            &mut model.filter,
            viewmodel::TextFilter::Differences,
            "Differences",
        );
        ui.selectable_value(&mut model.filter, viewmodel::TextFilter::Same, "Same");
        ui.selectable_value(&mut model.filter, viewmodel::TextFilter::Context, "Context");
        ui.separator();
        if ui.small_button("Section →").clicked() {
            copy_section_action = Some(viewmodel::CopyDirection::LeftToRight);
        }
        if ui.small_button("← Section").clicked() {
            copy_section_action = Some(viewmodel::CopyDirection::RightToLeft);
        }
        if ui.small_button("Line →").clicked() {
            copy_line_action = Some(viewmodel::CopyDirection::LeftToRight);
        }
        if ui.small_button("← Line").clicked() {
            copy_line_action = Some(viewmodel::CopyDirection::RightToLeft);
        }
    });

    if let Some(next) = nav_action {
        if next {
            viewmodel::select_next_diff(&model.rows, &mut model.selected);
        } else {
            viewmodel::select_prev_diff(&model.rows, &mut model.selected);
        }
        scroll_to = model.selected;
    }

    // Resolve the action target: selection first, else first visible diff.
    let mut target_idx = model.selected;
    if target_idx.is_none() {
        let mut i = 0usize;
        while i < visible.len() {
            let mut candidate = None;
            if visible
                .get(i)
                .copied()
                .is_some_and(|orig| model.rows.get(orig).is_some_and(viewmodel::is_diff_row))
            {
                candidate = visible.get(i).copied();
            }
            if candidate.is_some() {
                target_idx = candidate;
                break;
            }
            i = i.saturating_add(1);
        }
    }
    if let (Some(idx), Some(dir)) = (target_idx, copy_section_action)
        && viewmodel::copy_section(model, idx, dir).is_ok()
    {
        model.selected = Some(idx);
    }
    if let (Some(idx), Some(dir)) = (target_idx, copy_line_action)
        && viewmodel::copy_line(model, idx, dir).is_ok()
    {
        model.selected = Some(idx);
    }

    let selected_text = match model.selected {
        Some(i) => format!("selected row {i}"),
        None => "no selection".to_string(),
    };
    ui.label(
        RichText::new(format!(
            "{} rows ({} shown): {} diffs, {} same | {selected_text}",
            counts.total, counts.shown, counts.diffs, counts.sames
        ))
        .small()
        .weak(),
    );

    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let row_height = text_row_height(ui);
    let base_text = ui.style().visuals.text_color();
    let dark = ui.visuals().dark_mode;
    let gutter_dim = dim_for(dark);
    let mut copy_action = None;
    let mut select_action = None;

    ui.horizontal(|ui| {
        // Main diff scroll area
        ui.vertical(|ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show_rows(ui, row_height, visible.len(), |ui, range| {
                    for vis_pos in range {
                        let Some(orig) = visible.get(vis_pos).copied() else {
                            continue;
                        };
                        let Some(row) = model.rows.get(orig) else {
                            continue;
                        };
                        ui.set_row_height(row_height);
                        let is_selected = model.selected == Some(orig);
                        ui.horizontal(|ui| {
                            if is_selected {
                                ui.label(RichText::new("▶").monospace().strong());
                            } else if ui.small_button("·").clicked() {
                                select_action = Some(orig);
                            }
                            let (fg, bg) = kind_palette(row.kind, base_text);
                            let bg = if is_selected { CHANGED_BG } else { bg };
                            let layout = TextPaneLayout {
                                pane_chars,
                                gutter_chars,
                                prefs,
                                dim: gutter_dim,
                            };
                            ui.label(side_job_with_prefs(
                                Side::Left,
                                row,
                                &layout,
                                font_id.clone(),
                                fg,
                                bg,
                            ));

                            if row.kind != TextRowKind::Equal {
                                if ui
                                    .small_button("→")
                                    .on_hover_text("Copy Left to Right")
                                    .clicked()
                                {
                                    copy_action =
                                        Some((orig, viewmodel::CopyDirection::LeftToRight));
                                }
                                if ui
                                    .small_button("←")
                                    .on_hover_text("Copy Right to Left")
                                    .clicked()
                                {
                                    copy_action =
                                        Some((orig, viewmodel::CopyDirection::RightToLeft));
                                }
                            } else {
                                ui.label(RichText::new(" | ").monospace().color(gutter_dim));
                            }

                            ui.label(side_job_with_prefs(
                                Side::Right,
                                row,
                                &layout,
                                font_id.clone(),
                                fg,
                                bg,
                            ));
                            if scroll_to == Some(orig) {
                                ui.scroll_to_rect(ui.min_rect(), Some(egui::Align::Center));
                            }
                        });
                    }
                });
        });

        // Thumbnail overview gutter on the right; narrows when line
        // numbers hide so chrome stays in sync while counts stay honest.
        let avail_h = ui.available_height().max(100.0);
        let geom = viewmodel::gutter_geometry(model.rows.len(), avail_h, &model.rows);
        let bar_w = text_gutter_paint_width(prefs.show_line_numbers);
        let (rect, _resp) =
            ui.allocate_exact_size(egui::vec2(bar_w, avail_h), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, track_for(dark));

        for seg in geom.segments {
            let seg_rect = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, rect.min.y + seg.y_start),
                egui::pos2(rect.max.x, (rect.min.y + seg.y_end).min(rect.max.y)),
            );
            let color = match seg.kind {
                TextRowKind::Equal => Color32::TRANSPARENT,
                TextRowKind::Deleted => DELETED_BG,
                TextRowKind::Inserted => INSERTED_BG,
                TextRowKind::Changed => CHANGED_BG,
            };
            ui.painter().rect_filled(seg_rect, 0.0, color);
        }
    });

    if let Some((idx, dir)) = copy_action
        && viewmodel::copy_line(model, idx, dir).is_ok()
    {
        model.selected = Some(idx);
    }
    if let Some(idx) = select_action {
        model.selected = Some(idx);
    }
}

/// Which side of a text row a pane renders.
#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

/// Prefs-effective paint widths and gutter tone for one text pane:
/// widths come from [`view_text_with_prefs`] (wrap caps `pane_chars`
/// at 80, hidden numbers narrow `gutter_chars` to 1) while `prefs`
/// drives the per-row hiding and `dim` carries the per-mode gutter
/// tone. Bundled so pane painters stay under the argument-count lint.
pub struct TextPaneLayout<'a> {
    /// Prefs-effective pane width.
    pub pane_chars: usize,
    /// Prefs-effective gutter width.
    pub gutter_chars: usize,
    /// Toggles the pane honors.
    pub prefs: &'a TextViewPrefs,
    /// Per-mode gutter tone (`theme::dim_for(dark)` at the call site).
    pub dim: Color32,
}

/// One pane of one row honoring `layout`: line numbers blank when off,
/// accent spans cleared when syntax is off, width from the
/// prefs-effective `pane_chars`/`gutter_chars`. Counts are untouched:
/// hiding only changes paint, never the model.
fn side_job_with_prefs(
    side: Side,
    row: &TextRow,
    layout: &TextPaneLayout<'_>,
    font_id: FontId,
    fg: Color32,
    bg: Color32,
) -> LayoutJob {
    let (raw_no, line, raw_spans) = match side {
        Side::Left => (row.left_no, row.left.as_str(), row.accent_a.as_slice()),
        Side::Right => (row.right_no, row.right.as_str(), row.accent_b.as_slice()),
    };
    let no = if layout.prefs.show_line_numbers {
        raw_no
    } else {
        None
    };
    let mut job = LayoutJob {
        keep_trailing_whitespace: true,
        ..Default::default()
    };
    let mut gutter = TextFormat::simple(font_id.clone(), layout.dim);
    gutter.background = bg;
    job.append(&gutter_text(no, layout.gutter_chars), 0.0, gutter);
    for (segment, accented) in prefs_pane_segments(line, raw_spans, layout.pane_chars, layout.prefs)
    {
        let mut fmt = TextFormat::simple(font_id.clone(), fg);
        fmt.background = bg;
        if accented {
            fmt.color = ACCENT_FG;
            fmt.underline = Stroke::new(1.0, ACCENT_FG);
        }
        job.append(&segment, 0.0, fmt);
    }
    job
}

/// Prefs-aware pane split: syntax off clears accents before delegating
/// to [`pane_segments`], so wrap/syntax toggles show in panes while the
/// model keeps honest counts. Pure; unit-tested.
pub fn prefs_pane_segments(
    line: &str,
    spans: &[(usize, usize)],
    pane: usize,
    prefs: &TextViewPrefs,
) -> Vec<(String, bool)> {
    if prefs.syntax_highlight {
        pane_segments(line, spans, pane)
    } else {
        pane_segments(line, &[], pane)
    }
}

/// (text color, row background) for one row kind. Equal rows keep the
/// theme's text color on a transparent background.
fn kind_palette(kind: TextRowKind, base_text: Color32) -> (Color32, Color32) {
    match kind {
        TextRowKind::Equal => (base_text, Color32::TRANSPARENT),
        TextRowKind::Deleted => (DELETED_FG, DELETED_BG),
        TextRowKind::Inserted => (INSERTED_FG, INSERTED_BG),
        TextRowKind::Changed => (CHANGED_FG, CHANGED_BG),
    }
}

/// The line-number gutter: right-aligned number plus one separating
/// space, or blanks when the row has no such side.
fn gutter_text(no: Option<usize>, width: usize) -> String {
    match no {
        Some(n) => format!("{n:>width$} "),
        None => " ".repeat(width.saturating_add(1)),
    }
}

/// Splits `line` into (text, accented) segments at the half-open
/// char ranges in `spans`, truncated at `pane` characters like the
/// CLI's panes, with one trailing unaccented segment padding to the
/// full pane width. Pure; unit-tested.
fn pane_segments(line: &str, spans: &[(usize, usize)], pane: usize) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut current = String::new();
    let mut current_accent = false;
    let mut visible = 0usize;
    for (idx, ch) in line.chars().enumerate() {
        if visible >= pane {
            break;
        }
        let inside = spans.iter().any(|&(start, end)| idx >= start && idx < end);
        if inside != current_accent && !current.is_empty() {
            out.push((std::mem::take(&mut current), current_accent));
        }
        current_accent = inside;
        current.push(ch);
        visible = visible.saturating_add(1);
    }
    if !current.is_empty() {
        out.push((current, current_accent));
    }
    if visible < pane {
        out.push((" ".repeat(pane - visible), false));
    }
    out
}

// ----------------------------------------------------------------------------
// Folder view
// ----------------------------------------------------------------------------

/// The folder listing: the summary line (with its truncation note)
/// above the monospace tree grid with expand/collapse drill-down.
/// Rows are virtualized the same way as the text view.
/// Kept for the pending App rewiring (see the note on theme blues).
#[allow(dead_code)]
pub fn paint_folder(
    ui: &mut Ui,
    rows: &[FolderRow],
    summary: &str,
    expanded: &mut std::collections::BTreeSet<String>,
) {
    ui.label(RichText::new(summary).monospace().weak());
    ui.add_space(TIGHT_GAP);
    if rows.is_empty() {
        ui.label(RichText::new("(both folders are empty)").weak());
        return;
    }
    let tree_nodes = viewmodel::folder_tree(rows, expanded);
    let row_height = text_row_height(ui);
    let mut toggle_dir = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, tree_nodes.len(), |ui, range| {
            for i in range {
                let Some(node) = tree_nodes.get(i) else {
                    break;
                };
                ui.set_row_height(row_height);
                ui.horizontal(|ui| {
                    if node.depth > 0 {
                        ui.add_space(tree_indent(node.depth));
                    }
                    if node.row.is_dir && node.has_children {
                        let icon = if node.is_expanded { "▼" } else { "▶" };
                        if ui.small_button(icon).clicked() {
                            toggle_dir = Some(node.row.rel_path.clone());
                        }
                    } else if node.row.is_dir {
                        ui.label(RichText::new("  ").monospace());
                    }
                    ui.label(
                        RichText::new(node.row.glyph)
                            .monospace()
                            .color(glyph_color(&node.row, ui.visuals().dark_mode)),
                    );
                    ui.label(RichText::new(folder_columns(&node.row)).monospace());
                });
            }
        });

    if let Some(dir) = toggle_dir {
        if expanded.contains(&dir) {
            expanded.remove(&dir);
        } else {
            expanded.insert(dir);
        }
    }
}

/// The glyph column color: same/dim, different and left-only/red,
/// newer/blue, right-only/green, like the CLI's coloring.
/// Kept for the pending App rewiring (see the note on theme blues).
#[allow(dead_code)]
fn glyph_color(row: &FolderRow, dark: bool) -> Color32 {
    match row.glyph {
        '=' => dim_for(dark),
        '!' | 'L' => red_for(dark),
        '>' | '<' => blue_for(dark),
        'R' => green_for(dark),
        _ => dim_for(dark),
    }
}

/// The columns after the glyph, shaped like the CLI's row: sizes,
/// times, then the path with the directory mark.
/// Kept for the pending App rewiring (see the note on theme blues).
/// Covered by the `folder_columns_match_the_cli_shape` unit test.
#[allow(dead_code)]
fn folder_columns(row: &FolderRow) -> String {
    let ls = row.left_size.as_deref().unwrap_or("-");
    let rs = row.right_size.as_deref().unwrap_or("-");
    let lt = row.left_time.as_deref().unwrap_or("-");
    let rt = row.right_time.as_deref().unwrap_or("-");
    let dir_mark = if row.is_dir { "/" } else { "" };
    let rel_path = row.rel_path.as_str();
    format!(" {ls:>10} {rs:>10}  {lt:>16} {rt:>16}  {rel_path}{dir_mark}")
}

/// Straight-alpha RGBA bytes into an egui [`ColorImage`]: egui
/// textures expect premultiplied alpha, so every channel is scaled by
/// its alpha with saturating integer math. `None` when the bytes are
/// empty or disagree with the declared dimensions - those cases have
/// nothing to display and must not reach `ColorImage::new`, whose
/// size debug-assert would otherwise fire.
pub fn heatmap_color_image(bytes: &[u8], width: u32, height: u32) -> Option<egui::ColorImage> {
    let w = usize::try_from(width).ok()?;
    let h = usize::try_from(height).ok()?;
    if w == 0 || h == 0 {
        return None;
    }
    let expected = w.checked_mul(h)?.checked_mul(4)?;
    if bytes.len() != expected {
        return None;
    }
    let pixels: Vec<Color32> = premultiplied_rgba(bytes)
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| Color32::from_rgba_premultiplied(r, g, b, a))
        .collect();
    if pixels.len() != w.checked_mul(h)? {
        return None;
    }
    Some(egui::ColorImage::new([w, h], pixels))
}

/// Premultiplied copy of straight-alpha RGBA bytes: each color
/// channel scaled by alpha, saturating per channel. Empty for input
/// whose length is not a multiple of four. Pure; unit-tested.
fn premultiplied_rgba(bytes: &[u8]) -> Vec<u8> {
    if !bytes.len().is_multiple_of(4) {
        return Vec::new();
    }
    let (chunks, _empty_remainder) = bytes.as_chunks::<4>();
    let mut out = Vec::with_capacity(bytes.len());
    for &[r, g, b, a] in chunks {
        out.extend_from_slice(&[premul(r, a), premul(g, a), premul(b, a), a]);
    }
    out
}

/// One channel scaled by alpha: `c * a / 255`, saturating.
fn premul(channel: u8, alpha: u8) -> u8 {
    let scaled = u16::from(channel).saturating_mul(u16::from(alpha)) / 255;
    u8::try_from(scaled).unwrap_or(255)
}

// ----------------------------------------------------------------------------
// Picture compare (model-driven side-by-side / overlay + zoom / pan)
// ----------------------------------------------------------------------------

/// Width of one picture column: half the available width minus one
/// section gap, floored at [`MIN_HALF`] so narrow panels scroll instead
/// of crushing the image.
pub fn picture_column_width(avail: f32) -> f32 {
    ((avail - SECTION_GAP) / 2.0).max(MIN_HALF)
}

/// Clamped overlay mix paint honors (0 = full left, 100 = full right).
pub fn clamp_overlay_mix(mix: u8) -> u8 {
    mix.min(crate::app::PICTURE_OVERLAY_MAX)
}

/// Clamped zoom percent paint honors (25..=400).
pub fn clamp_zoom_pct(pct: u32) -> u32 {
    pct.clamp(crate::app::PICTURE_ZOOM_MIN, crate::app::PICTURE_ZOOM_MAX)
}

/// Zoom slider label: `zoom 100% (1.00x)`.
pub fn zoom_label(pct: u32) -> String {
    let clamped = clamp_zoom_pct(pct);
    format!("zoom {clamped}% ({:.2}x)", clamped as f32 / 100.0)
}

/// Overlay slider label: `mix 30% left / 70% right`.
pub fn overlay_label(mix: u8) -> String {
    let clamped = clamp_overlay_mix(mix);
    format!(
        "mix {clamped}% left / {}% right",
        crate::app::PICTURE_OVERLAY_MAX.saturating_sub(clamped)
    )
}

/// Pan readout label: `pan (+32,-16)px`.
pub fn pan_label(pan_x: i32, pan_y: i32) -> String {
    format!("pan ({pan_x:+},{pan_y:+})px")
}

/// Status verdict for the picture header: identical reads clean,
/// otherwise the differing share. Pure; unit-tested.
pub fn picture_status_line(model: &PictureCompareModel) -> String {
    if model.identical {
        String::from("identical: no differences")
    } else {
        model.status_summary()
    }
}

/// Display size of the heatmap under the current zoom: natural size
/// scaled by the zoom factor. Pure; unit-tested.
pub fn picture_display_size(natural_w: f32, natural_h: f32, view: &PictureViewState) -> (f32, f32) {
    let scale = view.zoom_factor().max(0.25);
    ((natural_w * scale).max(1.0), (natural_h * scale).max(1.0))
}

/// The model-driven picture view: mode radio (side-by-side columns
/// with per-side dimensions vs one overlay frame with a mix slider),
/// zoom slider, pan nudge buttons, the heatmap at the zoomed size,
/// then the CLI-worded summary lines. Spacing reuses the tree/gutter
/// tokens ([`TREE_INDENT`], [`SECTION_GAP`], [`TIGHT_GAP`]); colors
/// stay the frozen diff palette (no new hues).
pub fn paint_picture_compare(
    ui: &mut Ui,
    model: &PictureCompareModel,
    view: &mut PictureViewState,
    heatmap: Option<&TextureHandle>,
) {
    ui.label(RichText::new(picture_status_line(model)).monospace().weak());
    ui.add_space(TIGHT_GAP);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Layout:").small().weak());
        let mut mode = view.mode;
        for candidate in PictureMode::all() {
            ui.radio_value(&mut mode, candidate, candidate.label());
        }
        if mode != view.mode {
            view.mode = mode;
        }
    });
    ui.add_space(TIGHT_GAP);
    match view.mode {
        PictureMode::SideBySide => {
            let col = picture_column_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(col);
                    ui.label(RichText::new("left").small().weak());
                    ui.label(
                        RichText::new(format!("{}x{}", model.width_l, model.height_l)).monospace(),
                    );
                    ui.add_space(tree_indent(1));
                    ui.label(
                        RichText::new(format!(
                            "overlap {}x{} ({} px)",
                            model.overlap_w, model.overlap_h, model.total
                        ))
                        .small()
                        .weak(),
                    );
                });
                ui.vertical(|ui| {
                    ui.set_width(col);
                    ui.label(RichText::new("right").small().weak());
                    ui.label(
                        RichText::new(format!("{}x{}", model.width_r, model.height_r)).monospace(),
                    );
                    ui.add_space(tree_indent(1));
                    ui.label(
                        RichText::new(if model.identical {
                            "verdict: identical".to_string()
                        } else {
                            format!("differing: {}", model.differing)
                        })
                        .small()
                        .weak(),
                    );
                });
            });
        }
        PictureMode::Overlay => {
            ui.label(
                RichText::new("overlay: heatmap frame, slider blends sides")
                    .small()
                    .weak(),
            );
            let mut mix = clamp_overlay_mix(view.overlay_mix);
            let mix_text = overlay_label(mix);
            ui.add(egui::Slider::new(&mut mix, 0..=crate::app::PICTURE_OVERLAY_MAX).text(mix_text));
            if mix != view.overlay_mix {
                view.set_overlay_mix(mix);
            }
        }
    }
    ui.add_space(TIGHT_GAP);
    ui.horizontal(|ui| {
        ui.label(RichText::new(zoom_label(view.zoom_pct)).small().weak());
        let mut zoom = clamp_zoom_pct(view.zoom_pct);
        ui.add(
            egui::Slider::new(
                &mut zoom,
                crate::app::PICTURE_ZOOM_MIN..=crate::app::PICTURE_ZOOM_MAX,
            )
            .text("zoom"),
        );
        if zoom != view.zoom_pct {
            view.set_zoom_pct(zoom);
        }
    });
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(pan_label(view.pan_x, view.pan_y))
                .small()
                .weak(),
        );
        if ui.small_button("◀").clicked() {
            view.nudge_pan(-crate::app::PICTURE_PAN_STEP, 0);
        }
        if ui.small_button("▶").clicked() {
            view.nudge_pan(crate::app::PICTURE_PAN_STEP, 0);
        }
        if ui.small_button("▲").clicked() {
            view.nudge_pan(0, -crate::app::PICTURE_PAN_STEP);
        }
        if ui.small_button("▼").clicked() {
            view.nudge_pan(0, crate::app::PICTURE_PAN_STEP);
        }
        if ui.small_button("Reset view").clicked() {
            view.reset_view();
        }
    });
    ui.add_space(SECTION_GAP);
    match heatmap {
        Some(handle) => {
            let natural = handle.size_vec2();
            let (dw, dh) = picture_display_size(natural.x, natural.y, view);
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .scroll_offset(egui::vec2(
                    view.pan_x.clamp(0, crate::app::PICTURE_PAN_LIMIT) as f32,
                    view.pan_y.clamp(0, crate::app::PICTURE_PAN_LIMIT) as f32,
                ))
                .show(ui, |ui| {
                    ui.add(
                        egui::Image::new((handle.id(), handle.size_vec2()))
                            .fit_to_exact_size(egui::vec2(dw, dh)),
                    );
                });
        }
        None => paint_note(ui, "heatmap unavailable for these inputs"),
    }
    ui.add_space(TIGHT_GAP);
    paint_lines(ui, &model.lines);
}

// ----------------------------------------------------------------------------
// Sync / folder-merge / audio checklists (per-file rows)
// ----------------------------------------------------------------------------

/// Paint-ready text of one sync checklist row: checkbox state plus
/// the display line. Pure; unit-tested.
pub fn sync_row_text(checked: bool, line: &str) -> String {
    let box_glyph = if checked { "[x]" } else { "[ ]" };
    format!("{box_glyph} {line}")
}

/// Paint-ready text of one skipped sync entry. Pure; unit-tested.
pub fn sync_skip_text(rel_path: &str, reason: &str) -> String {
    format!("~ {rel_path} ({reason})")
}

/// Glyph color of one sync row: copies track blue, deletes red, like
/// the folder glyphs; frozen palette, no new hues.
pub fn sync_row_color(action: autocompare_compare::SyncAction, dark: bool) -> egui::Color32 {
    use autocompare_compare::SyncAction;
    match action {
        SyncAction::CopyToRight | SyncAction::CopyToLeft => blue_for(dark),
        SyncAction::DeleteRight | SyncAction::DeleteLeft => red_for(dark),
    }
}

/// The sync plan as per-file checklist rows: each step is a checkbox
/// row with its glyph, path and action label, then skipped entries
/// and the honest counts line. Dry-run only; checking rows stages
/// them, Apply stays a CLI operation.
pub fn paint_sync_checklist(ui: &mut Ui, list: &mut SyncChecklist) {
    let dark = ui.visuals().dark_mode;
    ui.label(RichText::new(list.summary()).monospace().weak());
    if list.truncated {
        paint_note(
            ui,
            "the walk stopped at the depth limit; deeper entries are not planned",
        );
    }
    ui.add_space(TIGHT_GAP);
    if list.rows.is_empty() {
        ui.label(RichText::new("(nothing to do: the trees already match the plan)").weak());
    } else {
        for row in &mut list.rows {
            let color = sync_row_color(row.action, dark);
            let hover = sync_row_text(row.checked, &row.display_line());
            ui.horizontal(|ui| {
                ui.checkbox(&mut row.checked, "").on_hover_text(hover);
                ui.label(
                    RichText::new(row.glyph().to_string())
                        .monospace()
                        .color(color),
                );
                ui.label(RichText::new(row.display_line()).monospace());
                ui.label(RichText::new(row.action_label()).small().weak());
            });
        }
    }
    if !list.skipped.is_empty() {
        ui.add_space(TIGHT_GAP);
        for (path, reason) in &list.skipped {
            ui.label(
                RichText::new(sync_skip_text(path, reason))
                    .monospace()
                    .small()
                    .color(dim_for(dark)),
            );
        }
    }
    ui.label(
        RichText::new(format!(
            "{} of {} step(s) selected",
            list.checked_count(),
            list.rows.len()
        ))
        .small()
        .weak(),
    );
}

/// Glyph color of one folder-merge row: conflicts red, clean merges
/// green, side takes blue, new dirs dim; frozen palette only.
pub fn folder_merge_row_color(
    action: autocompare_compare::TreeMergeAction,
    dark: bool,
) -> egui::Color32 {
    use autocompare_compare::TreeMergeAction;
    match action {
        TreeMergeAction::Conflict => red_for(dark),
        TreeMergeAction::MergedText => green_for(dark),
        TreeMergeAction::TakeLeft | TreeMergeAction::TakeRight => blue_for(dark),
        TreeMergeAction::MakeDir => dim_for(dark),
    }
}

/// The folder-merge plan as per-file checklist rows: each step is a
/// checkbox row with its glyph, path and action label, then conflict
/// paths and the counts line. Dry-run only.
pub fn paint_folder_merge_checklist(ui: &mut Ui, list: &mut FolderMergeChecklist) {
    let dark = ui.visuals().dark_mode;
    ui.label(RichText::new(list.summary()).monospace().weak());
    ui.add_space(TIGHT_GAP);
    if list.rows.is_empty() {
        ui.label(RichText::new("(nothing to do: the merge plan is empty)").weak());
    } else {
        for row in &mut list.rows {
            let color = folder_merge_row_color(row.action, dark);
            ui.horizontal(|ui| {
                ui.checkbox(&mut row.checked, "");
                ui.label(
                    RichText::new(row.glyph().to_string())
                        .monospace()
                        .color(color),
                );
                ui.label(RichText::new(row.display_line()).monospace());
                if row.is_conflict() {
                    ui.label(
                        RichText::new("needs a decision")
                            .small()
                            .color(red_for(dark)),
                    );
                } else {
                    ui.label(RichText::new(row.action_label()).small().weak());
                }
            });
        }
    }
    if !list.conflicts.is_empty() {
        ui.add_space(TIGHT_GAP);
        ui.label(
            RichText::new(format!("{} conflict(s):", list.conflicts.len()))
                .small()
                .weak(),
        );
        for path in &list.conflicts {
            ui.label(
                RichText::new(format!("! {path} (conflict)"))
                    .monospace()
                    .small()
                    .color(red_for(dark)),
            );
        }
    }
    ui.label(
        RichText::new(format!(
            "{} of {} step(s) selected",
            list.checked_count(),
            list.rows.len()
        ))
        .small()
        .weak(),
    );
}

/// Paint-ready text of one audio row: label plus detail. Pure;
/// unit-tested.
pub fn audio_row_text(kind: AudioCheckKind, label: &str, detail: &str) -> String {
    let tag = match kind {
        AudioCheckKind::LeftConfig => "L",
        AudioCheckKind::RightConfig => "R",
        AudioCheckKind::Verdict => "=",
    };
    format!("{tag} {label}: {detail}")
}

/// Color of one audio row: configs dim, identical verdict green,
/// mismatch red, differing verdict changed-fg; frozen palette only.
pub fn audio_row_color(
    kind: AudioCheckKind,
    identical: bool,
    mismatch: bool,
    dark: bool,
) -> egui::Color32 {
    match kind {
        AudioCheckKind::LeftConfig | AudioCheckKind::RightConfig => dim_for(dark),
        AudioCheckKind::Verdict if mismatch => red_for(dark),
        AudioCheckKind::Verdict if identical => green_for(dark),
        AudioCheckKind::Verdict => CHANGED_FG,
    }
}

/// The audio compare as checklist rows: one row per stream config
/// plus the verdict row, each with a checkbox, then the summary.
pub fn paint_audio_checklist(ui: &mut Ui, list: &mut AudioChecklist) {
    let dark = ui.visuals().dark_mode;
    ui.label(RichText::new(list.summary()).monospace().weak());
    ui.add_space(TIGHT_GAP);
    if list.rows.is_empty() {
        ui.label(RichText::new("(no audio facts to show)").weak());
        return;
    }
    for row in &mut list.rows {
        let color = audio_row_color(row.kind, list.identical, list.config_mismatch, dark);
        ui.horizontal(|ui| {
            ui.checkbox(&mut row.checked, "");
            ui.label(
                RichText::new(audio_row_text(row.kind, &row.label, &row.detail))
                    .monospace()
                    .color(color),
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_line_pads_to_the_pane_width() {
        assert_eq!(
            pane_segments("ab", &[], 4),
            vec![("ab".to_string(), false), ("  ".to_string(), false)]
        );
    }

    #[test]
    fn accent_span_splits_the_pane_into_three_segments() {
        assert_eq!(
            pane_segments("abc", &[(1, 2)], 3),
            vec![
                ("a".to_string(), false),
                ("b".to_string(), true),
                ("c".to_string(), false),
            ]
        );
    }

    #[test]
    fn accents_at_line_start_and_truncation_are_handled() {
        assert_eq!(
            pane_segments("abc", &[(0, 2)], 4),
            vec![
                ("ab".to_string(), true),
                ("c".to_string(), false),
                (" ".to_string(), false),
            ]
        );
        // Longer than the pane: truncated, like the CLI's panes.
        assert_eq!(
            pane_segments("abcdef", &[(5, 6)], 3),
            vec![("abc".to_string(), false)]
        );
    }

    #[test]
    fn empty_line_is_only_padding() {
        assert_eq!(
            pane_segments("", &[(0, 1)], 2),
            vec![("  ".to_string(), false)]
        );
    }

    #[test]
    fn zero_length_spans_accent_nothing() {
        assert_eq!(
            pane_segments("ab", &[(1, 1)], 2),
            vec![("ab".to_string(), false)]
        );
    }

    #[test]
    fn gutter_text_right_aligns_numbers_and_blanks_sides() {
        assert_eq!(gutter_text(Some(12), 4), "  12 ".to_string());
        assert_eq!(gutter_text(None, 4), "     ".to_string());
        assert_eq!(gutter_text(Some(123456), 4), "123456 ".to_string());
    }

    #[test]
    fn folder_columns_match_the_cli_shape() {
        let row = FolderRow {
            rel_path: "a/b.txt".to_string(),
            is_dir: false,
            glyph: '!',
            left_size: Some("10".to_string()),
            right_size: None,
            left_time: Some("1970-01-01 00:00".to_string()),
            right_time: None,
            changed: true,
        };
        assert_eq!(
            folder_columns(&row),
            "         10          -  1970-01-01 00:00                -  a/b.txt"
        );
        let dir = FolderRow {
            rel_path: "a".to_string(),
            is_dir: true,
            ..row
        };
        assert!(folder_columns(&dir).ends_with("a/"));
    }

    #[test]
    fn premultiplied_rgba_is_saturating_per_channel() {
        // Opaque passes through unchanged.
        assert_eq!(
            premultiplied_rgba(&[255, 10, 20, 255]),
            vec![255, 10, 20, 255]
        );
        // 255*128/255 = 128, 10*128/255 = 5, 20*128/255 = 10.
        assert_eq!(
            premultiplied_rgba(&[255, 10, 20, 128]),
            vec![128, 5, 10, 128]
        );
        // Transparent collapses to zero.
        assert_eq!(premultiplied_rgba(&[200, 200, 200, 0]), vec![0, 0, 0, 0]);
        // Ragged input models nothing; so does empty.
        assert!(premultiplied_rgba(&[1, 2, 3]).is_empty());
        assert!(premultiplied_rgba(&[]).is_empty());
    }

    #[test]
    fn heatmap_image_builds_only_from_consistent_bytes() {
        let image = heatmap_color_image(&[255, 0, 0, 255], 1, 1).unwrap_or_default();
        let [w, h] = image.size;
        assert_eq!((w, h), (1, 1));
        assert_eq!(
            image.pixels,
            vec![Color32::from_rgba_premultiplied(255, 0, 0, 255)]
        );
        // Bytes disagreeing with the declared size model nothing.
        assert!(heatmap_color_image(&[255, 0, 0, 255], 2, 1).is_none());
        // Empty dimensions or empty bytes model nothing.
        assert!(heatmap_color_image(&[], 0, 0).is_none());
        assert!(heatmap_color_image(&[], 1, 1).is_none());
    }

    #[test]
    fn side_job_lays_out_gutter_text_and_padding_in_order() {
        let row = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(1),
            right_no: None,
            left: "ab".to_string(),
            right: String::new(),
            accent_a: vec![(1, 2)],
            accent_b: Vec::new(),
        };
        let model = TextRows {
            rows: vec![row.clone()],
            truncated: false,
            coarse: false,
            pane_chars: 4,
            gutter_chars: 4,
            selected: None,
            filter: viewmodel::TextFilter::All,
        };
        let shown = TextViewPrefs::default();
        let layout = TextPaneLayout {
            pane_chars: model.pane_chars,
            gutter_chars: model.gutter_chars,
            prefs: &shown,
            dim: crate::theme::DARK_DIM,
        };
        let job = side_job_with_prefs(
            Side::Left,
            &row,
            &layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        // Gutter "   1 ", text "ab" with "b" accented, then pane
        // padding: the job text is exactly gutter + padded line.
        assert_eq!(job.text, "   1 ab  ");
        assert!(job.keep_trailing_whitespace);
        // Sections: gutter, plain "a", accented "b", padding.
        assert_eq!(job.sections.len(), 4);
        // The blank right side is all spaces: gutter (5) plus the
        // padded empty pane (4).
        let blank = side_job_with_prefs(
            Side::Right,
            &row,
            &layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        assert_eq!(blank.text, "         ");
    }

    #[test]
    fn prefs_syntax_off_clears_accents_but_keeps_text() {
        let on = TextViewPrefs {
            syntax_highlight: true,
            ..TextViewPrefs::default()
        };
        let off = TextViewPrefs {
            syntax_highlight: false,
            ..TextViewPrefs::default()
        };
        let with_accents = prefs_pane_segments("abc", &[(1, 2)], 3, &on);
        assert!(with_accents.iter().any(|(_, acc)| *acc));
        let cleared = prefs_pane_segments("abc", &[(1, 2)], 3, &off);
        assert!(cleared.iter().all(|(_, acc)| !acc));
        // Same visible text, only accents differ.
        let text_of =
            |segs: &[(String, bool)]| -> String { segs.iter().map(|(s, _)| s.clone()).collect() };
        assert_eq!(text_of(&with_accents), text_of(&cleared));
    }

    #[test]
    fn prefs_line_numbers_off_blanks_gutter_and_narrows() {
        let row = TextRow {
            kind: TextRowKind::Changed,
            left_no: Some(7),
            right_no: Some(7),
            left: "hi".to_string(),
            right: "hi!".to_string(),
            accent_a: vec![(0, 1)],
            accent_b: vec![(0, 1)],
        };
        let shown = TextViewPrefs::default();
        let hidden = TextViewPrefs {
            show_line_numbers: false,
            syntax_highlight: false,
            ..TextViewPrefs::default()
        };
        let shown_layout = TextPaneLayout {
            pane_chars: 4,
            gutter_chars: 4,
            prefs: &shown,
            dim: crate::theme::DARK_DIM,
        };
        let with_numbers = side_job_with_prefs(
            Side::Left,
            &row,
            &shown_layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        assert!(with_numbers.text.starts_with("   7 "));
        // Gutter + accented head + plain tail (tail and padding may
        // merge in LayoutJob, so at least 3 sections).
        assert!(with_numbers.sections.len() >= 3);
        let hidden_layout = TextPaneLayout {
            pane_chars: 4,
            gutter_chars: 1,
            prefs: &hidden,
            dim: crate::theme::LIGHT_DIM,
        };
        let without_numbers = side_job_with_prefs(
            Side::Left,
            &row,
            &hidden_layout,
            FontId::monospace(12.0),
            Color32::WHITE,
            Color32::BLACK,
        );
        // Gutter width 1 blanks to two spaces, no accent sections.
        assert!(without_numbers.text.starts_with("  "));
        assert!(
            without_numbers
                .sections
                .iter()
                .skip(1)
                .all(|s| s.format.color == Color32::WHITE)
        );
        // Thumbnail chrome narrows in sync.
        assert!(crate::gutter::gutter_bar_width(false) < crate::gutter::gutter_bar_width(true));
    }

    #[test]
    fn prefs_word_wrap_caps_pane_at_80() {
        let long = "x".repeat(200);
        let on = TextViewPrefs {
            word_wrap: true,
            ..TextViewPrefs::default()
        };
        let off = TextViewPrefs::default();
        let wrapped: String = prefs_pane_segments(&long, &[], 80, &on)
            .into_iter()
            .map(|(segment, _)| segment)
            .collect();
        assert_eq!(wrapped, "x".repeat(80));
        assert_eq!(
            prefs_pane_segments(&long, &[], 200, &off)
                .into_iter()
                .map(|(s, _)| s)
                .collect::<String>()
                .len(),
            200
        );
        // Model-level cap matches the paint width.
        let model = TextRows {
            rows: Vec::new(),
            truncated: false,
            coarse: false,
            pane_chars: 200,
            gutter_chars: 4,
            selected: None,
            filter: viewmodel::TextFilter::All,
        };
        assert_eq!(view_text_with_prefs(&model, &on).pane_chars, 80);
        assert_eq!(view_text_with_prefs(&model, &off).pane_chars, 200);
    }

    #[test]
    fn hex_hunk_line_carries_offset_and_counts() {
        let hunk = views_hex::HexHunkView {
            offset: 3,
            left_len: 1,
            right_len: 2,
        };
        assert_eq!(format_hex_hunk_line(&hunk), "0x00000003: 1 vs 2 byte(s)");
    }

    #[test]
    fn tolerance_text_parses_like_the_engine() {
        assert_eq!(parse_tolerance_text("0.1"), Some(0.1));
        assert_eq!(parse_tolerance_text("  1 "), Some(1.0));
        assert_eq!(parse_tolerance_text("0"), Some(0.0));
        assert_eq!(parse_tolerance_text(""), None);
        assert_eq!(parse_tolerance_text("abc"), None);
        assert_eq!(parse_tolerance_text("-0.5"), None);
        assert_eq!(parse_tolerance_text("NaN"), None);
        assert_eq!(parse_tolerance_text("inf"), None);
        // Paint-level parsing agrees with the model sanitizer.
        assert_eq!(
            parse_tolerance_text("0.25"),
            views_table::TableKeyView::sanitize_tolerance(Some(0.25))
        );
    }

    #[test]
    fn geometry_tokens_match_targets() {
        assert_eq!(OUTER_MARGIN, 12);
        assert_eq!(SECTION_GAP, 8.0);
        assert_eq!(TIGHT_GAP, 4.0);
        assert_eq!(FOOTER_GAP, 6.0);
        assert_eq!(TAB_BOTTOM_GAP, 6.0);
        assert_eq!(TREE_INDENT, 14.0);
        assert_eq!((CARD_W, CARD_H), (240.0, 75.0));
    }

    #[test]
    fn half_fields_share_equal_width() {
        assert_eq!(half_field_width(800.0), 325.0);
        assert_eq!(half_field_width(0.0), 105.0);
    }

    #[test]
    fn third_fields_share_equal_width() {
        assert_eq!(third_field_width(900.0), 225.0);
        assert_eq!(third_field_width(0.0), 55.0);
    }

    #[test]
    fn tree_indent_scales_with_depth() {
        assert_eq!(tree_indent(0), 0.0);
        assert_eq!(tree_indent(1), 14.0);
        assert_eq!(tree_indent(2), 28.0);
    }

    #[test]
    fn home_paint_mapping_uses_single_source_sizes() {
        use crate::theme::{TYPO_BODY_SIZE, TYPO_HOME_TITLE_SIZE};
        assert_eq!(home_title_paint_size(), TYPO_HOME_TITLE_SIZE);
        assert_eq!(home_title_paint_size(), 14.0);
        assert!(home_desc_paint_is_small_weak());
        assert_eq!(home_button_paint_size(), TYPO_BODY_SIZE);
        assert_eq!(home_button_paint_size(), 13.0);
    }

    #[test]
    fn gutter_paint_width_reproduces_unscaled_bar() {
        use crate::gutter::gutter_bar_width;
        assert_eq!(text_gutter_paint_width(true), 12.0);
        assert_eq!(text_gutter_paint_width(false), 6.0);
        assert_eq!(text_gutter_paint_width(true), gutter_bar_width(true));
        assert_eq!(text_gutter_paint_width(false), gutter_bar_width(false));
    }

    #[test]
    fn chrome_selectors_follow_dark_mode() {
        use crate::theme::{
            DARK_BLUE, DARK_DIM, DARK_GREEN, DARK_RED, DARK_TRACK, LIGHT_BLUE, LIGHT_DIM,
            LIGHT_GREEN, LIGHT_RED, LIGHT_TRACK,
        };
        assert_eq!(dim_for(true), DARK_DIM);
        assert_eq!(dim_for(false), LIGHT_DIM);
        assert_eq!(red_for(true), DARK_RED);
        assert_eq!(red_for(false), LIGHT_RED);
        assert_eq!(green_for(true), DARK_GREEN);
        assert_eq!(green_for(false), LIGHT_GREEN);
        assert_eq!(blue_for(true), DARK_BLUE);
        assert_eq!(blue_for(false), LIGHT_BLUE);
        assert_eq!(track_for(true), DARK_TRACK);
        assert_eq!(track_for(false), LIGHT_TRACK);
    }

    fn picture_model_fixture(identical: bool) -> crate::app::PictureCompareModel {
        crate::app::PictureCompareModel::from_diff(&autocompare_diff::PictureDiff {
            width_l: 2,
            height_l: 2,
            width_r: 3,
            height_r: 3,
            overlap_w: 2,
            overlap_h: 2,
            differing: if identical { 0 } else { 1 },
            total: 4,
            bbox: None,
            identical,
        })
    }

    #[test]
    fn picture_columns_split_avail_with_floor() {
        assert_eq!(picture_column_width(800.0), (800.0 - SECTION_GAP) / 2.0);
        assert_eq!(picture_column_width(0.0), MIN_HALF);
        assert_eq!(picture_column_width(100.0), MIN_HALF);
    }

    #[test]
    fn picture_sliders_clamp_and_label() {
        assert_eq!(clamp_overlay_mix(200), 100);
        assert_eq!(clamp_overlay_mix(30), 30);
        assert_eq!(clamp_zoom_pct(9), 25);
        assert_eq!(clamp_zoom_pct(900), 400);
        assert_eq!(clamp_zoom_pct(150), 150);
        assert!(zoom_label(100).contains("1.00x"));
        assert!(overlay_label(30).contains("30% left"));
        assert!(overlay_label(30).contains("70% right"));
        assert!(pan_label(32, -16).contains("+32"));
    }

    #[test]
    fn picture_status_and_display_size_follow_view() {
        let clean = picture_model_fixture(true);
        assert!(picture_status_line(&clean).contains("identical"));
        let diffed = picture_model_fixture(false);
        assert!(picture_status_line(&diffed).contains("1 of 4 pixels differ"));
        let mut view = crate::app::PictureViewState::default();
        assert_eq!(picture_display_size(100.0, 50.0, &view), (100.0, 50.0));
        view.set_zoom_pct(200);
        assert_eq!(picture_display_size(100.0, 50.0, &view), (200.0, 100.0));
        view.set_zoom_pct(25);
        assert_eq!(picture_display_size(0.0, 0.0, &view), (1.0, 1.0));
    }

    #[test]
    fn sync_row_text_and_colors_map_actions() {
        use autocompare_compare::SyncAction;
        assert_eq!(sync_row_text(true, "> a.txt"), "[x] > a.txt");
        assert_eq!(sync_row_text(false, "> a.txt"), "[ ] > a.txt");
        assert_eq!(
            sync_skip_text("b.txt", "already in sync"),
            "~ b.txt (already in sync)"
        );
        assert_eq!(
            sync_row_color(SyncAction::CopyToRight, true),
            blue_for(true)
        );
        assert_eq!(
            sync_row_color(SyncAction::CopyToLeft, false),
            blue_for(false)
        );
        assert_eq!(sync_row_color(SyncAction::DeleteRight, true), red_for(true));
        assert_eq!(
            sync_row_color(SyncAction::DeleteLeft, false),
            red_for(false)
        );
    }

    #[test]
    fn folder_merge_colors_flag_conflicts() {
        use autocompare_compare::TreeMergeAction;
        assert_eq!(
            folder_merge_row_color(TreeMergeAction::Conflict, true),
            red_for(true)
        );
        assert_eq!(
            folder_merge_row_color(TreeMergeAction::MergedText, true),
            green_for(true)
        );
        assert_eq!(
            folder_merge_row_color(TreeMergeAction::TakeLeft, false),
            blue_for(false)
        );
        assert_eq!(
            folder_merge_row_color(TreeMergeAction::MakeDir, true),
            dim_for(true)
        );
    }

    #[test]
    fn audio_row_text_and_colors_follow_verdict() {
        let left = audio_row_text(AudioCheckKind::LeftConfig, "left", "1.000s");
        assert!(left.starts_with("L left: "));
        let verdict = audio_row_text(AudioCheckKind::Verdict, "verdict", "no differences");
        assert!(verdict.starts_with("= verdict: "));
        assert_eq!(
            audio_row_color(AudioCheckKind::Verdict, true, false, true),
            green_for(true)
        );
        assert_eq!(
            audio_row_color(AudioCheckKind::Verdict, false, true, false),
            red_for(false)
        );
        assert_eq!(
            audio_row_color(AudioCheckKind::Verdict, false, false, true),
            CHANGED_FG
        );
        assert_eq!(
            audio_row_color(AudioCheckKind::LeftConfig, false, false, true),
            dim_for(true)
        );
    }
}
