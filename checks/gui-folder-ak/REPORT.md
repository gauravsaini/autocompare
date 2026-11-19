# REPORT — Wave AK-FOLDER (Gap #6 folder cols paint)

OWNS: `crates/gui/src/tree.rs` + `checks/gui-folder-ak/**`
KEVL untouched: views.rs, app.rs, viewmodel.rs, views_table.rs, views_hex.rs, home.rs, gutter.rs, theme.rs, screenshot.rs.

Clean-room note: Beyond Compare ko sirf bahar se dekha (folder compare me naam flex, size/time fixed columns ek baseline par). Koi code/API copy nahi — sab apne shabdon me headless spec.

## Kya kiya (geometry first, typography second, colors frozen)

`tree.rs` me Gap #6 polish section joda (headless, no egui, no I/O, no hues):

Geometry (spacing scale 4/8/12/16/24 par, indent 12.0 frozen):
- `FOLDER_ROW_HEIGHT` 24.0, `FOLDER_HEADER_HEIGHT` 24.0, `FOLDER_CELL_PAD_X` 8.0, `FOLDER_COL_GAP` 8.0, `FOLDER_MONO_ADVANCE` 7.0, `FOLDER_PATH_MIN_CHARS` 24.
- `folder_row_height()`, `folder_header_height()`, `folder_cell_pad_x()`, `folder_col_gap()`.
- `folder_column_align()`: Path Left, sizes Right (byte counts ka ikai column seedhe), times Left (fixed-width mono dates left se padho). Header same align.
- `folder_column_min_px()`: fixed `chars * advance + 2*pad` (size 86px, time 128px), Path flex-min 184px. Time > size, path sabse bada.
- Char widths frozen: `FOLDER_SIZE_CHARS` 10, `FOLDER_TIME_CHARS` 16 (format_time 16 chars me fit).

Typography (mono/size/weight, TypoRole precedent naam se, bina theme import):
- `FolderTypo { mono, size, strong }`, `folder_typo_for_column()`, `folder_header_typo()`.
- Path: mono body-regular 13.0 (Body precedent). Sides: mono caption-regular 12.0 (Caption precedent). Header: mono caption-strong 12.0.
- Missing sides `-` dash (pehle se), headless `pad_folder_cell` left-pad stable — right-align sirf paint ka numeric polish.

## Gates evidence

- G1 geometry: `cargo test -p autocompare-gui folder_col_geometry` → ok (`folder_col_geometry_tokens_stay_on_scale` pass).
- G2 typo: `cargo test -p autocompare-gui folder_col_typo` → ok (`folder_col_typo_mono_sizes_with_header_strong` pass).
- G3 colors frozen: `grep -rn "Color32\|Visuals" crates/gui/src/tree.rs` → no output (CLEAN).
- G4 tests/fmt/banned: `cargo test -p autocompare-gui` → 378 passed, 0 failed. `cargo fmt --check -- crates/gui/src/tree.rs` → ok (sirf OWNS file par `cargo fmt --` chalaya). Banned macros (`unwrap()/expect(/panic!/todo!/unimplemented!/unreachable!`) → CLEAN; `unsafe` sirf `forbid` line.
- G5 harness: `tools/gui-polish/run-geo-typo.sh checks/gui-folder-ak/round-1` → exit 0, `RESULT: PASS` (PNG 9624 bytes, magic ok, distinct 256).

## Clippy note (honest, blocker nahi chhupaya)

- `cargo clippy -p autocompare-gui --all-targets -- -D warnings` crate-wide fail hota hai — 6+6 pre-existing errors dusri files me (`views.rs` me `egui::Id::new` deprecated x4, assign-op x2). Ye files KEVL hain, chhui nahi.
- Apni file clean hai: clippy output me `tree.rs` ki zero errors/warnings (grep `tree.rs` → khali). Apna 1 naya `assertions_on_constants` warning aaya tha, fix kar diya (const compare → function-output compare).

## Proof files

- `checks/gui-folder-ak/GATES.md` (5 gates)
- `checks/gui-folder-ak/round-1/after.png` (9624 bytes valid PNG)
- `checks/gui-folder-ak/round-1/geo-typo-report.md` (RESULT: PASS)
- `checks/gui-folder-ak/round-1/tests.out`, `screenshot.out`
