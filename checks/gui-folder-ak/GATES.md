# GATES — Wave AK-FOLDER (Gap #6 folder cols paint)

OWNS: `crates/gui/src/tree.rs` + `checks/gui-folder-ak/**`
KEVL (no touch, read bhi avoid; viewmodel read-only ok): views.rs, app.rs, viewmodel.rs, views_table.rs, views_hex.rs, home.rs, gutter.rs, theme.rs, screenshot.rs.
Clean-room: Beyond Compare bahar se observe only, apne shabdon me. Colors frozen — is wave me koi hue change nahi.

Scope: folder compare columns ka paint spec — geometry first (widths, col alignment, row heights, padding) + typography second (mono/size/weight, TypoRole precedent). Paint wiring dusre agent ke paas hai; ye wave sirf `tree.rs` me headless spec + unit tests deta hai. Koi egui import nahi, koi I/O nahi.

- [ ] G1 — geometry spec headless: `cargo test -p autocompare-gui folder_col_geometry`
  CHECK: `cargo test -p autocompare-gui folder_col_geometry -- --nocapture`
  EXPECT: ok (row height 24, header 24, pad-x 8, gap 8, Path flex Left, sizes Right, times Right, size min-px > time min-px nahi — time wider; indent 12 frozen)
- [ ] G2 — typography spec headless: `cargo test -p autocompare-gui folder_col_typo`
  CHECK: `cargo test -p autocompare-gui folder_col_typo -- --nocapture`
  EXPECT: ok (Path mono body-regular, sides mono caption-regular, header mono caption-strong; sizes ghate hue me `-` dash, colors zero)
- [ ] G3 — colors frozen: `grep -rn "Color32\|Visuals::\|from_rgb" crates/gui/src/tree.rs` khali
  CHECK: `grep -rn "Color32\|Visuals" crates/gui/src/tree.rs || true`
  EXPECT: no output
- [ ] G4 — full gui tests green + clippy zero warnings + fmt clean (sirf OWNS files par `cargo fmt -- <files>`)
  CHECK: `cargo test -p autocompare-gui` + `cargo clippy -p autocompare-gui --all-targets -- -D warnings` + `cargo fmt --check -- crates/gui/src/tree.rs`
  EXPECT: all ok, 0 warnings
- [ ] G5 — headless proof: `tools/gui-polish/run-geo-typo.sh checks/gui-folder-ak/round-1`
  CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-folder-ak/round-1`
  EXPECT: `RESULT: PASS` in `round-1/geo-typo-report.md`, `after.png` valid PNG >= 8192 bytes
